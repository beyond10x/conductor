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

    /// The `--repo` arguments of `worktree prune-archives`, written by [`strip_fixture`]'s fake.
    fn repos(&self) -> PathBuf {
        self.dir.join("repos.log")
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

/// The logged calls that remove something: `worktree gc --apply`, `worktree discard-cache`,
/// `worktree prune-archives … --apply` and any `cargo` call.
fn removing(logged: &str) -> Vec<&str> {
    logged
        .lines()
        .filter(|line| {
            line.starts_with("cargo ")
                || line.starts_with("worktree discard-cache")
                || (line.starts_with("worktree gc") && line.contains(" --apply"))
                || (line.starts_with("worktree prune-archives") && line.contains(" --apply"))
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

/// What `conductor gc` calls the strip of build output from worktree archives.
const ARCHIVE_BUILD_OUTPUT: &str = "build output in worktree archives";

/// The `manifest.json` `worktree` writes into the archive at `archive`: a record of a managed tree
/// of `alpha`, so the archive is one of the instance's.
fn manifest(case: &Case, archive: &Path) {
    let name = archive
        .file_name()
        .expect("an archive directory")
        .to_string_lossy()
        .into_owned();
    write(
        &archive.join("manifest.json"),
        &json!({"format": "worktree.archive/1", "id": name,
                "repository_root": case.checkout("alpha"), "path": case.tree(&name)})
        .to_string(),
    );
}

/// The fixture's archives of `alpha`, under the case's home: `one` holds 32 KiB of build output
/// in its patch `one/dirty.patch`, `two` holds none. A `worktree` first on `PATH` answers
/// `prune-archives` as the real one would over them, and with `--apply` empties `one`'s patch;
/// every other call goes to the fixture's fake. Each `prune-archives` call is appended to
/// `calls.log`, and its `--repo` argument to `repos.log`, one line `list<TAB><repo>` or
/// `apply<TAB><repo>`. Returns the archives' directory.
fn strip_fixture(case: &Case) -> PathBuf {
    let archives = case.home().join(".local/state/worktree/archives/alpha");
    let patch = archives.join("one/dirty.patch");
    fill(&patch, 32 * KIB);
    fill(&archives.join("two/commits.bundle"), 8 * KIB);
    for name in ["one", "two"] {
        manifest(case, &archives.join(name));
    }
    let assessment = |name: &str, verdict: &str, strip: u64| {
        json!({
            "archive_directory": name, "path": archives.join(name), "worktree_id": name,
            "repository_root": case.checkout("alpha"), "bytes": 48 * KIB, "verdict": verdict,
            "reason": "r", "strip_bytes": strip, "sections_stripped": u64::from(strip > 0),
            "sections_kept": 0, "kept_patch_bytes": 0, "nested_images": 0,
            "prune_verdict_after": null, "applied": false, "freed_bytes": 0,
        })
    };
    write(
        &case.dir.join("strip.json"),
        &json!({"version": 4, "ok": true, "applied": false, "archives": [
            assessment("one", "strippable", 32 * KIB),
            assessment("two", "nothing-to-strip", 0),
        ], "skipped": [], "totals": {"archives": 2, "strip_bytes": 32 * KIB, "freed_bytes": 0}})
        .to_string(),
    );
    let bin = case.bin();
    fs::rename(bin.join("worktree"), bin.join("worktree-fixture"))
        .expect("keep the fixture's worktree");
    program(
        &bin,
        "worktree",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = prune-archives ]; then\n\
             \x20 printf 'worktree %s\\n' \"$*\" >> '{calls}'\n\
             \x20 repo=; prev=\n\
             \x20 for arg; do\n\
             \x20   if [ \"$prev\" = --repo ]; then repo=$arg; fi\n\
             \x20   prev=$arg\n\
             \x20 done\n\
             \x20 case \" $* \" in\n\
             \x20 *' --apply '*)\n\
             \x20   printf 'apply\\t%s\\n' \"$repo\" >> '{repos}'\n\
             \x20   : > '{patch}'; printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
             \x20 *)\n\
             \x20   printf 'list\\t%s\\n' \"$repo\" >> '{repos}'\n\
             \x20   cat '{strip}' ;;\n\
             \x20 esac\n\
             \x20 exit 0\n\
             fi\n\
             exec '{fixture}' \"$@\"\n",
            calls = case.calls().display(),
            repos = case.repos().display(),
            patch = patch.display(),
            strip = case.dir.join("strip.json").display(),
            fixture = bin.join("worktree-fixture").display(),
        ),
    );
    archives
}

/// The `--repo` arguments the fixture's `worktree prune-archives` was given, in order, each with
/// `list` or `apply`.
fn repos(case: &Case) -> Vec<(String, PathBuf)> {
    fs::read_to_string(case.repos())
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let (mode, repo) = line.split_once('\t').unwrap_or((line, ""));
            (mode.to_owned(), PathBuf::from(repo))
        })
        .collect()
}

/// `story:gc-before-pause`: `conductor gc --dry-run` lists, beside the idle trees, the build
/// output `worktree prune-archives --strip-build-output` would strip from the archives, read from
/// that command's own dry run, and strips nothing; `conductor gc` strips it through `worktree
/// prune-archives --strip-build-output --apply`, naming each strippable archive and no other.
///
/// The archives are [`strip_fixture`]'s.
#[test]
fn gc_strips_build_output_from_archives_and_its_dry_run_only_lists_it() {
    let case = Case::new("strip-archives");
    let archives = strip_fixture(&case);
    let patch = archives.join("one/dirty.patch");
    let text = |path: PathBuf| path.display().to_string();

    // The dry run: the strip row, read from worktree's own dry run, and nothing stripped.
    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).expect("gc prints JSON");
    let row = listed["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["what"] == ARCHIVE_BUILD_OUTPUT)
        .unwrap_or_else(|| panic!("no strip row: {}", shown(&output)));
    assert_eq!(row["outcome"], "reclaimable", "{}", shown(&output));
    assert_eq!(row["bytes"].as_u64(), Some(32 * KIB), "{}", shown(&output));
    assert_eq!(row["count"].as_u64(), Some(1), "{}", shown(&output));
    assert_eq!(row["paths"], json!([text(archives.join("one"))]));
    let logged = case.logged();
    assert!(
        logged
            .lines()
            .any(|line| line.starts_with("worktree prune-archives --strip-build-output")),
        "the dry run did not list the strip:\n{logged}"
    );
    assert!(
        removing(&logged).is_empty(),
        "the dry run removed or stripped through a call:\n{logged}"
    );
    assert_eq!(
        fs::metadata(&patch).expect("the patch is there").len(),
        32 * KIB,
        "the dry run stripped the patch"
    );

    // The run: the strippable archive stripped by worktree, the other named nowhere.
    let output = case.gc(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let logged = case.logged();
    let call = format!(
        "worktree prune-archives --strip-build-output --apply --json --repo {} --id alpha/one",
        text(case.checkout("alpha"))
    );
    assert!(
        logged.lines().any(|line| line == call),
        "no `{call}`:\n{logged}"
    );
    assert!(
        !removing(&logged).iter().any(|line| line.contains("two")),
        "a removing call names the archive with nothing to strip:\n{logged}"
    );
    assert_eq!(fs::metadata(&patch).expect("the patch is kept").len(), 0);
    assert!(archives.join("one").is_dir() && archives.join("two/commits.bundle").is_file());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|line| line.starts_with("removed")
            && line.contains("bytes freed of 32768")
            && line.contains(ARCHIVE_BUILD_OUTPUT)),
        "no removed strip row: {}",
        shown(&output)
    );
}

/// `story:gc-archive-profile`: `worktree prune-archives` resolves its profile from `--repo`, and
/// refuses the records directory, which no profile contains. Every call `conductor gc` makes, the
/// listing of each archive (`story:gc-archive-measure`: one per archive in the dry run, none in
/// the run, whose archives the cache answers) and the strip, names the first git checkout under
/// the checkouts root in name order, `alpha`, and never the records directory the binary runs in.
#[test]
fn prune_archives_is_given_the_first_checkout_under_the_checkouts_root_not_the_records() {
    let case = Case::new("strip-archives-repo");
    strip_fixture(&case);
    let alpha = case.checkout("alpha");

    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    assert_eq!(
        repos(&case),
        vec![("list".to_owned(), alpha.clone()); 2],
        "the dry run's listing named another --repo: {}",
        shown(&output)
    );

    let output = case.gc(&[]);
    assert!(output.status.success(), "{}", shown(&output));
    let called = repos(&case);
    assert_eq!(
        called,
        vec![
            ("list".to_owned(), alpha.clone()),
            ("list".to_owned(), alpha.clone()),
            ("apply".to_owned(), alpha),
        ],
        "the run's listing or strip named another --repo:\n{}",
        case.logged()
    );
    assert!(
        called.iter().all(|(_, repo)| repo != &case.records()),
        "a prune-archives call named the records directory:\n{}",
        case.logged()
    );
}

/// `story:gc-archive-profile`: with no git checkout under the checkouts root, `conductor gc`
/// calls no `worktree prune-archives` and keeps the archive strip as a holder, not measured,
/// saying why.
#[test]
fn without_a_checkout_the_archive_strip_is_kept_not_measured() {
    let case = Case::new("strip-archives-no-checkout");
    strip_fixture(&case);
    for name in ["alpha", "beta"] {
        fs::remove_dir_all(case.checkout(name).join(".git")).expect("remove the checkout's .git");
    }

    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).expect("gc prints JSON");
    assert!(
        listed["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|row| row["what"] != ARCHIVE_BUILD_OUTPUT),
        "the strip is listed reclaimable without a checkout: {}",
        shown(&output)
    );
    let holder = listed["holders"]
        .as_array()
        .expect("holders")
        .iter()
        .find(|holder| holder["what"] == ARCHIVE_BUILD_OUTPUT)
        .unwrap_or_else(|| panic!("no strip holder: {}", shown(&output)));
    let kept = holder["kept"].as_str().expect("why it is kept");
    assert!(
        kept.starts_with("not measured (failed: no git checkout under the checkouts root"),
        "{kept}"
    );
    assert!(
        repos(&case).is_empty(),
        "prune-archives was called without a checkout:\n{}",
        case.logged()
    );
}

/// `story:gc-before-pause`: `conductor gc` prints one table of the holders the disk panel
/// measures and gc never reclaims, the Go build cache `~/.cache/go-build` among them, each with
/// its path and size, in the dry run and in the run, and removes none of them.
#[test]
fn gc_names_the_go_build_cache_among_the_holders_it_does_not_reclaim() {
    let case = Case::new("holders");
    let go = case.home().join(".cache/go-build");
    fill(&go.join("0a/0123-d"), 16 * KIB);
    let bytes = allocated(&go);
    let path = go.display().to_string();

    let output = case.gc(&["--dry-run", "--format", "json"]);
    assert!(output.status.success(), "{}", shown(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).expect("gc prints JSON");
    let holders = listed["holders"].as_array().expect("holders");
    let holder = |what: &str| holders.iter().find(|holder| holder["what"] == what);
    let go_row = holder("Go build cache")
        .unwrap_or_else(|| panic!("no Go build cache holder: {}", shown(&output)));
    assert_eq!(go_row["path"], path.as_str());
    assert_eq!(go_row["bytes"].as_u64(), Some(bytes));
    assert_eq!(go_row["kept"], "shared Go build cache");
    assert!(
        holder("rustup toolchains").is_some(),
        "the other holders stay in the table: {}",
        shown(&output)
    );
    assert!(
        listed["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|row| row["what"] != "Go build cache"),
        "the Go build cache is listed as reclaimable: {}",
        shown(&output)
    );

    let line = format!("kept  {bytes} bytes  -  Go build cache  {path}: shared Go build cache");
    for args in [&["--dry-run"][..], &[][..]] {
        let output = case.gc(args);
        assert!(output.status.success(), "{}", shown(&output));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines();
        assert!(
            lines.any(|printed| printed == "measured and not reclaimed by gc:"),
            "no holders table: {}",
            shown(&output)
        );
        assert!(
            lines.any(|printed| printed == line),
            "no `{line}` under the table's header: {}",
            shown(&output)
        );
    }
    assert!(
        go.join("0a/0123-d").is_file(),
        "gc removed the Go build cache"
    );
    let logged = case.logged();
    assert!(
        !removing(&logged).iter().any(|call| call.contains(&path)),
        "a removing call names the Go build cache:\n{logged}"
    );
}

/// The variable that bounds how long `conductor gc` walks archives, in seconds.
const ARCHIVE_BOUND: &str = "CONDUCTOR_GC_ARCHIVE_BOUND";

/// Three archives of `alpha`, `a1`, `a2` and `a3`, each a record of a managed tree of `alpha`
/// with a 4 KiB patch. A `worktree` first on `PATH` answers `prune-archives --id <id>` for the
/// archive `<id>` alone, 4 KiB strippable, unless the file `slow-<name>` lies in the case's
/// directory: then it never answers. Each `prune-archives` call is appended to `calls.log`; every
/// other call goes to the fixture's fake. Returns the archives' directory.
fn measure_fixture(case: &Case) -> PathBuf {
    let root = case.home().join(".local/state/worktree/archives");
    let archives = root.join("alpha");
    for name in ["a1", "a2", "a3"] {
        fill(&archives.join(name).join("dirty.patch"), 4 * KIB);
        manifest(case, &archives.join(name));
    }
    let bin = case.bin();
    fs::rename(bin.join("worktree"), bin.join("worktree-fixture"))
        .expect("keep the fixture's worktree");
    program(
        &bin,
        "worktree",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = prune-archives ]; then\n\
             \x20 printf 'worktree %s\\n' \"$*\" >> '{calls}'\n\
             \x20 id=; prev=\n\
             \x20 for arg; do\n\
             \x20   if [ \"$prev\" = --id ]; then id=$arg; fi\n\
             \x20   prev=$arg\n\
             \x20 done\n\
             \x20 case \" $* \" in *' --apply '*) printf '{{\"version\":4,\"ok\":true}}\\n'; exit 0 ;; esac\n\
             \x20 name=${{id#*/}}\n\
             \x20 if [ -e '{dir}'/slow-\"$name\" ]; then exec sleep 600; fi\n\
             \x20 printf '{{\"version\":4,\"ok\":true,\"applied\":false,\"archives\":[{{\"archive_directory\":\"%s\",\"path\":\"%s/%s\",\"verdict\":\"strippable\",\"strip_bytes\":4096}}],\"skipped\":[]}}\\n' \"$name\" '{root}' \"$id\"\n\
             \x20 exit 0\n\
             fi\n\
             exec '{fixture}' \"$@\"\n",
            calls = case.calls().display(),
            dir = case.dir.display(),
            root = root.display(),
            fixture = bin.join("worktree-fixture").display(),
        ),
    );
    archives
}

/// `conductor gc --dry-run` with the archive bound `bound` in seconds, if any, from the records
/// directory, with a fresh `calls.log`.
fn measured(case: &Case, bound: Option<&str>) -> Output {
    let _ = fs::remove_file(case.calls());
    let path = format!("{}:/usr/bin:/bin", case.bin().display());
    let mut command = common::conductor(case.home());
    command
        .args(["gc", "--dry-run"])
        .current_dir(case.records())
        .env("PATH", path)
        .env_remove(ARCHIVE_BOUND)
        .stdin(Stdio::null());
    if let Some(bound) = bound {
        command.env(ARCHIVE_BOUND, bound);
    }
    command.output().expect("conductor runs")
}

/// The archives the last run walked: the ids its `worktree prune-archives` listings named.
fn walked(case: &Case) -> Vec<String> {
    case.logged()
        .lines()
        .filter(|line| line.starts_with("worktree prune-archives") && !line.contains(" --apply"))
        .filter_map(|line| line.split(" --id ").nth(1))
        .map(str::to_owned)
        .collect()
}

/// The archive rows a text run printed, one per archive measured.
fn archive_rows(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.starts_with("archive  "))
        .map(str::to_owned)
        .collect()
}

/// `story:gc-archive-measure`: gc measures archive by archive and, when its bound ends the walk,
/// prints the archives it measured and a partial row instead of no row, and exits 0; the next run
/// walks only the archive the bound left.
#[test]
fn the_archive_row_is_partial_when_the_bound_ends_the_walk() {
    let case = Case::new("archives-partial");
    measure_fixture(&case);
    write(&case.dir.join("slow-a3"), "");

    let started = Instant::now();
    let output = measured(&case, Some("5"));
    assert!(output.status.success(), "{}", shown(&output));
    assert!(
        started.elapsed() < Duration::from_secs(120),
        "the bound did not stop the walk: {}",
        shown(&output)
    );
    let rows = archive_rows(&output);
    assert_eq!(rows.len(), 2, "{}", shown(&output));
    assert!(
        rows[0].contains("alpha/a1") && rows[1].contains("alpha/a2"),
        "{}",
        shown(&output)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|line| line.starts_with("reclaimable")
            && line.contains(ARCHIVE_BUILD_OUTPUT)
            && line.contains("8192 bytes")
            && line.contains("partial: 2 of 3 archives")),
        "no partial archive row: {}",
        shown(&output)
    );

    // The next run: the two measured are answered from the cache, the third walked.
    fs::remove_file(case.dir.join("slow-a3")).expect("make a3 answer");
    let output = measured(&case, None);
    assert!(output.status.success(), "{}", shown(&output));
    assert_eq!(walked(&case), ["alpha/a3"], "{}", shown(&output));
    assert_eq!(archive_rows(&output).len(), 3, "{}", shown(&output));
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("partial:"),
        "{}",
        shown(&output)
    );
}

/// `story:gc-archive-measure`: an archive's size is cached by its id and modification time, so a
/// second run with nothing changed walks no archive and prints all three, and an archive whose
/// modification time changed is walked again.
#[test]
fn an_unchanged_archive_is_answered_from_the_cache_and_a_changed_one_walked_again() {
    let case = Case::new("archives-cache");
    let archives = measure_fixture(&case);

    let output = measured(&case, None);
    assert!(output.status.success(), "{}", shown(&output));
    assert_eq!(
        walked(&case),
        ["alpha/a1", "alpha/a2", "alpha/a3"],
        "{}",
        shown(&output)
    );
    assert_eq!(archive_rows(&output).len(), 3, "{}", shown(&output));

    let output = measured(&case, None);
    assert!(output.status.success(), "{}", shown(&output));
    assert!(
        walked(&case).is_empty(),
        "an unchanged archive was walked again: {}\n{}",
        case.logged(),
        shown(&output)
    );
    assert_eq!(archive_rows(&output).len(), 3, "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|line| line.starts_with("reclaimable")
            && line.contains(ARCHIVE_BUILD_OUTPUT)
            && line.contains("12288 bytes")),
        "no archive row of all three: {}",
        shown(&output)
    );
    assert!(!stdout.contains("partial:"), "{}", shown(&output));

    let later = std::time::SystemTime::now() + Duration::from_secs(60);
    fs::OpenOptions::new()
        .write(true)
        .open(archives.join("a2/dirty.patch"))
        .expect("open a2's patch")
        .set_modified(later)
        .expect("change a2's modification time");
    let output = measured(&case, None);
    assert!(output.status.success(), "{}", shown(&output));
    assert_eq!(walked(&case), ["alpha/a2"], "{}", shown(&output));
    assert_eq!(archive_rows(&output).len(), 3, "{}", shown(&output));
}
