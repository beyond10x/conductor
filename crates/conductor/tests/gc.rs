//! `story:conductor-gc`: `conductor gc --dry-run` lists exactly the rows the dashboard's disk panel
//! marks reclaimable, with their bytes, and removes nothing; `conductor gc` removes those rows,
//! and no tree a live session works in, and prints the bytes it freed.
//!
//! Each case is a fixture home of its own: the checkouts root `example-org/` holds two Rust
//! checkouts, `alpha` and `beta`, each with cargo's `target/`; the managed trees of `alpha` lie
//! under the built-in instance's trees directory: `idle` and `live` with build cache the sweep
//! names, `old` and `live-done` eligible for removal. A live session works in `beta`, in `live`
//! and in `live-done`; one that exited worked in `idle`. Beside them lie what the panel measures
//! and never marks reclaimable: archives, a shared build target and rustup toolchains.
//!
//! The binary runs with `HOME` naming the fixture home and a `PATH` whose first directory holds
//! fake `claude`, `worktree` and `cargo` programs: `claude agents --json` prints the session list,
//! the `worktree` dry-runs print fixture JSON, and the removing calls (`worktree discard-cache`,
//! `worktree gc --apply`, `cargo clean`) remove what they name, as the real programs would. Every
//! call is appended to `calls.log`. The panel is served in this process over the same fixture, its
//! programs named by path, and read from `/data.json`.

mod active;
mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use conductor_cli::cli::{Access, LEAVES};
use conductor_cli::dashboard::{self, Sources};
use conductor_cli::disk;
use serde_json::{Value, json};

const KIB: u64 = 1 << 10;

/// What the panel calls the rows `conductor gc` may remove.
const BUILD_CACHE: &str = "build cache in idle managed trees";
const ELIGIBLE: &str = "trees eligible for removal";
const TARGET: &str = "target/ of the checkout";

/// A case directory of its own: `home/`, `bin/` with the fake programs, and the fixture JSON the
/// fakes print.
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

/// Writes the fake program `name` into `bin`.
fn program(bin: &Path, name: &str, script: &str) {
    let path = bin.join(name);
    write(&path, script);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// The bytes the walk the panel uses finds under `path`.
fn allocated(path: &Path) -> u64 {
    disk::tree_size(&[path.to_path_buf()], None)
        .expect("a walk without a deadline ends")
        .allocated
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("gc")
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

    fn checkout(&self, name: &str) -> PathBuf {
        self.home().join("example-org").join(name)
    }

    /// The records directory the binary runs in: no checkout, so not a tracked repository.
    fn records(&self) -> PathBuf {
        self.checkout("conductor")
    }

    fn tree(&self, name: &str) -> PathBuf {
        self.home()
            .join(".local/state/worktree/trees/example-org/alpha")
            .join(name)
    }

    /// One `worktree sweep --dry-run --json` item: an idle tree of `alpha` at `tree`, holding the
    /// cache under its `target/`.
    fn sweep_item(&self, tree: &Path) -> Value {
        json!({
            "record": {"id": tree.file_name().map(|name| name.to_string_lossy().into_owned()),
                       "repository_root": self.checkout("alpha"), "path": tree,
                       "purpose": "p", "owner": "agent", "lifecycle": "active",
                       "created_at": 0, "last_seen_at": 0, "finished_at": null, "head": null},
            "idle_seconds": 172_800,
            "cache": {"id": "wt", "path": tree, "observed_at": 0, "applied": false,
                      "discarded": [{"path": "target", "kind": "cargo-target",
                                     "allocated_bytes": allocated(&tree.join("target"))}],
                      "retained_ignored": [], "retained_bytes": 0, "processes_observed": true},
        })
    }

    /// One `worktree gc --dry-run --json` assessment of `alpha`'s tree at `tree`.
    fn assessment(&self, tree: &Path, eligible: bool) -> Value {
        json!({
            "record": {"id": tree.file_name().map(|name| name.to_string_lossy().into_owned()),
                       "repository_root": self.checkout("alpha"), "path": tree,
                       "purpose": "p", "owner": "agent", "lifecycle": "finished",
                       "created_at": 0, "last_seen_at": 0, "finished_at": 0, "head": null},
            "eligible": eligible,
            "refusal": null,
            "evidence": null,
        })
    }

    /// The fixture the module documentation describes, and the fake programs.
    fn fixture(&self) {
        let home = self.home();
        for name in ["alpha", "beta"] {
            let checkout = self.checkout(name);
            write(&checkout.join(".git/HEAD"), "ref: refs/heads/main\n");
            write(&checkout.join("Cargo.toml"), "[package]\nname = \"x\"\n");
            write(
                &checkout.join("target/CACHEDIR.TAG"),
                "Signature: 8a477f597d28d172789f06886806bc55\n",
            );
            fill(&checkout.join("target/debug/out.bin"), 64 * KIB);
        }
        fs::create_dir_all(self.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(self.records().join("decisions")).expect("create decisions/");
        for name in ["idle", "live", "old", "live-done"] {
            let tree = self.tree(name);
            write(&tree.join(".git"), "gitdir: elsewhere\n");
            fill(&tree.join("src/lib.rs"), 8 * KIB);
            fill(&tree.join("target/debug/out.bin"), 64 * KIB);
        }
        fill(
            &home.join(".local/state/worktree/archives/alpha/one/commits.bundle"),
            16 * KIB,
        );
        fill(
            &home.join(".cache/example-org-target/alpha/debug/out.bin"),
            16 * KIB,
        );
        fill(&home.join(".rustup/toolchains/stable/bin/rustc"), 16 * KIB);
        fs::create_dir_all(self.dir.join("var-tmp")).expect("create the fixture /var/tmp");

        let sessions = json!([
            {"id": "s-beta", "name": "beta", "cwd": self.checkout("beta"), "pid": 101,
             "status": "busy"},
            {"id": "s-live", "name": "live", "cwd": self.tree("live").join("src"), "pid": 102,
             "status": "busy"},
            {"id": "s-done", "name": "done", "cwd": self.tree("live-done"), "pid": 103,
             "status": "idle"},
            {"id": "s-gone", "name": "gone", "cwd": self.tree("idle"), "status": "idle"}
        ]);
        write(&self.dir.join("agents.json"), &sessions.to_string());
        write(
            &self.dir.join("sweep.json"),
            &json!({"version": 4, "ok": true, "items": [
                self.sweep_item(&self.tree("idle")),
                self.sweep_item(&self.tree("live")),
                self.sweep_item(&self.tree("old")),
            ]})
            .to_string(),
        );
        write(
            &self.dir.join("gc.json"),
            &json!({"version": 4, "ok": true, "assessments": [
                self.assessment(&self.tree("old"), true),
                self.assessment(&self.tree("live-done"), true),
                self.assessment(&self.tree("idle"), false),
            ]})
            .to_string(),
        );

        let bin = self.bin();
        let calls = self.calls().display().to_string();
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
                 gc)\n\
                 \x20 case \" $* \" in\n\
                 \x20 *' --apply '*)\n\
                 \x20   while [ \"$#\" -gt 0 ]; do\n\
                 \x20     if [ \"$1\" = --id ]; then rm -rf \"$2\"; shift; fi\n\
                 \x20     shift\n\
                 \x20   done\n\
                 \x20   printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 \x20 *) cat '{gc}' ;;\n\
                 \x20 esac ;;\n\
                 discard-cache)\n\
                 \x20 for tree; do :; done\n\
                 \x20 rm -rf \"$tree/target\"\n\
                 \x20 printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
                 *) exit 2 ;;\n\
                 esac\n",
                sweep = self.dir.join("sweep.json").display(),
                gc = self.dir.join("gc.json").display(),
            ),
        );
        program(
            &bin,
            "cargo",
            &format!(
                "#!/bin/sh\n\
                 printf 'cargo %s\\n' \"$*\" >> '{calls}'\n\
                 [ \"$1\" = clean ] || exit 2\n\
                 while [ \"$#\" -gt 0 ]; do\n\
                 \x20 if [ \"$1\" = --target-dir ]; then rm -rf \"$2\"; shift; fi\n\
                 \x20 shift\n\
                 done\n"
            ),
        );
    }

    /// `conductor gc <args>` from the records directory, with `HOME` naming the fixture home and
    /// the fake programs first on `PATH`.
    fn gc(&self, args: &[&str]) -> Output {
        let path = format!("{}:/usr/bin:/bin", self.bin().display());
        common::conductor(self.home())
            .arg("gc")
            .args(args)
            .current_dir(self.records())
            .env("PATH", path)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// The calls the fake programs logged, one per line.
    fn logged(&self) -> String {
        fs::read_to_string(self.calls()).unwrap_or_default()
    }

    /// Every file under the case's home, with its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("list a case directory") {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    let bytes = fs::read(&path).expect("read a case file");
                    out.insert(path, bytes);
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.home(), &mut out);
        out
    }

    /// The panel's sources over the fixture: as the binary builds them without a config file,
    /// with the programs named by their path and `/var/tmp` a fixture directory.
    fn sources(&self) -> Sources {
        let mut sources = Sources::new(self.home(), self.records());
        let fake = |name: &str, args: &[&str]| {
            let mut command = vec![self.bin().join(name).into_os_string()];
            command.extend(args.iter().map(OsString::from));
            command
        };
        let records = self.records().display().to_string();
        let records = records.as_str();
        sources.agents = fake("claude", &["agents", "--json"]);
        sources.sweep = fake(
            "worktree",
            &["sweep", "--dry-run", "--json", "--repo", records],
        );
        sources.gc = fake(
            "worktree",
            &["gc", "--dry-run", "--json", "--repo", records],
        );
        sources.var_tmp = self.dir.join("var-tmp");
        sources.measure_timeout = Duration::from_secs(30);
        sources
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

/// The panel's `/data.json` once its disk measurement is complete, polled for at most 60 s.
fn panel(sources: Sources) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
    let port = listener.local_addr().expect("the bound address").port();
    std::thread::spawn(move || dashboard::serve(listener, sources));
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .expect("set a read timeout");
        write!(
            stream,
            "GET /data.json HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .expect("send the request");
        let mut raw = String::new();
        stream.read_to_string(&mut raw).expect("read the answer");
        let (_, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
        let data: Value = serde_json::from_str(body).expect("/data.json is JSON");
        if !data["disk"]["measured_at"].is_null() {
            return data;
        }
        assert!(
            Instant::now() < deadline,
            "no measurement in 60 s: {data:#}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A row as both reports carry it: repository, what, count and bytes.
fn key(row: &Value) -> (Option<String>, String, Option<u64>, Option<u64>) {
    (
        row["repository"].as_str().map(str::to_owned),
        row["what"].as_str().expect("a row's what").to_owned(),
        row["count"].as_u64(),
        row["bytes"].as_u64(),
    )
}

/// The logged calls that remove something: `worktree gc --apply`, `worktree discard-cache` and
/// any `cargo` call.
fn removing(logged: &str) -> Vec<&str> {
    logged
        .lines()
        .filter(|line| {
            line.starts_with("cargo ")
                || line.starts_with("worktree discard-cache")
                || (line.starts_with("worktree gc") && line.contains(" --apply"))
        })
        .collect()
}

#[test]
fn gc_is_declared_a_writing_leaf_only_conductor_runs() {
    let declared = LEAVES
        .iter()
        .find(|(words, _)| *words == ["gc"])
        .map(|(_, access)| *access);
    assert_eq!(declared, Some(Access::Writes));
    assert!(!Access::Writes.controller_runs());
}

#[test]
fn the_dry_run_lists_the_panels_reclaimable_rows_and_gc_removes_them_and_no_live_tree() {
    let case = Case::new("reclaim");

    // The dry run: exactly the panel's reclaimable rows, and nothing removed.
    let data = panel(case.sources());
    let marked: Vec<&Value> = data["disk"]["rows"]
        .as_array()
        .expect("disk rows")
        .iter()
        .filter(|row| row["reclaimable"] == true)
        .collect();
    let mut panel_keys: Vec<_> = marked.iter().copied().map(key).collect();
    panel_keys.sort();
    let mut expected = vec![
        (Some("alpha".to_owned()), BUILD_CACHE.to_owned()),
        (Some("alpha".to_owned()), ELIGIBLE.to_owned()),
        (Some("alpha".to_owned()), TARGET.to_owned()),
    ];
    expected.sort();
    let panel_kinds: Vec<_> = panel_keys
        .iter()
        .map(|(repository, what, _, _)| (repository.clone(), what.clone()))
        .collect();
    assert_eq!(panel_kinds, expected, "{data:#}");

    let before = case.files();
    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).expect("gc prints JSON");
    let rows = listed["rows"].as_array().expect("rows");
    let mut gc_keys: Vec<_> = rows.iter().map(key).collect();
    gc_keys.sort();
    assert_eq!(
        gc_keys,
        panel_keys,
        "the dry run lists the panel's reclaimable rows with their bytes: {}\n{data:#}",
        shown(&output)
    );
    assert!(
        rows.iter()
            .all(|row| row["outcome"] == "reclaimable" && row["bytes"].as_u64() > Some(0)),
        "{}",
        shown(&output)
    );
    let paths = |what: &str| -> Vec<String> {
        rows.iter()
            .find(|row| row["what"] == what)
            .and_then(|row| row["paths"].as_array())
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(|path| path.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let text = |path: PathBuf| path.display().to_string();
    assert_eq!(paths(BUILD_CACHE), [text(case.tree("idle"))]);
    assert_eq!(paths(ELIGIBLE), [text(case.tree("old"))]);
    assert_eq!(paths(TARGET), [text(case.checkout("alpha").join("target"))]);
    assert_eq!(
        listed["reclaimable_bytes"].as_u64(),
        Some(
            rows.iter()
                .filter_map(|row| row["bytes"].as_u64())
                .sum::<u64>()
        ),
        "{}",
        shown(&output)
    );
    assert_eq!(case.files(), before, "the dry run removed something");
    let logged = case.logged();
    assert!(
        removing(&logged).is_empty(),
        "the dry run removed through a call:\n{logged}"
    );

    // The run: those rows removed, and nothing else.
    let freed_target = allocated(&case.checkout("alpha").join("target"));
    let freed_old = allocated(&case.tree("old"));
    let freed_cache = allocated(&case.tree("idle").join("target"));
    let output = case.gc(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let logged = case.logged();
    for call in [
        format!("worktree discard-cache --json {}", text(case.tree("idle"))),
        format!(
            "worktree gc --repo {} --apply --json --id {}",
            text(case.checkout("alpha")),
            text(case.tree("old"))
        ),
        format!(
            "cargo clean --manifest-path {} --target-dir {}",
            text(case.checkout("alpha").join("Cargo.toml")),
            text(case.checkout("alpha").join("target"))
        ),
    ] {
        assert!(logged.contains(&call), "no `{call}`:\n{logged}");
    }
    for kept in [
        case.tree("live"),
        case.tree("live-done"),
        case.checkout("beta"),
        case.home().join(".cache"),
        case.home().join(".rustup"),
        case.home().join(".local/state/worktree/archives"),
    ] {
        let kept = text(kept);
        assert!(
            !removing(&logged).iter().any(|line| line.contains(&kept)),
            "a removing call names {kept}:\n{logged}"
        );
    }
    assert!(!case.tree("idle").join("target").exists());
    assert!(case.tree("idle").join("src/lib.rs").is_file());
    assert!(!case.tree("old").exists());
    assert!(!case.checkout("alpha").join("target").exists());
    for kept in [
        case.tree("live").join("target/debug/out.bin"),
        case.tree("live-done").join("target/debug/out.bin"),
        case.checkout("beta").join("target/debug/out.bin"),
        case.home()
            .join(".local/state/worktree/archives/alpha/one/commits.bundle"),
        case.home()
            .join(".cache/example-org-target/alpha/debug/out.bin"),
        case.home().join(".rustup/toolchains/stable/bin/rustc"),
    ] {
        assert!(kept.is_file(), "{} was removed", kept.display());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let freed: u64 = stdout
        .lines()
        .last()
        .and_then(|line| line.strip_prefix("freed "))
        .and_then(|rest| rest.strip_suffix(" bytes"))
        .and_then(|bytes| bytes.parse().ok())
        .unwrap_or_else(|| panic!("no last line `freed <n> bytes`: {}", shown(&output)));
    assert_eq!(
        freed,
        freed_target + freed_old + freed_cache,
        "{}",
        shown(&output)
    );
    for what in [BUILD_CACHE, ELIGIBLE, TARGET] {
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("removed") && line.contains(what)),
            "no removed row {what:?}: {}",
            shown(&output)
        );
    }
}

#[test]
fn gc_removes_nothing_when_the_live_sessions_cannot_be_read() {
    let case = Case::new("unread");
    program(&case.bin(), "claude", "#!/bin/sh\nexit 3\n");
    let before = case.files();
    for args in [&["--dry-run"][..], &[][..]] {
        let output = case.gc(args);
        assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.starts_with("conductor: gc: ") && stderr.contains("live sessions"),
            "{}",
            shown(&output)
        );
        assert_eq!(
            output.stderr.iter().filter(|&&byte| byte == b'\n').count(),
            1
        );
    }
    assert_eq!(case.files(), before, "gc removed something");
    let logged = case.logged();
    assert!(
        removing(&logged).is_empty(),
        "gc removed through a call:\n{logged}"
    );
}

/// Integrity conformance check of the re-check before each row (`gc.rs` `remove`): a live session
/// that starts in the checkout `alpha` after the listing, its working directory the checkout
/// itself and not `target/`, must make `gc` refuse the checkout's `target/` row, exit 1, and run
/// no `cargo clean`. The listing kept clear of the checkout (`waste::cargo_target` asks whether a
/// session works in the checkout); the re-check asks only of the row's paths, which for a target
/// are `target/` alone.
#[test]
fn a_session_that_starts_in_the_checkout_after_the_listing_keeps_its_target() {
    let case = Case::new("late-checkout-session");
    let mut later: Value = serde_json::from_str(
        &fs::read_to_string(case.dir.join("agents.json")).expect("read the session list"),
    )
    .expect("the session list is JSON");
    later.as_array_mut().expect("a session array").push(json!({
        "id": "s-alpha", "name": "alpha", "cwd": case.checkout("alpha"),
        "pid": 104, "status": "busy"
    }));
    write(&case.dir.join("agents-later.json"), &later.to_string());
    // The first call is the listing's; every later one, a re-check, sees the new session.
    program(
        &case.bin(),
        "claude",
        &format!(
            "#!/bin/sh\n\
             count='{count}'\n\
             if [ -e \"$count\" ]; then cat '{later}'; else : > \"$count\"; cat '{first}'; fi\n",
            count = case.dir.join("claude.count").display(),
            later = case.dir.join("agents-later.json").display(),
            first = case.dir.join("agents.json").display(),
        ),
    );

    let output = case.gc(&[]);
    let logged = case.logged();
    assert!(
        !logged.lines().any(|line| line.starts_with("cargo ")),
        "cargo clean ran on a checkout a live session now works in:\n{logged}\n{}",
        shown(&output)
    );
    assert!(
        case.checkout("alpha")
            .join("target/debug/out.bin")
            .is_file(),
        "the target of a checkout a live session now works in was removed: {}",
        shown(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("refused") && line.contains(TARGET)),
        "no refused target row: {}",
        shown(&output)
    );
}

/// A process started for a case, stopped when the case ends, pass or fail.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_process_working_in_the_checkout_keeps_its_target() {
    let case = Case::new("process-in-checkout");
    // No session works in alpha's checkout; a plain process does, as a build started there would.
    let _sleeper = Running(
        Command::new("sleep")
            .arg("120")
            .current_dir(case.checkout("alpha"))
            .stdin(Stdio::null())
            .spawn()
            .expect("start a process in the checkout"),
    );

    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).expect("gc prints JSON");
    let whats: Vec<&str> = listed["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|row| row["what"].as_str())
        .collect();
    assert!(
        !whats.contains(&TARGET),
        "the target of a checkout a process works in is listed: {}",
        shown(&output)
    );
    assert!(
        whats.contains(&BUILD_CACHE) && whats.contains(&ELIGIBLE),
        "{}",
        shown(&output)
    );

    let output = case.gc(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let logged = case.logged();
    assert!(
        !logged.lines().any(|line| line.starts_with("cargo ")),
        "cargo clean ran on a checkout a process works in:\n{logged}"
    );
    assert!(
        case.checkout("alpha")
            .join("target/debug/out.bin")
            .is_file(),
        "the target of a checkout a process works in was removed: {}",
        shown(&output)
    );
}
