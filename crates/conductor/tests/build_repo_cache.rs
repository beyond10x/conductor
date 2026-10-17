//! `conductor build run` gives a cargo-based command its repository's compile cache
//! (`story:build-run-repo-cache`).
//!
//! Each case runs [`build::run`] over a case directory, as `build_run.rs` does, in a git
//! repository of the case (`repo/`) or a second worktree of it (`tree2/`). The `sccache` it is
//! given (`Sources::sccache`) is a fake script of the case: it starts no server and leaves no
//! process behind; it appends each call, with its process group and the cache variables it was
//! given, to `calls.txt` in its `SCCACHE_DIR`, and answers `--show-stats` with counts that grow by
//! 5 hits and 2 misses a call. The cache home is the case's home (`Sources::home`), never the real
//! one.

mod active;
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::ReclaimReport;
use conductor_cli::host;
use time::OffsetDateTime;

const GIB: u64 = 1 << 30;

/// The fake `sccache`.
const FAKE: &str = r#"#!/bin/sh
mkdir -p "$SCCACHE_DIR"
calls="$SCCACHE_DIR/calls.txt"
echo "$1 pgid=$(awk '{print $5}' /proc/self/stat) idle=$SCCACHE_IDLE_TIMEOUT port=$SCCACHE_SERVER_PORT size=$SCCACHE_CACHE_SIZE" >> "$calls"
case "$1" in
  --show-stats)
    n=$(grep -c -- '^--show-stats' "$calls")
    printf '{"stats":{"cache_hits":{"counts":{"Rust":%d}},"cache_misses":{"counts":{"Rust":%d}}}}\n' $((n * 5)) $((n * 2))
    ;;
esac
exit 0
"#;

struct Case {
    dir: PathBuf,
}

impl Case {
    /// A case with a git repository `repo/` holding one commit, a Cargo project, and a second
    /// worktree of it, `tree2/`.
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("build_repo_cache")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("repo")).expect("create the case's repository");
        fs::write(
            dir.join("repo/Cargo.toml"),
            "[package]\nname = \"example\"\n",
        )
        .expect("write the case's Cargo.toml");
        fs::write(dir.join("free.txt"), (100 * GIB).to_string()).expect("write free disk");
        let case = Self { dir };
        case.git(&["init", "-q", "repo"]);
        case.git(&["-C", "repo", "add", "Cargo.toml"]);
        case.git(&["-C", "repo", "commit", "-q", "-m", "example"]);
        case.git(&[
            "-C", "repo", "worktree", "add", "-q", "--detach", "../tree2",
        ]);
        fs::create_dir_all(case.dir.join("bin")).expect("create the case's bin");
        let fake = case.fake();
        fs::write(&fake, FAKE).expect("write the fake sccache");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        case
    }

    /// Runs git in the case directory, reading no configuration of the host's user.
    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "example")
            .env("GIT_AUTHOR_EMAIL", "example@example.org")
            .env("GIT_COMMITTER_NAME", "example")
            .env("GIT_COMMITTER_EMAIL", "example@example.org")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }

    fn fake(&self) -> PathBuf {
        self.dir.join("bin").join("sccache")
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home").join("example-user")
    }

    /// The cache directory of the case's repository, `repo`.
    fn cache(&self) -> PathBuf {
        self.home().join(".cache/sccache/repo")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn host(&self) -> PathBuf {
        self.dir.join("host")
    }

    fn sources(&self) -> Sources {
        Sources {
            disk: self.dir.join("free.txt"),
            free_bytes: |file| Ok(fs::read_to_string(file)?.trim().parse()?),
            reclaim: reclaim_nothing,
            clock: OffsetDateTime::now_utc,
            every: Duration::from_millis(50),
            poll: Duration::from_millis(20),
            home: Some(self.home()),
            var_tmp: self.dir.join("var-tmp"),
            scratch: self.dir.join("scratch"),
            say: said_to_file,
            env: |_| None,
            sessions: Vec::new(),
            walk: build::walk,
            sccache: Some(self.fake()),
        }
    }

    /// `sh -c <script>` in `tree`, a script that names cargo, so it is a cargo-based command.
    fn request(&self, tree: &str, script: &str) -> Request {
        Request {
            reserve: Some(GIB),
            repository: Some("example-repo".to_owned()),
            session_name: None,
            command: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                format!("{script} # runs cargo"),
            ],
            measure: Vec::new(),
            cwd: self.dir.join(tree),
        }
    }

    fn run(&self, sources: &Sources, request: &Request) -> Outcome {
        build::run(&self.state(), &self.host(), &settings(), sources, request)
            .expect("the run ends")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap_or_default()
    }

    /// What the runs of the case wrote to standard error.
    fn said(&self) -> String {
        self.read("stderr.txt")
    }

    /// The calls the fake sccache received.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.cache().join("calls.txt"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

fn settings() -> Settings {
    Settings {
        instance: "example-org".to_owned(),
        fall_key: "example-org".to_owned(),
        floor: 10 * GIB,
        clear: 20 * GIB,
        gate_stop: 5 * GIB,
        gate_resume: 8 * GIB,
        wait: Duration::ZERO,
        cores: 20,
        cpu_reserve: 2,
        slots: 0,
        repo_cache: true,
        repo_cache_size: 2 * GIB,
    }
}

fn reclaim_nothing(_state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

/// Appends `line` to `stderr.txt` of the case, the state directory's parent.
fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let case = state.parent().expect("the state lies in its case");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case.join("stderr.txt"))
        .expect("open the case's stderr.txt");
    writeln!(file, "{line}").expect("write the case's stderr.txt");
}

/// The script that writes the cache variables the command sees to `env.txt` of its tree.
const ENV: &str = "echo \"${RUSTC_WRAPPER-unset}|${SCCACHE_DIR-unset}|${SCCACHE_SERVER_PORT-unset}|\
                   ${SCCACHE_CACHE_SIZE-unset}\" > env.txt";

/// Acceptance 2: a cargo-based command gets `RUSTC_WRAPPER`, `SCCACHE_DIR`, `SCCACHE_SERVER_PORT`
/// and `SCCACHE_CACHE_SIZE`; the directory and the port are the repository's, the same in each of
/// its worktrees, and the port lies outside sccache's own 4226.
#[test]
fn a_cargo_command_gets_its_repositorys_cache_in_every_worktree() {
    let case = Case::new("shared");
    for tree in ["repo", "tree2"] {
        let outcome = case.run(&case.sources(), &case.request(tree, ENV));
        assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    }
    let main = case.read("repo/env.txt");
    let second = case.read("tree2/env.txt");
    assert_eq!(main, second, "both worktrees share the repository's cache");
    let fields: Vec<&str> = main.trim().split('|').collect();
    assert_eq!(fields.len(), 4, "{main}");
    assert_eq!(fields[0], case.fake().display().to_string(), "{main}");
    assert_eq!(fields[1], case.cache().display().to_string(), "{main}");
    let port: u16 = fields[2].parse().expect("the port is a number");
    assert_eq!(port, build::cache_port("repo"), "{main}");
    assert_eq!(fields[3], "2G", "{main}");
}

/// Acceptance 2: the port is derived stably from the repository's name, never sccache's 4226.
#[test]
fn the_port_is_stable_and_never_sccaches_own() {
    for name in ["repo", "conductor", "connectors", "ess", "example", "a", ""] {
        let port = build::cache_port(name);
        assert_eq!(port, build::cache_port(name), "stable for {name:?}");
        assert_ne!(port, 4226, "{name:?}");
        assert!(port >= 1024, "{name:?}: {port}");
    }
    assert_ne!(
        build::cache_port("conductor"),
        build::cache_port("connectors")
    );
}

/// Acceptance 2: a variable the environment of `build run` already sets is left as it is; a
/// wrapper it sets keeps every cache variable unset by `build run` and starts nothing.
#[test]
fn a_cache_variable_the_environment_sets_is_kept() {
    let case = Case::new("kept");
    let inherited = |name: &str| std::env::var(name).unwrap_or_else(|_| "unset".to_owned());
    let sized = Sources {
        env: |name| (name == "SCCACHE_CACHE_SIZE").then(|| "9G".to_owned()),
        ..case.sources()
    };
    case.run(&sized, &case.request("repo", ENV));
    let line = case.read("repo/env.txt");
    let fields: Vec<&str> = line.trim().split('|').collect();
    assert_eq!(fields[1], case.cache().display().to_string(), "{line}");
    assert_eq!(fields[3], inherited("SCCACHE_CACHE_SIZE"), "{line}");

    let wrapped = Sources {
        env: |name| (name == "RUSTC_WRAPPER").then(|| "example-wrapper".to_owned()),
        ..case.sources()
    };
    fs::remove_dir_all(case.cache()).expect("the first run made the cache directory");
    case.run(&wrapped, &case.request("tree2", ENV));
    let line = case.read("tree2/env.txt");
    let fields: Vec<&str> = line.trim().split('|').collect();
    assert_eq!(fields[0], inherited("RUSTC_WRAPPER"), "{line}");
    assert_eq!(fields[1], inherited("SCCACHE_DIR"), "{line}");
    assert!(case.calls().is_empty(), "no server: {:?}", case.calls());
}

/// Acceptance 2: a command that is not cargo-based gets no cache, and `thresholds.repo_cache`
/// `off` gives none to a cargo-based one.
#[test]
fn no_cache_for_a_command_that_runs_no_cargo_or_with_the_cache_off() {
    let case = Case::new("not-cargo");
    let plain = Request {
        command: vec!["sh".to_owned(), "-c".to_owned(), ENV.to_owned()],
        ..case.request("repo", ENV)
    };
    case.run(&case.sources(), &plain);
    let inherited = std::env::var("SCCACHE_DIR").unwrap_or_else(|_| "unset".to_owned());
    let line = case.read("repo/env.txt");
    assert_eq!(
        line.trim().split('|').nth(1),
        Some(inherited.as_str()),
        "{line}"
    );
    let off = Settings {
        repo_cache: false,
        ..settings()
    };
    build::run(
        &case.state(),
        &case.host(),
        &off,
        &case.sources(),
        &case.request("tree2", ENV),
    )
    .expect("the run ends");
    let line = case.read("tree2/env.txt");
    assert_eq!(
        line.trim().split('|').nth(1),
        Some(inherited.as_str()),
        "{line}"
    );
    assert!(case.calls().is_empty(), "no server: {:?}", case.calls());
}

/// Acceptance 3: the repository's server is started before the command, outside the command's
/// process group, with an idle timeout.
#[test]
fn the_server_starts_outside_the_commands_process_group_with_an_idle_timeout() {
    let case = Case::new("detached");
    let outcome = case.run(
        &case.sources(),
        &case.request("repo", "awk '{print $5}' /proc/self/stat > pgid.txt"),
    );
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    let command_group = case.read("repo/pgid.txt").trim().to_owned();
    assert!(!command_group.is_empty());
    let calls = case.calls();
    let start = calls
        .iter()
        .find(|call| call.starts_with("--start-server "))
        .unwrap_or_else(|| panic!("the server is started: {calls:?}"));
    assert!(
        !start.contains(&format!("pgid={command_group} ")),
        "the server is outside the command's group {command_group}: {start}"
    );
    let idle = start
        .split(' ')
        .find_map(|word| word.strip_prefix("idle="))
        .unwrap_or_default();
    assert!(
        idle.parse::<u64>().is_ok_and(|seconds| seconds > 0),
        "SCCACHE_IDLE_TIMEOUT is set: {start}"
    );
    assert!(
        start.contains(&format!("port={} ", build::cache_port("repo"))),
        "{start}"
    );
}

/// Acceptance 4: with no sccache on `PATH`, nothing is set, one line says so, and the build runs.
#[test]
fn with_no_sccache_on_path_nothing_is_set_and_the_build_runs() {
    let case = Case::new("no-sccache");
    let sources = Sources {
        sccache: None,
        ..case.sources()
    };
    let outcome = case.run(&sources, &case.request("repo", &format!("{ENV}; exit 3")));
    assert_eq!(outcome.exit_code(), 3, "the command ran: {outcome:?}");
    let inherited = std::env::var("RUSTC_WRAPPER").unwrap_or_else(|_| "unset".to_owned());
    let line = case.read("repo/env.txt");
    assert_eq!(
        line.trim().split('|').next(),
        Some(inherited.as_str()),
        "{line}"
    );
    let said = case.said();
    let lines: Vec<&str> = said
        .lines()
        .filter(|line| line.contains("sccache"))
        .collect();
    assert_eq!(lines.len(), 1, "one line says so: {said}");
    assert!(lines[0].contains("not on PATH"), "{said}");
}

/// Acceptance 5: the grant records the cache's hits and misses over its run.
#[test]
fn each_grant_records_the_cache_hits_and_misses_of_its_run() {
    let case = Case::new("counts");
    let outcome = case.run(&case.sources(), &case.request("repo", "true"));
    let Outcome::Ran { ref grant, .. } = outcome else {
        panic!("the command ran: {outcome:?}");
    };
    let rows = host::rows(&case.host()).expect("the ledger reads");
    let row = rows
        .iter()
        .find(|row| row.grant_id == *grant)
        .expect("the ledger holds the grant");
    assert_eq!(
        (row.cache_hits, row.cache_misses),
        (Some(5), Some(2)),
        "{row:?}"
    );
    // A run given no cache records none.
    let sources = Sources {
        sccache: None,
        ..case.sources()
    };
    let outcome = case.run(&sources, &case.request("repo", "true"));
    let Outcome::Ran { ref grant, .. } = outcome else {
        panic!("the command ran: {outcome:?}");
    };
    let rows = host::rows(&case.host()).expect("the ledger reads");
    let row = rows
        .iter()
        .find(|row| row.grant_id == *grant)
        .expect("the ledger holds the grant");
    assert_eq!((row.cache_hits, row.cache_misses), (None, None), "{row:?}");
}

/// Acceptance 5: `host grants --format json` shows the counts a release recorded.
#[test]
fn host_grants_shows_the_cache_counts() {
    let case = Case::new("grants-json");
    let home = case.dir.join("home");
    let state = case.state();
    let run = |args: &[&str]| {
        common::conductor(&home)
            .arg("--state-dir")
            .arg(&state)
            .args(args)
            .current_dir(case.dir.join("repo"))
            .output()
            .expect("conductor runs")
    };
    let requested = run(&[
        "host",
        "request-disk",
        "--instance",
        "example-org",
        "--bytes",
        "1024",
        "--command",
        "cargo test",
        "--requested-at",
        "2026-10-13T12:00:00Z",
    ]);
    assert!(requested.status.success(), "{requested:?}");
    let grant = String::from_utf8(requested.stdout)
        .unwrap()
        .trim()
        .to_owned();
    for args in [
        vec![
            "host",
            "reserve-disk",
            "--grant-id",
            &grant,
            "--shortfall-bytes",
            "0",
            "--reclaimed-bytes",
            "0",
        ],
        vec![
            "host",
            "start-run",
            "--grant-id",
            &grant,
            "--process-group",
            "4242",
            "--started-at",
            "2026-10-13T12:00:01Z",
        ],
        vec![
            "host",
            "release-grant",
            "--grant-id",
            &grant,
            "--reason",
            "Exited",
            "--released-at",
            "2026-10-13T12:00:02Z",
            "--cache-hits",
            "12",
            "--cache-misses",
            "3",
        ],
    ] {
        let output = run(&args);
        assert!(output.status.success(), "{args:?}: {output:?}");
    }
    let json = run(&["host", "grants", "--format", "json"]);
    let rows: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json rows");
    assert_eq!(rows[0]["cache_hits"], 12, "{rows}");
    assert_eq!(rows[0]["cache_misses"], 3, "{rows}");
}
