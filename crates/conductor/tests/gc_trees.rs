//! `story:gc-finished-trees`: `conductor gc` and every reclaim run reclaim the managed trees whose
//! work is finished, through `worktree finish --discard-cache --archive` and `worktree gc --apply`,
//! and give back the `target/` of the stale ones, which they keep.
//!
//! Each case is a fixture home of its own, the built-in instance's: a bare origin
//! `<case>/origin/alpha.git` whose default branch is `main`, its clone `example-org/alpha` (the
//! checkouts root), and linked git worktrees of that clone under the built-in trees directory,
//! one for each kind of tree the story names:
//!
//! | tree | what it is | verdict |
//! |---|---|---|
//! | `merged` | its branch pushed and merged into `main` | finished |
//! | `deleted` | merged into `main`, its branch never on origin | finished |
//! | `build-only` | merged; untracked `target/` and `.check/` only | finished |
//! | `refuses` | merged; `target/` holds a nested `.git`, which `worktree finish` refuses until `target/` is gone | finished, after one retry |
//! | `done` | `finished` in the registry already | finished: only `worktree gc` |
//! | `unmerged-old` | a commit `main` lacks, untouched for 72 h | stale |
//! | `dirty-old` | merged, a changed tracked file, untouched for 72 h | stale |
//! | `remote-moved` | merged, but its branch on origin moved on | kept: unmerged |
//! | `dirty` | merged, a changed tracked file, untouched for an hour | kept: dirty |
//! | `leased` | merged; `worktree` counts a live lease on it | kept |
//! | `busy` | merged; a process works in it | kept |
//! | `session` | merged; a live session works in it | kept |
//! | `fresh` | merged, touched a moment ago | kept |
//!
//! Every tree has cargo build output under `target/`. The binary runs with `HOME` naming the
//! fixture home and a `PATH` led by fake `claude` and `worktree` programs: `claude agents --json`
//! prints the session list; `worktree inspect` prints the registry fixture, `worktree finish`
//! marks a tree finished (and refuses `refuses` while its `target/` is there), and `worktree gc
//! --apply --id` removes a finished tree, as the real ones would. Every call is logged. Nothing
//! outside the case's directory is read or written.

mod active;
mod common;

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use conductor_cli::gc::reclaim::{READINGS_VARIABLE, VAR_TMP_VARIABLE};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::host::ReclaimKind;
use conductor_model::host::obligations::ReclaimRunsQuery as _;
use serde_json::{Value, json};

const KIB: u64 = 1 << 10;
const GIB: u64 = 1 << 30;
const HOUR: Duration = Duration::from_secs(3600);

/// The trees `conductor gc` finds finished.
const FINISHED: [&str; 5] = ["merged", "deleted", "build-only", "refuses", "done"];
/// The trees it finds stale.
const STALE: [&str; 2] = ["unmerged-old", "dirty-old"];
/// The trees it keeps whole, `target/` included.
const KEPT: [&str; 6] = [
    "remote-moved",
    "dirty",
    "leased",
    "busy",
    "session",
    "fresh",
];

struct Case {
    dir: PathBuf,
    /// The process that works in `busy`: this case's own child, stopped by its handle.
    busy: Option<Child>,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read. The
/// process working in `busy` is always stopped.
impl Drop for Case {
    fn drop(&mut self) {
        if let Some(mut child) = self.busy.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

/// A file of `bytes` non-zero bytes, so it takes that much of the disk.
fn fill(path: &Path, bytes: u64) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(
        path,
        vec![0xAB_u8; usize::try_from(bytes).expect("a small file")],
    )
    .expect("write a case file");
}

fn program(bin: &Path, name: &str, script: &str) {
    let path = bin.join(name);
    write(&path, script);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// Runs git in `dir` with no user or system configuration, and answers its standard output.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=example",
            "-c",
            "user.email=example@example.invalid",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Sets the modification time of `path` and of everything under it to `ago` ago, the entries
/// before the directories that hold them; a link is left alone.
fn age_by(path: &Path, ago: Duration) {
    let then = SystemTime::now() - ago;
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("list a case directory") {
            age_by(&entry.expect("an entry").path(), ago);
        }
    }
    File::open(path)
        .and_then(|file| file.set_modified(then))
        .expect("set a modification time");
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("gc_trees")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let mut case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
            busy: None,
        };
        case.fixture();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn calls(&self) -> PathBuf {
        self.dir.join("calls.log")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn checkout(&self, name: &str) -> PathBuf {
        self.home().join("example-org").join(name)
    }

    /// The directory the binary runs in: no checkout, so no tracked repository.
    fn records(&self) -> PathBuf {
        self.checkout("conductor")
    }

    fn tree(&self, name: &str) -> PathBuf {
        self.home()
            .join(".local/state/worktree/trees/example-org/alpha")
            .join(name)
    }

    /// A linked worktree `name` of `alpha` on a new branch `name` from `main`, with one commit of
    /// its own and cargo build output under `target/`.
    fn branch(&self, name: &str) -> PathBuf {
        let alpha = self.checkout("alpha");
        let tree = self.tree(name);
        let shown = tree.display().to_string();
        git(
            &alpha,
            &["worktree", "add", "-q", "-b", name, &shown, "main"],
        );
        write(&tree.join(format!("{name}.txt")), name);
        git(&tree, &["add", "."]);
        git(&tree, &["commit", "-q", "-m", name]);
        write(
            &tree.join("target/CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55\n",
        );
        fill(&tree.join("target/debug/deps/out.bin"), 64 * KIB);
        tree
    }

    /// Merges the branch `name` into `main` with a merge commit, and pushes `main`.
    fn merge(&self, name: &str) {
        let alpha = self.checkout("alpha");
        git(&alpha, &["merge", "-q", "--no-ff", "-m", name, name]);
        git(&alpha, &["push", "-q", "origin", "main"]);
    }

    /// Ages everything the tree `tree` shows of its activity: its files and its git `HEAD` and
    /// `index`.
    fn age(&self, tree: &Path, ago: Duration) {
        let gitdir = PathBuf::from(git(tree, &["rev-parse", "--absolute-git-dir"]));
        age_by(tree, ago);
        for file in ["HEAD", "index"] {
            age_by(&gitdir.join(file), ago);
        }
    }

    fn fixture(&mut self) {
        let origin = self.dir.join("origin/alpha.git");
        fs::create_dir_all(&origin).expect("create the origin");
        git(&origin, &["init", "-q", "--bare", "-b", "main"]);
        let alpha = self.checkout("alpha");
        fs::create_dir_all(alpha.parent().expect("the checkouts root"))
            .expect("create the checkouts root");
        git(
            &self.home().join("example-org"),
            &["clone", "-q", &origin.display().to_string(), "alpha"],
        );
        write(&alpha.join("README.md"), "alpha\n");
        write(&alpha.join("Cargo.toml"), "[package]\nname = \"alpha\"\n");
        git(&alpha, &["add", "."]);
        git(&alpha, &["commit", "-q", "-m", "initial"]);
        git(&alpha, &["push", "-q", "-u", "origin", "main"]);
        git(&alpha, &["remote", "set-head", "origin", "main"]);
        fs::create_dir_all(self.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(self.records().join("decisions")).expect("create decisions/");

        // Merged, pushed or not.
        for name in [
            "merged",
            "build-only",
            "refuses",
            "done",
            "dirty",
            "dirty-old",
            "leased",
            "busy",
            "session",
            "fresh",
            "remote-moved",
        ] {
            let tree = self.branch(name);
            git(&tree, &["push", "-q", "-u", "origin", name]);
            self.merge(name);
        }
        let deleted = self.branch("deleted");
        drop(deleted);
        self.merge("deleted");
        // Its branch on origin moves on after the merge; the tree stays at the merged commit.
        let moved = self.tree("remote-moved");
        write(&moved.join("later.txt"), "later");
        git(&moved, &["add", "later.txt"]);
        git(&moved, &["commit", "-q", "-m", "later"]);
        git(&moved, &["push", "-q", "origin", "remote-moved"]);
        git(&moved, &["reset", "-q", "--hard", "HEAD~1"]);
        // A commit `main` lacks.
        self.branch("unmerged-old");
        // Untracked build output and gate logs only.
        fill(&self.tree("build-only").join(".check/gate.log"), 4 * KIB);
        // A nested repository under `target/`, which `worktree` cannot archive.
        write(
            &self
                .tree("refuses")
                .join("target/debug/build/dep/.git/HEAD"),
            "ref: refs/heads/main\n",
        );
        // Changed tracked files.
        for name in ["dirty", "dirty-old"] {
            write(&self.tree(name).join(format!("{name}.txt")), "changed\n");
        }

        let mut registry = Vec::new();
        for name in FINISHED.iter().chain(&STALE).chain(&KEPT) {
            let tree = self.tree(name);
            let ago = match *name {
                "fresh" => Duration::ZERO,
                "unmerged-old" | "dirty-old" => 72 * HOUR,
                _ => HOUR,
            };
            if !ago.is_zero() {
                self.age(&tree, ago);
            }
            write(
                &self.dir.join("paths").join(name),
                &tree.display().to_string(),
            );
            let lifecycle = if *name == "done" {
                write(&self.dir.join("finished").join(name), "");
                "finished"
            } else {
                "active"
            };
            if *name == "refuses" {
                write(&self.dir.join("refuse").join(name), "");
            }
            registry.push(json!({
                "record": {"id": name, "repository_root": alpha, "path": tree,
                           "purpose": "p", "owner": "agent", "lifecycle": lifecycle,
                           "created_at": 0, "last_seen_at": now_secs() - ago.as_secs(),
                           "finished_at": null, "head": null},
                "observed_at": now_secs(),
                "live_leases": u64::from(*name == "leased"),
            }));
        }
        // A tree the registry removed long ago: not looked at.
        registry.push(json!({
            "record": {"id": "gone", "repository_root": alpha, "path": self.tree("gone"),
                       "purpose": "p", "owner": "agent", "lifecycle": "removed",
                       "created_at": 0, "last_seen_at": 0, "finished_at": 0, "head": null},
            "observed_at": now_secs(), "live_leases": 0,
        }));
        write(
            &self.dir.join("inspect.json"),
            &json!({"version": 4, "ok": true, "format": "worktree.inspection/2",
                    "repository_root": alpha, "workspace": true, "inspections": registry})
            .to_string(),
        );
        write(
            &self.dir.join("agents.json"),
            &json!([{"id": "s-session", "name": "session",
                     "cwd": self.tree("session").join("src"), "pid": 104, "status": "busy"}])
            .to_string(),
        );

        let bin = self.bin();
        program(
            &bin,
            "claude",
            &format!(
                "#!/bin/sh\ncat '{}'\n",
                self.dir.join("agents.json").display()
            ),
        );
        program(
            &bin,
            "worktree",
            &format!(
                "#!/bin/sh\n\
                 D='{dir}'\n\
                 printf 'worktree %s\\n' \"$*\" >> \"$D/calls.log\"\n\
                 for a; do last=$a; done\n\
                 id=; prev=; for a; do [ \"$prev\" = --id ] && id=$a; prev=$a; done\n\
                 case \"$1\" in\n\
                 inspect) cat \"$D/inspect.json\" ;;\n\
                 sweep) printf '{{\"version\":4,\"ok\":true,\"items\":[]}}\\n' ;;\n\
                 gc)\n\
                 \x20 case \" $* \" in\n\
                 \x20 *' --apply '*)\n\
                 \x20   path=$(cat \"$D/paths/$id\")\n\
                 \x20   if [ -f \"$D/finished/$id\" ]; then rm -rf \"$path\"; fi\n\
                 \x20   printf '{{\"version\":4,\"ok\":true,\"assessments\":[]}}\\n' ;;\n\
                 \x20 *) printf '{{\"version\":4,\"ok\":true,\"assessments\":[]}}\\n' ;;\n\
                 \x20 esac ;;\n\
                 finish)\n\
                 \x20 path=$(cat \"$D/paths/$last\")\n\
                 \x20 if [ -f \"$D/refuse/$last\" ] && [ -e \"$path/target\" ]; then\n\
                 \x20   printf '{{\"code\":\"archive-unsupported-entry\",\"message\":\"a nested repository\",\"ok\":false,\"version\":4}}\\n'\n\
                 \x20   exit 1\n\
                 \x20 fi\n\
                 \x20 : > \"$D/finished/$last\"\n\
                 \x20 printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 discard-cache)\n\
                 \x20 rm -rf \"$last/target\"\n\
                 \x20 printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 *) exit 2 ;;\n\
                 esac\n",
                dir = self.dir.display(),
            ),
        );

        // A process of this case's own works in `busy`.
        let child = Command::new("sleep")
            .arg("600")
            .current_dir(self.tree("busy"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start a process in busy");
        self.busy = Some(child);
    }

    fn path(&self) -> String {
        format!("{}:/usr/bin:/bin", self.bin().display())
    }

    /// `conductor --state-dir <case>/state <args>` from the records directory.
    fn conductor(&self, args: &[&str]) -> Output {
        common::conductor(self.home())
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .current_dir(self.records())
            .env("PATH", self.path())
            .env(READINGS_VARIABLE, self.dir.join("free.txt"))
            .env(VAR_TMP_VARIABLE, self.dir.join("var-tmp"))
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// Writes the free-disk readings, in GiB, the binary takes in turn, the last one standing.
    fn readings(&self, gib: &[u64]) {
        let text: String = gib.iter().map(|gib| format!("{}\n", gib * GIB)).collect();
        write(&self.dir.join("free.txt"), &text);
    }

    fn logged(&self) -> String {
        fs::read_to_string(self.calls()).unwrap_or_default()
    }
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The logged `worktree` calls that change a tree: `finish`, `gc --apply` and `discard-cache`.
fn changing(logged: &str) -> Vec<&str> {
    logged
        .lines()
        .filter(|line| {
            line.starts_with("worktree finish")
                || line.starts_with("worktree discard-cache")
                || (line.starts_with("worktree gc") && line.contains(" --apply"))
        })
        .collect()
}

/// The JSON report of `conductor gc`.
fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("gc prints one JSON object ({error}): {}", shown(output)))
}

/// The rows of `report` that name a managed tree, by its id: (verdict, row).
fn tree_rows(report: &Value) -> Vec<(String, String, Value)> {
    report["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|row| {
            let tree = row["tree"].as_str()?.to_owned();
            let verdict = row["verdict"].as_str().unwrap_or_default().to_owned();
            Some((tree, verdict, row.clone()))
        })
        .collect()
}

fn ids(rows: &[(String, String, Value)], verdict: &str) -> Vec<String> {
    let mut ids: Vec<String> = rows
        .iter()
        .filter(|(_, row_verdict, _)| row_verdict == verdict)
        .map(|(tree, _, _)| tree.clone())
        .collect();
    ids.sort();
    ids
}

fn sorted(names: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    names.sort();
    names
}

#[test]
fn the_dry_run_lists_each_tree_with_its_verdict_and_size_and_removes_nothing() {
    let case = Case::new("dry-run");
    let output = case.conductor(&["gc", "--dry-run", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let report = report(&output);
    let rows = tree_rows(&report);
    assert_eq!(ids(&rows, "finished"), sorted(&FINISHED), "{report:#}");
    assert_eq!(ids(&rows, "stale"), sorted(&STALE), "{report:#}");
    for (tree, verdict, row) in &rows {
        assert_eq!(row["outcome"], "reclaimable", "{tree}: {row}");
        assert_eq!(row["repository"], "alpha", "{tree}: {row}");
        let bytes = row["bytes"].as_u64().unwrap_or_default();
        assert!(bytes >= 64 * KIB, "{tree} is measured: {row}");
        if verdict == "stale" {
            let detail = row["detail"].as_str().unwrap_or_default();
            assert!(detail.starts_with("stale, kept"), "{tree}: {row}");
            assert!(
                row["paths"][0]
                    .as_str()
                    .is_some_and(|path| path.ends_with("/target")),
                "a stale tree gives back its target/: {row}"
            );
        }
    }
    // Every other tree is a holder gc keeps, with why.
    let holders = report["holders"].as_array().expect("holders");
    let mut kept: Vec<String> = holders
        .iter()
        .filter(|holder| holder["what"] == "managed tree")
        .filter_map(|holder| {
            let path = holder["path"].as_str()?;
            Some(Path::new(path).file_name()?.to_string_lossy().into_owned())
        })
        .collect();
    kept.sort();
    assert_eq!(kept, sorted(&KEPT), "{report:#}");
    let why = |name: &str| {
        let path = case.tree(name).display().to_string();
        holders
            .iter()
            .find(|holder| holder["path"] == path.as_str())
            .and_then(|holder| holder["kept"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    assert!(why("leased").contains("lease"), "{}", why("leased"));
    assert!(why("busy").contains("process"), "{}", why("busy"));
    assert!(
        why("session").contains("live session"),
        "{}",
        why("session")
    );
    assert!(why("dirty").contains("dirty"), "{}", why("dirty"));
    assert!(
        why("remote-moved").contains("unmerged"),
        "{}",
        why("remote-moved")
    );
    assert!(why("fresh").contains("touched"), "{}", why("fresh"));
    // One summary per repository.
    let summary = report["trees"]
        .as_array()
        .expect("a per-repository summary")
        .iter()
        .find(|summary| summary["repository"] == "alpha")
        .cloned()
        .unwrap_or_else(|| panic!("no summary for alpha: {report:#}"));
    assert_eq!(summary["finished"], 5, "{summary}");
    assert_eq!(summary["stale"], 2, "{summary}");
    assert_eq!(summary["kept"], 6, "{summary}");
    // Nothing changed.
    assert_eq!(changing(&case.logged()), Vec::<&str>::new());
    for name in FINISHED.iter().chain(&STALE).chain(&KEPT) {
        assert!(
            case.tree(name).join("target/debug/deps/out.bin").is_file(),
            "{name}'s build output is still there"
        );
    }
}

#[test]
fn the_text_dry_run_ends_with_one_summary_line_per_repository() {
    let case = Case::new("text");
    let output = case.conductor(&["gc", "--dry-run"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| line.starts_with("trees  alpha: "))
        .unwrap_or_else(|| panic!("no summary line for alpha: {}", shown(&output)));
    assert!(line.contains("finished 5 ("), "{line}");
    assert!(line.contains("stale 2 ("), "{line}");
    assert!(line.contains("kept 6 ("), "{line}");
    assert!(line.contains("GiB"), "{line}");
    assert!(
        stdout
            .lines()
            .any(|line| line.contains("stale, kept") && line.contains("unmerged-old")),
        "{stdout}"
    );
}

#[test]
fn gc_finishes_and_removes_the_finished_trees_through_worktree_and_keeps_every_other() {
    let case = Case::new("apply");
    let output = case.conductor(&["gc", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let report = report(&output);
    let logged = case.logged();
    for name in FINISHED {
        assert!(!case.tree(name).exists(), "{name} is removed: {logged}");
        let gc = format!("--id {name} --apply");
        assert!(
            logged
                .lines()
                .any(|line| line.starts_with("worktree gc") && line.contains(&gc)),
            "{name} goes through worktree gc: {logged}"
        );
    }
    for name in FINISHED.iter().filter(|name| **name != "done") {
        assert!(
            logged
                .lines()
                .any(|line| line.starts_with("worktree finish")
                    && line.contains("--discard-cache")
                    && line.contains("--archive")
                    && line.ends_with(&format!(" {name}"))),
            "{name} is finished through worktree: {logged}"
        );
    }
    // A finished registry record is not finished again.
    assert!(
        !logged
            .lines()
            .any(|line| line.starts_with("worktree finish") && line.ends_with(" done")),
        "{logged}"
    );
    // The refused finish is retried once, after target/ is gone.
    let refuses = logged
        .lines()
        .filter(|line| line.starts_with("worktree finish") && line.ends_with(" refuses"))
        .count();
    assert_eq!(refuses, 2, "{logged}");
    let rows = tree_rows(&report);
    let (_, _, row) = rows
        .iter()
        .find(|(tree, _, _)| tree == "refuses")
        .expect("the row of refuses");
    assert_eq!(row["outcome"], "removed", "{row}");
    assert!(
        row["detail"].as_str().is_some_and(
            |detail| detail.contains("archive-unsupported-entry") && detail.contains("target/")
        ),
        "the row says why it retried: {row}"
    );
    for (tree, _, row) in &rows {
        assert_eq!(row["outcome"], "removed", "{tree}: {row}");
        assert!(
            row["freed"].as_u64().unwrap_or_default() > 0,
            "{tree}: {row}"
        );
    }
    // The stale trees are kept and lose only their target/.
    for name in STALE {
        let tree = case.tree(name);
        assert!(tree.join(format!("{name}.txt")).is_file(), "{name} is kept");
        assert!(!tree.join("target").exists(), "{name}'s target/ is gone");
    }
    // Every other tree is untouched.
    for name in KEPT {
        assert!(
            case.tree(name).join("target/debug/deps/out.bin").is_file(),
            "{name} is untouched"
        );
    }
    // Nothing of a kept or stale tree went through worktree.
    for name in STALE.iter().chain(&KEPT) {
        assert!(
            !changing(&logged)
                .iter()
                .any(|line| line.ends_with(&format!(" {name}"))
                    || line.contains(&format!("--id {name} "))),
            "{name}: {logged}"
        );
    }
}

#[test]
fn reclaim_takes_the_finished_trees_first_then_the_stale_build_output_and_records_them() {
    let case = Case::new("reclaim");
    case.readings(&[1]);
    let output = case.conductor(&["host", "reclaim", "--format", "json"]);
    let report = report(&output);
    let steps = report["steps"].as_array().expect("steps");
    let kinds: Vec<&str> = steps
        .iter()
        .filter_map(|step| step["kind"].as_str())
        .collect();
    let finished = kinds
        .iter()
        .filter(|kind| **kind == "finished-tree")
        .count();
    let stale = kinds
        .iter()
        .filter(|kind| **kind == "stale-tree-build-output")
        .count();
    assert_eq!(finished, 5, "{report:#}");
    assert_eq!(stale, 2, "{report:#}");
    // The pass is the first step: every tree step comes before any other kind.
    let first_other = kinds
        .iter()
        .position(|kind| !matches!(*kind, "finished-tree" | "stale-tree-build-output"))
        .unwrap_or(kinds.len());
    assert_eq!(first_other, 7, "{report:#}");
    for name in FINISHED {
        assert!(!case.tree(name).exists(), "{name} is removed");
    }
    for name in KEPT {
        assert!(
            case.tree(name).join("target/debug/deps/out.bin").is_file(),
            "{name} is untouched"
        );
    }
    let store = store::open_existing(&case.state()).expect("the store is there");
    let runs = Generated::new(store)
        .reclaim_runs()
        .expect("the reclaim runs are read");
    let recorded: Vec<ReclaimKind> = runs
        .iter()
        .flat_map(|run| run.steps.iter().map(|step| step.kind))
        .collect();
    assert!(
        recorded.contains(&ReclaimKind::FinishedTree)
            && recorded.contains(&ReclaimKind::StaleTreeBuildOutput),
        "{recorded:?}"
    );
}

#[test]
fn reclaim_stops_after_the_tree_that_reached_the_floor() {
    let case = Case::new("floor");
    // Short at the start; enough after the first tree.
    case.readings(&[1, 100]);
    let output = case.conductor(&["host", "reclaim", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let report = report(&output);
    let steps = report["steps"].as_array().expect("steps");
    assert_eq!(steps.len(), 1, "{report:#}");
    assert_eq!(steps[0]["kind"], "finished-tree", "{report:#}");
    let removed = FINISHED
        .iter()
        .filter(|name| !case.tree(name).exists())
        .count();
    assert_eq!(removed, 1, "one tree, then the floor is reached");
}
