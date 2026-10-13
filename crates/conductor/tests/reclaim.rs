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
//! every call is logged), and `CONDUCTOR_DISK_READINGS` naming a file of free-disk readings, one
//! taken at the start and one after each step.

mod active;
mod common;

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::{Duration, SystemTime};

use conductor_cli::gc::reclaim::{READINGS_VARIABLE, floor_bytes};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::host::obligations::ReclaimRunsQuery as _;
use conductor_model::host::{ReclaimKind, ReclaimResult, ReclaimRuns};
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
    let hour_ago = SystemTime::now() - Duration::from_secs(3600);
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("list a case directory") {
            age(&entry.expect("an entry").path());
        }
    }
    File::open(path)
        .and_then(|file| file.set_modified(hour_ago))
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
