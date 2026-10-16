//! `story:disk-reclaim`: `conductor host reclaim` frees what needs no decision until free disk is
//! back over the floor (the built-in `disk_admit`, 20G, plus the margin), prints each step with
//! the bytes it freed, never touches a tree a live session or a running gate works in, and,
//! short of the floor, lists what is left and exits 1. Each run is recorded in the store.
//!
//! Each case is a fixture home of its own, the built-in instance's: the checkouts root
//! `example-org/` holds the git checkout `alpha`; the managed trees of `alpha` are `idle-a` and
//! `idle-b` (idle: nothing in them written for an hour), `live` (a live session works in it) and
//! `gate` (its `target/` was just written: a gate runs in it), each with build output under
//! `target/` that `worktree sweep` names; the archive `alpha/one` holds 32 KiB of build output.
//!
//! The binary runs with `HOME` naming the fixture home, `PATH` led by fake `claude` and
//! `worktree` programs (the session list; the `worktree` dry runs print fixture JSON, and
//! `discard-cache` and `prune-archives --apply` remove what they name, as the real ones would;
//! every call is logged), `CONDUCTOR_DISK_READINGS` naming a file of free-disk readings, one
//! taken at the start and one after each step, and `CONDUCTOR_RECLAIM_VAR_TMP` naming the case's
//! `var-tmp/`, read instead of `/var/tmp`: nothing reads or deletes the real one.

mod active;
mod common;

use std::fs::{self, File};
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, SystemTime};

use conductor_cli::config::CONFIG_VARIABLE;
use conductor_cli::gc::reclaim::{READINGS_VARIABLE, VAR_TMP_VARIABLE, floor_bytes};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::host::obligations::ReclaimRunsQuery as _;
use conductor_model::host::{ReclaimKind, ReclaimResult, ReclaimRuns};
use nix::sys::stat::lutimes;
use nix::sys::time::TimeVal;
use serde_json::{Value, json};

const KIB: u64 = 1 << 10;
const GIB: u64 = 1 << 30;

/// The built-in instance's `disk_admit`, in GiB.
const DISK_ADMIT: u64 = 20;

struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
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

/// Sets the modification time of `path` and of everything under it to an hour ago, the entries
/// before the directories that hold them.
fn age(path: &Path) {
    age_by(path, Duration::from_secs(3600));
}

/// Sets the modification time of `path` and of everything under it to `ago` ago, the entries
/// before the directories that hold them.
fn age_by(path: &Path, ago: Duration) {
    let then = SystemTime::now() - ago;
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("list a case directory") {
            age_by(&entry.expect("an entry").path(), ago);
        }
    }
    File::open(path)
        .and_then(|file| file.set_modified(then))
        .expect("set a modification time");
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("reclaim")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
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

    /// The case's `/var/tmp`.
    fn var_tmp(&self) -> PathBuf {
        self.dir.join("var-tmp")
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

    fn archive(&self) -> PathBuf {
        self.home().join(".local/state/worktree/archives/alpha/one")
    }

    /// One `worktree sweep --dry-run --json` item: the tree `name` of `alpha`, holding the cache
    /// under its `target/`.
    fn sweep_item(&self, name: &str) -> Value {
        let tree = self.tree(name);
        json!({
            "record": {"id": name, "repository_root": self.checkout("alpha"), "path": tree,
                       "purpose": "p", "owner": "agent", "lifecycle": "active",
                       "created_at": 0, "last_seen_at": 0, "finished_at": null, "head": null},
            "idle_seconds": 172_800,
            "cache": {"id": "wt", "path": tree, "observed_at": 0, "applied": false,
                      "discarded": [{"path": "target", "kind": "cargo-target",
                                     "allocated_bytes": 64 * KIB}],
                      "retained_ignored": [], "retained_bytes": 0, "processes_observed": true},
        })
    }

    fn fixture(&self) {
        let alpha = self.checkout("alpha");
        write(&alpha.join(".git/HEAD"), "ref: refs/heads/main\n");
        fs::create_dir_all(self.records()).expect("create the records directory");
        for name in ["idle-a", "idle-b", "live", "gate"] {
            let tree = self.tree(name);
            write(&tree.join(".git"), "gitdir: elsewhere\n");
            fill(&tree.join("src/lib.rs"), 8 * KIB);
            fill(&tree.join("target/debug/deps/out.bin"), 64 * KIB);
            if name != "gate" {
                age(&tree);
            }
        }
        let archive = self.archive();
        fill(&archive.join("dirty.patch"), 32 * KIB);
        write(
            &archive.join("manifest.json"),
            &json!({"format": "worktree.archive/1", "id": "one",
                    "repository_root": alpha, "path": self.tree("one")})
            .to_string(),
        );

        let sessions = json!([
            {"id": "s-live", "name": "live", "cwd": self.tree("live").join("src"), "pid": 102,
             "status": "busy"}
        ]);
        write(&self.dir.join("agents.json"), &sessions.to_string());
        write(
            &self.dir.join("sweep.json"),
            &json!({"version": 4, "ok": true, "items": [
                self.sweep_item("idle-a"), self.sweep_item("idle-b"),
                self.sweep_item("live"), self.sweep_item("gate"),
            ]})
            .to_string(),
        );
        write(
            &self.dir.join("gc.json"),
            &json!({"version": 4, "ok": true, "assessments": []}).to_string(),
        );
        write(
            &self.dir.join("strip.json"),
            &json!({"version": 4, "ok": true, "applied": false, "archives": [{
                "archive_directory": "one", "path": archive, "worktree_id": "one",
                "repository_root": alpha, "bytes": 40 * KIB, "verdict": "strippable",
                "reason": "r", "strip_bytes": 32 * KIB, "sections_stripped": 1,
                "sections_kept": 0, "kept_patch_bytes": 0, "nested_images": 0,
                "prune_verdict_after": null, "applied": false, "freed_bytes": 0,
            }], "skipped": [], "totals": {"archives": 1, "strip_bytes": 32 * KIB,
                                          "freed_bytes": 0}})
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
                 printf 'worktree %s\\n' \"$*\" >> '{calls}'\n\
                 case \"$1\" in\n\
                 sweep) cat '{sweep}' ;;\n\
                 gc) cat '{gc}' ;;\n\
                 discard-cache)\n\
                 \x20 for tree; do :; done\n\
                 \x20 rm -rf \"$tree/target\"\n\
                 \x20 printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 prune-archives)\n\
                 \x20 case \" $* \" in\n\
                 \x20 *' --apply '*) : > '{patch}'; printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 \x20 *) cat '{strip}' ;;\n\
                 \x20 esac ;;\n\
                 *) exit 2 ;;\n\
                 esac\n",
                calls = self.calls().display(),
                sweep = self.dir.join("sweep.json").display(),
                gc = self.dir.join("gc.json").display(),
                strip = self.dir.join("strip.json").display(),
                patch = archive.join("dirty.patch").display(),
            ),
        );
    }

    /// Writes the free-disk readings, in GiB, the binary takes in turn, the last one standing.
    fn readings(&self, gib: &[u64]) {
        let text: String = gib.iter().map(|gib| format!("{}\n", gib * GIB)).collect();
        write(&self.dir.join("free.txt"), &text);
    }

    /// `conductor --state-dir <case>/state host reclaim <args>` from the records directory.
    fn reclaim(&self, args: &[&str]) -> Output {
        let path = format!("{}:/usr/bin:/bin", self.bin().display());
        common::conductor(self.home())
            .arg("--state-dir")
            .arg(self.state())
            .args(["host", "reclaim"])
            .args(args)
            .current_dir(self.records())
            .env("PATH", path)
            .env(READINGS_VARIABLE, self.dir.join("free.txt"))
            .env(VAR_TMP_VARIABLE, self.var_tmp())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    fn logged(&self) -> String {
        fs::read_to_string(self.calls()).unwrap_or_default()
    }

    /// The reclaim runs the store under the case's state directory holds.
    fn runs(&self) -> Vec<ReclaimRuns> {
        let store = store::open_existing(&self.state()).expect("the store is there");
        Generated::new(store)
            .reclaim_runs()
            .expect("the reclaim runs are read")
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

/// The calls that removed something: `worktree discard-cache` and `worktree prune-archives …
/// --apply`.
fn removing(logged: &str) -> Vec<&str> {
    logged
        .lines()
        .filter(|line| {
            line.starts_with("worktree discard-cache")
                || (line.starts_with("worktree prune-archives") && line.contains(" --apply"))
        })
        .collect()
}

/// The text step lines `reclaimed  <bytes> bytes  <kind>  <path>`, as (bytes, kind, path).
fn steps(stdout: &str) -> Vec<(u64, String, String)> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("reclaimed  "))
        .map(|rest| {
            let words: Vec<&str> = rest.split("  ").collect();
            let bytes = words[0]
                .strip_suffix(" bytes")
                .and_then(|bytes| bytes.parse().ok())
                .unwrap_or_else(|| panic!("no bytes in {rest:?}"));
            (bytes, words[1].to_owned(), words[2].to_owned())
        })
        .collect()
}

#[test]
fn reclaim_frees_the_idle_trees_and_the_archive_and_leaves_the_live_and_running_trees() {
    let case = Case::new("reaches");
    // At the start, after idle-a, after idle-b, after the archive: over the 25G floor.
    case.readings(&[10, 12, 14, 30]);
    let output = case.reclaim(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);

    let text = |path: PathBuf| path.display().to_string();
    let taken = steps(&stdout);
    let shape: Vec<(String, String)> = taken
        .iter()
        .map(|(_, kind, path)| (kind.clone(), path.clone()))
        .collect();
    assert_eq!(
        shape,
        [
            (
                "idle-tree-build-output".to_owned(),
                text(case.tree("idle-a"))
            ),
            (
                "idle-tree-build-output".to_owned(),
                text(case.tree("idle-b"))
            ),
            ("archive-build-output".to_owned(), text(case.archive())),
        ],
        "{}",
        shown(&output)
    );
    assert!(
        taken.iter().all(|(bytes, _, _)| *bytes > 0),
        "each step prints the bytes it freed: {}",
        shown(&output)
    );
    assert!(
        stdout
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("reclaimed ")),
        "{}",
        shown(&output)
    );

    let logged = case.logged();
    assert_eq!(
        removing(&logged),
        [
            format!(
                "worktree discard-cache --json {}",
                text(case.tree("idle-a"))
            ),
            format!(
                "worktree discard-cache --json {}",
                text(case.tree("idle-b"))
            ),
            format!(
                "worktree prune-archives --strip-build-output --apply --json --repo {} --id \
                 alpha/one",
                text(case.checkout("alpha"))
            ),
        ],
        "only the idle trees and the archive are touched:\n{logged}"
    );
    for name in ["idle-a", "idle-b"] {
        assert!(!case.tree(name).join("target").exists(), "{name}'s target/");
        assert!(
            case.tree(name).join("src/lib.rs").is_file(),
            "{name} is kept"
        );
    }
    for name in ["live", "gate"] {
        assert!(
            case.tree(name).join("target/debug/deps/out.bin").is_file(),
            "{name}'s build output is left"
        );
    }
    assert_eq!(
        fs::metadata(case.archive().join("dirty.patch"))
            .expect("the archive is kept")
            .len(),
        0
    );

    let runs = case.runs();
    assert_eq!(runs.len(), 1, "one run is recorded");
    let run = &runs[0];
    assert_eq!(run.result, ReclaimResult::Reclaimed);
    assert_eq!(run.shortfall_bytes, None);
    let kinds: Vec<ReclaimKind> = run.steps.iter().map(|step| step.kind).collect();
    assert_eq!(
        kinds,
        [
            ReclaimKind::IdleTreeBuildOutput,
            ReclaimKind::IdleTreeBuildOutput,
            ReclaimKind::ArchiveBuildOutput
        ]
    );
    let sum: i64 = run.steps.iter().map(|step| step.bytes).sum();
    assert_eq!(run.reclaimed_bytes, sum);
}

#[test]
fn reclaim_stops_once_the_reading_is_over_the_floor() {
    let case = Case::new("stops");
    // Over the floor after idle-a.
    case.readings(&[10, 30]);
    let output = case.reclaim(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let logged = case.logged();
    assert_eq!(
        removing(&logged),
        [format!(
            "worktree discard-cache --json {}",
            case.tree("idle-a").display()
        )],
        "nothing after the step that reached the floor:\n{logged}"
    );
    assert!(case.tree("idle-b").join("target").is_dir());
    assert_eq!(steps(&String::from_utf8_lossy(&output.stdout)).len(), 1);
}

#[test]
fn reclaim_over_the_floor_frees_nothing_and_records_a_run() {
    let case = Case::new("over");
    case.readings(&[40]);
    let output = case.reclaim(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    assert!(removing(&case.logged()).is_empty(), "{}", case.logged());
    let runs = case.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].result, ReclaimResult::Reclaimed);
    assert!(runs[0].steps.is_empty());
}

#[test]
fn reclaim_short_of_the_floor_lists_what_is_left_and_exits_1() {
    let case = Case::new("short");
    // Every step taken, and 13G free after the last.
    case.readings(&[10, 11, 12, 13]);
    let output = case.reclaim(&["--format", "json"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON object");
    let short = floor_bytes(DISK_ADMIT) - 13 * GIB;
    assert_eq!(report["result"], "floor-not-reached", "{}", shown(&output));
    assert_eq!(report["shortfall_bytes"].as_u64(), Some(short));
    assert_eq!(report["steps"].as_array().map(Vec::len), Some(3));
    let left: Vec<String> = report["left"]
        .as_array()
        .expect("left")
        .iter()
        .map(|left| {
            assert!(left["bytes"].as_u64() > Some(0), "{left}");
            left["path"].as_str().expect("a path").to_owned()
        })
        .collect();
    let gate = case.tree("gate").display().to_string();
    assert!(
        left.iter().any(|path| path.starts_with(&gate)),
        "the running gate's tree is left: {left:?}"
    );
    assert!(
        left.iter()
            .any(|path| path == "alpha: build cache in idle managed trees"),
        "the live tree's build output is left: {left:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("floor not reached: {short} bytes short")),
        "{}",
        shown(&output)
    );
    assert!(
        case.tree("live")
            .join("target/debug/deps/out.bin")
            .is_file()
    );

    let runs = case.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].result, ReclaimResult::FloorNotReached);
    assert_eq!(runs[0].shortfall_bytes, i64::try_from(short).ok());
    assert!(!runs[0].left.is_empty());
}

/// Past the staleness age after which a scratch directory or a cache no process holds is deleted
/// (a day, `conductor_cli::gc::reclaim::STALE`).
const TWO_DAYS: Duration = Duration::from_secs(2 * 24 * 3600);

/// The processes a case starts, killed when it ends, passed or not.
struct Held(Vec<Child>);

impl Drop for Held {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `git <args>` in `dir`, under the case's home.
fn git(dir: &Path, home: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} in {}", dir.display());
}

/// `story:reclaim-scratch-class-c`: freeing disk is class C, stale scratch and caches included.
/// Reclaim deletes an entry of the agents' scratch, of a cache and of `/var/tmp` untouched for
/// longer than the staleness age, and prints one line for each with its bytes. It keeps one that
/// was written lately, one a process holds a file open in, one a process works in, one a live
/// session works in, one holding a git repository with uncommitted work, one that is a git
/// worktree (a tree a session may hold a lease on) and one holding a `target/` directory. A git
/// repository with nothing uncommitted is no reason to keep.
#[test]
fn reclaim_deletes_stale_scratch_and_caches_and_never_what_is_held_dirty_a_tree_or_a_target() {
    let case = Case::new("scratch");
    let home = case.home();
    let scratch = home.join(".cache/claude-tmp");
    let cache = home.join(".cache/go-build");
    let var_tmp = case.var_tmp();

    // Deleted: stale, and nothing holds them.
    let stale = scratch.join("stale");
    fill(&stale.join("out/data.bin"), 64 * KIB);
    let clean = scratch.join("clean-repo");
    fill(&clean.join("build.bin"), 64 * KIB);
    git(&clean, &home, &["init", "-q"]);
    write(&clean.join(".git/info/exclude"), "*.bin\n");
    let cached = cache.join("00");
    fill(&cached.join("0011-d"), 64 * KIB);
    let var_stale = var_tmp.join("stale-entry");
    fill(&var_stale.join("dump.bin"), 64 * KIB);

    // Kept although stale.
    let open = scratch.join("open-file");
    fill(&open.join("log.txt"), 64 * KIB);
    let working = scratch.join("working-dir");
    fill(&working.join("data.bin"), 64 * KIB);
    let session = scratch.join("session");
    fill(&session.join("data.bin"), 64 * KIB);
    let dirty = scratch.join("dirty-repo");
    fill(&dirty.join("notes.txt"), 64 * KIB);
    git(&dirty, &home, &["init", "-q"]);
    let tree = scratch.join("leased-tree");
    write(&tree.join(".git"), "gitdir: elsewhere\n");
    fill(&tree.join("src/lib.rs"), 64 * KIB);
    let built = scratch.join("built");
    fill(&built.join("target/debug/out.bin"), 64 * KIB);
    age_by(&home.join(".cache"), TWO_DAYS);
    age_by(&var_tmp, TWO_DAYS);

    // Kept: written lately.
    let fresh = scratch.join("fresh");
    fill(&fresh.join("data.bin"), 64 * KIB);
    let var_fresh = var_tmp.join("fresh-entry");
    fill(&var_fresh.join("data.bin"), 64 * KIB);

    let sessions = json!([
        {"id": "s-live", "name": "live", "cwd": case.tree("live").join("src"), "pid": 102,
         "status": "busy"},
        {"id": "s-scratch", "name": "scratch", "cwd": session, "pid": 103, "status": "busy"}
    ]);
    write(&case.dir.join("agents.json"), &sessions.to_string());
    let holds_open = Command::new("sleep")
        .arg("300")
        .stdin(File::open(open.join("log.txt")).expect("open the held file"))
        .current_dir(&case.dir)
        .spawn()
        .expect("a process holds a file open");
    let works = Command::new("sleep")
        .arg("300")
        .stdin(Stdio::null())
        .current_dir(&working)
        .spawn()
        .expect("a process works in the scratch");
    let _held = Held(vec![holds_open, works]);

    // Never over the floor: every step is taken.
    case.readings(&[10]);
    let output = case.reclaim(&[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);

    let text = |path: &Path| path.display().to_string();
    let mut deleted: Vec<(String, u64)> = steps(&stdout)
        .into_iter()
        .filter(|(_, kind, _)| kind == "cache")
        .map(|(bytes, _, path)| (path, bytes))
        .collect();
    deleted.sort();
    let mut expected = vec![text(&stale), text(&clean), text(&cached), text(&var_stale)];
    expected.sort();
    assert_eq!(
        deleted
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>(),
        expected,
        "one line for each stale entry deleted: {}",
        shown(&output)
    );
    assert!(
        deleted.iter().all(|(_, bytes)| *bytes >= 64 * KIB),
        "each deletion prints its bytes: {}",
        shown(&output)
    );
    for gone in [&stale, &clean, &cached, &var_stale] {
        assert!(!gone.exists(), "{} is deleted", gone.display());
    }
    for (kept, file) in [
        (&open, "log.txt"),
        (&working, "data.bin"),
        (&session, "data.bin"),
        (&dirty, "notes.txt"),
        (&tree, "src/lib.rs"),
        (&built, "target/debug/out.bin"),
        (&fresh, "data.bin"),
        (&var_fresh, "data.bin"),
    ] {
        assert!(
            kept.join(file).is_file(),
            "{} is kept: {}",
            kept.display(),
            shown(&output)
        );
    }
    assert!(
        scratch.is_dir() && cache.is_dir() && var_tmp.is_dir(),
        "the roots stay"
    );

    let runs = case.runs();
    assert_eq!(runs.len(), 1);
    let caches = runs[0]
        .steps
        .iter()
        .filter(|step| step.kind == ReclaimKind::Cache)
        .count();
    assert_eq!(
        caches, 4,
        "each deletion is a step of the run: {:?}",
        runs[0]
    );
}

/// Sets the modification time of the link `link` itself, not of what it names, to two days ago.
fn age_link(link: &Path) {
    let then = SystemTime::now() - TWO_DAYS;
    let seconds = then
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let time = TimeVal::new(i64::try_from(seconds).expect("a time"), 0);
    lutimes(link, &time, &time).expect("set a link's modification time");
}

impl Case {
    /// `conductor host reclaim <args>` as [`Case::reclaim`] runs it, under a config file whose
    /// instance keeps its records in `records` and its state in the case's state directory.
    fn reclaim_with_records(&self, records: &Path, args: &[&str]) -> Output {
        let config = self.dir.join("conductor.yaml");
        write(
            &config,
            &format!(
                "version: conductor.config/1\ninstances:\n  - name: example-org\n    sources: \
                 [{{github: example-org}}]\n    checkouts: {{root: {:?}, trees: {:?}}}\n    \
                 records: {:?}\n    state: {:?}\n    cache: {:?}\n",
                self.home().join("example-org"),
                self.home().join(".local/state/worktree/trees"),
                records,
                self.state(),
                self.dir.join("cache"),
            ),
        );
        let path = format!("{}:/usr/bin:/bin", self.bin().display());
        common::conductor(self.home())
            .env(CONFIG_VARIABLE, &config)
            .arg("--state-dir")
            .arg(self.state())
            .args(["host", "reclaim"])
            .args(args)
            .current_dir(self.records())
            .env("PATH", path)
            .env(READINGS_VARIABLE, self.dir.join("free.txt"))
            .env(VAR_TMP_VARIABLE, self.var_tmp())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }
}

/// `story:reclaim-alias-protected`, the external review's probe: a scratch root that is itself a
/// link (`~/.cache/claude-tmp` naming `/var/tmp`) leads to the instance's records under it. The
/// root is skipped and named on standard error, and the records stay.
#[test]
fn reclaim_preserves_own_records_reached_through_a_scratch_root_symlink() {
    let case = Case::new("alias-root");
    let records = case.var_tmp().join("records");
    let valuable = records.join("NORTHSTAR.md");
    write(&valuable, "The instance's retained direction and decisions");
    age_by(&case.var_tmp(), TWO_DAYS);
    let root = case.home().join(".cache/claude-tmp");
    fs::create_dir_all(case.home().join(".cache")).expect("create the cache directory");
    symlink(case.var_tmp(), &root).expect("link the scratch root");
    // Never over the floor: every step is taken.
    case.readings(&[10]);
    let output = case.reclaim_with_records(&records, &[]);
    assert!(
        valuable.is_file(),
        "reclaim deleted the instance's records through a linked scratch root: {}",
        shown(&output)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&root.display().to_string()) && stderr.contains("link"),
        "the linked root is named on standard error: {}",
        shown(&output)
    );
}

/// `story:reclaim-alias-protected`: an entry of a scratch root that is a link is removed as a
/// link, never followed: a stale link to a directory outside the instance goes and what it names
/// stays; a link whose target is one of the instance's own places is refused.
#[test]
fn reclaim_removes_a_stale_entry_link_as_a_link_and_refuses_one_into_the_records() {
    let case = Case::new("alias-entry");
    let records = case.dir.join("records-elsewhere");
    let valuable = records.join("NORTHSTAR.md");
    write(&valuable, "The instance's retained direction and decisions");
    let outside = case.dir.join("outside");
    let data = outside.join("data.bin");
    fill(&data, 64 * KIB);
    let scratch = case.home().join(".cache/claude-tmp");
    fs::create_dir_all(&scratch).expect("create the scratch root");
    age_by(&records, TWO_DAYS);
    age_by(&outside, TWO_DAYS);
    let into_records = scratch.join("records-link");
    symlink(&records, &into_records).expect("link to the records");
    let stale = scratch.join("stale-link");
    symlink(&outside, &stale).expect("link outside the instance");
    age_link(&into_records);
    age_link(&stale);
    case.readings(&[10]);
    let output = case.reclaim_with_records(&records, &[]);
    assert!(valuable.is_file(), "the records stay: {}", shown(&output));
    assert!(
        fs::symlink_metadata(&into_records).is_ok_and(|meta| meta.file_type().is_symlink()),
        "a link into the records is refused: {}",
        shown(&output)
    );
    assert!(
        fs::symlink_metadata(&stale).is_err(),
        "the stale link is removed: {}",
        shown(&output)
    );
    assert!(
        data.is_file(),
        "what the removed link named stays: {}",
        shown(&output)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        steps(&stdout)
            .iter()
            .any(|(_, kind, path)| kind == "cache" && *path == stale.display().to_string()),
        "the link's removal is a step: {}",
        shown(&output)
    );
}

/// `story:reclaim-alias-protected`: a link deeper in a scratch entry is never followed, so the
/// entry goes and the records it names stay; and a scratch root reached through a linked parent
/// (`~/.cache` naming another directory) is resolved, so an entry there that is the instance's
/// records is refused although its path does not name them.
#[test]
fn reclaim_never_follows_a_nested_link_into_the_records() {
    let case = Case::new("alias-nested");
    let elsewhere = case.dir.join("elsewhere-cache");
    let records = elsewhere.join("claude-tmp/records");
    let valuable = records.join("NORTHSTAR.md");
    write(&valuable, "The instance's retained direction and decisions");
    let nested = elsewhere.join("claude-tmp/nested");
    fill(&nested.join("deep/data.bin"), 64 * KIB);
    age_by(&elsewhere, TWO_DAYS);
    let link = nested.join("deep/records-link");
    symlink(&records, &link).expect("link to the records");
    age_link(&link);
    // The directory the link was added to, aged again after it.
    File::open(nested.join("deep"))
        .and_then(|dir| dir.set_modified(SystemTime::now() - TWO_DAYS))
        .expect("age the directory");
    symlink(&elsewhere, case.home().join(".cache")).expect("link the cache directory");
    case.readings(&[10]);
    let output = case.reclaim_with_records(&records, &[]);
    assert!(
        valuable.is_file(),
        "reclaim deleted the records through a linked parent or a nested link: {}",
        shown(&output)
    );
    assert!(
        !nested.exists(),
        "the stale entry holding the nested link is deleted: {}",
        shown(&output)
    );
}
