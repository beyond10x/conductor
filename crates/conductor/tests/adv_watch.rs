//! Adversary cases for `story:watch-command` (wave 07 U4, pass 1), against `351dbdf`.
//!
//! Each case drives [`Watch`] through its one seam, [`Sources`], as `tests/watch.rs` does: the
//! session list and each run list are `cat` of a file the case writes, the free disk is the
//! number of bytes in a file it writes, the clock is a fixed instant per watch, and the notifier
//! appends its arguments to a file of the case. A watch that is "restarted" is a second [`Watch`]
//! over the same state directory, which is what a second `conductor watch run` is. The one binary
//! case gives the built `conductor` a `PATH` holding only fake `claude`, `gh` and `notify-send`
//! programs it writes under the case's directory, so no live session list, `gh` or notifier is
//! reached; it reads the free disk of `/` through `statvfs`.

mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot;
use conductor_cli::store;
use conductor_cli::watch::{Sources, Watch};
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotId, SnapshotState, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The checkouts root and the managed trees the fixture sessions live under; they do not exist.
const ROOT: &str = "/fixture-home/example-org";
const TREES: &str = "/fixture-home/.local/state/worktree/trees/example-org";

const GIB: u64 = 1 << 30;

const NOTHING: [&str; 0] = [];

fn instant(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("a fixed instant")
}

fn noon() -> OffsetDateTime {
    instant("2026-10-07T12:00:00Z")
}

fn eleven() -> OffsetDateTime {
    instant("2026-10-07T11:00:00Z")
}

fn half_past_eleven() -> OffsetDateTime {
    instant("2026-10-07T11:30:00Z")
}

fn before_midnight() -> OffsetDateTime {
    instant("2026-10-07T23:55:00Z")
}

fn after_midnight() -> OffsetDateTime {
    instant("2026-10-08T00:05:00Z")
}

fn session(id: &str, name: &str, pid: u64, cwd: &str) -> Value {
    json!({"id": format!("bg-{name}"), "sessionId": id, "name": name, "kind": "background",
           "state": "working", "status": "busy", "pid": pid, "startedAt": 1_791_370_800_000_u64,
           "cwd": cwd})
}

fn alpha() -> Value {
    session(
        "a1a1a1a1-0000-4000-8000-000000000001",
        "alpha",
        2101,
        "/fixture-home/example-org/alpha",
    )
}

fn beta() -> Value {
    session(
        "b2b2b2b2-0000-4000-8000-000000000002",
        "beta",
        2102,
        "/fixture-home/example-org/beta",
    )
}

/// `entry` as the list shows a session stopped at the usage limit: `state: blocked`, no `pid`
/// and no `status` (the shape of `tests/fixtures/sessions/claude-agents.json`'s exited entry).
fn stopped(mut entry: Value) -> Value {
    let object = entry.as_object_mut().expect("a session entry");
    object.remove("pid");
    object.remove("status");
    object.insert("state".to_owned(), json!("blocked"));
    entry
}

fn run(id: u64, workflow: &str, status: &str, conclusion: &str, at: &str) -> Value {
    json!({"databaseId": id, "workflowName": workflow, "status": status,
           "conclusion": conclusion, "createdAt": at})
}

struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        isolate_active();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adv_watch")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("runs")).expect("create the case's directory");
        let case = Self { dir };
        case.disk(100 * GIB);
        case
    }

    fn sessions(&self, entries: &[Value]) {
        replace(
            &self.dir.join("agents.json"),
            &Value::from(entries.to_vec()).to_string(),
        );
    }

    fn dispatches(&self, lines: &[Value]) {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        let dir = self.dir.join("dispatches");
        fs::create_dir_all(&dir).expect("create dispatches/");
        replace(&dir.join("2026-10.jsonl"), &text);
    }

    fn runs(&self, repository: &str, runs: &Value) {
        replace(
            &self.dir.join("runs").join(format!("{repository}.json")),
            &runs.to_string(),
        );
    }

    fn disk(&self, bytes: u64) {
        replace(&self.dir.join("free.txt"), &format!("{bytes}\n"));
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state/watch")
    }

    fn kept(&self, file: &str) -> String {
        fs::read_to_string(self.state().join(file)).unwrap_or_default()
    }

    fn notified(&self) -> String {
        fs::read_to_string(self.dir.join("notified.txt")).unwrap_or_default()
    }

    fn projects(&self) -> PathBuf {
        self.dir.join("projects")
    }

    /// The sources of this case with the clock `clock`, transcripts under `projects/` of the
    /// case, and no repository.
    fn sources(&self, clock: fn() -> OffsetDateTime) -> Sources {
        let cat = |file: &str| vec![OsString::from("cat"), self.dir.join(file).into_os_string()];
        Sources {
            agents: cat("agents.json"),
            root: PathBuf::from(ROOT),
            trees: PathBuf::from(TREES),
            records: None,
            projects: self.projects(),
            dispatches: self.dir.join("dispatches"),
            organization: "fixture-org".to_owned(),
            store: None,
            repositories: Vec::new(),
            runs: vec![
                "cat".to_owned(),
                format!("{}/runs/{{repository}}.json", self.dir.display()),
            ],
            disk: self.dir.join("free.txt"),
            free_bytes: written_bytes,
            notify: vec![
                OsString::from("sh"),
                OsString::from("-c"),
                OsString::from("printf '%s\\n' \"$@\" >> \"$0\""),
                self.dir.join("notified.txt").into_os_string(),
            ],
            clock,
            bound: Duration::from_secs(10),
        }
    }

    fn watch(&self, clock: fn() -> OffsetDateTime) -> Watch {
        Watch::new(self.sources(clock), self.state())
    }

    /// A watch at noon reading `main`'s CI of `repositories`.
    fn ci_watch(&self, repositories: &[&str]) -> Watch {
        let mut sources = self.sources(noon);
        sources.repositories = repositories.iter().map(|r| (*r).to_owned()).collect();
        Watch::new(sources, self.state())
    }

    /// Writes the transcript of the session `id` working in `cwd`, where Claude Code keeps it.
    fn transcript(&self, cwd: &str, id: &str, lines: &[Value]) {
        let directory: String = cwd
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let dir = self.projects().join(directory);
        fs::create_dir_all(&dir).expect("create the transcript's directory");
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        fs::write(dir.join(format!("{id}.jsonl")), text).expect("write the transcript");
    }
}

/// The free disk of a case: the number of bytes `file` holds, which [`Case::disk`] writes.
fn written_bytes(file: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(file)?.trim().parse()?)
}

fn replace(file: &Path, text: &str) {
    let next = file.with_extension("next");
    fs::write(&next, text).expect("write a case file");
    fs::rename(&next, file).expect("replace a case file");
}

fn pass(watch: &mut Watch) -> Vec<String> {
    let mut notes = Vec::new();
    watch.pass(false, &mut notes).expect("the pass ends")
}

fn ci_pass(watch: &mut Watch) -> Vec<String> {
    let mut notes = Vec::new();
    watch.pass(true, &mut notes).expect("the pass ends")
}

// ---------------------------------------------------------------------------------------------
// Contract: the state the module doc says the dashboard reads
// ---------------------------------------------------------------------------------------------

fn program(bin: &Path, name: &str, script: &str) {
    let path = bin.join(name);
    fs::write(&path, script).expect("write a fake program");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// Takes a complete snapshot, through the library, into the store under the state directory
/// `state`, in which `repository` had real commits in 7 days: Active.
fn active_snapshot(state: &Path, repository: &'static str) {
    let collector = Collector::new("repositories", move |recorder| {
        recorder.repository(RecordRepository {
            snapshot_id: SnapshotId(Uuid("00000000-0000-0000-0000-000000000000".to_owned())),
            repository: RepositoryName(repository.to_owned()),
            visibility: Visibility::Private,
            archived: false,
            local_checkout: true,
            main_head: CommitSha("0a1b2c3".to_owned()),
            main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
            main_commits_7d: 3,
            real_commits_7d: 1,
            behind_main: 0,
            dirty_files: 0,
            worktrees: 1,
            open_issues: 0,
            oldest_open_issue_at: None,
            latest_release: None,
            latest_release_at: None,
            unreleased_commits: None,
            planning_store_version: None,
            in_catalog: Some(true),
        })?;
        Ok(())
    });
    let store = store::open(state).expect("open the store");
    let taken = snapshot::take(store, &[collector]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
}

/// `watch.rs:41-45`: "Its state is kept under `<state>/watch/`, in the files and formats of the
/// stopgap, which the dashboard reads". The dashboard reads `~/.cache/conductor-watch/`
/// (`dashboard.rs:110-111`); the watch writes `<state>/watch/`, and no `--state-dir` makes the
/// two meet, since the watch's directory always ends in `watch`.
#[test]
fn adv_the_dashboard_reads_main_ci_where_the_watch_run_from_the_conductor_repository_kept_it() {
    let case = Case::new("dashboard");
    let home = case.dir.join("home");
    let root = home.join("example-org/conductor");
    let bin = case.dir.join("bin");
    for dir in [&root, &bin] {
        fs::create_dir_all(dir).expect("create the case's directories");
    }
    program(&bin, "claude", "#!/bin/sh\nprintf '[]\\n'\n");
    program(
        &bin,
        "gh",
        "#!/bin/sh\nprintf '%s\\n' '[{\"databaseId\":1,\"workflowName\":\"CI\",\"status\":\"completed\",\"conclusion\":\"success\",\"createdAt\":\"2026-10-07T10:00:00Z\"}]'\n",
    );
    program(&bin, "notify-send", "#!/bin/sh\nexit 0\n");
    // Wave 08 W4 decision 1: main's CI is read of the Active repositories of the newest
    // complete snapshot in the store the watch's state directory holds.
    active_snapshot(&root.join("state"), "alpha");

    let mut child = common::conductor(&home)
        .args(["watch", "run"])
        .current_dir(&root)
        .env("PATH", &bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the conductor binary starts");
    // The first pass keeps main's CI; it ends the watch only when the real `/` has less than the
    // built-in 100G free, which the binary reads through `statvfs`, so the case stops it once the
    // file is there.
    let written = root.join("state/watch/ci.state");
    let started = Instant::now();
    while !written.is_file() && child.try_wait().expect("ask after the watch").is_none() {
        if started.elapsed() > Duration::from_secs(60) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let output = child
        .wait_with_output()
        .expect("collect the watch's output");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty()
            || (stdout.starts_with("disk low: ")
                && stdout.ends_with("G free on /\n")
                && stdout.lines().count() == 1),
        "stdout {stdout:?}, stderr {stderr:?}"
    );
    assert!(
        written.is_file(),
        "the watch kept main's CI under <state>/watch/"
    );

    let dashboard = conductor_cli::dashboard::Sources::new(home.clone(), root.clone());
    assert!(
        dashboard.ci_state.is_file(),
        "the dashboard reads main's CI from {}, which no watch run wrote; the watch kept it in {}. \
         Its usage-limit and context files are read from {}, the watch keeps them in {}",
        dashboard.ci_state.display(),
        written.display(),
        dashboard.watch.display(),
        root.join("state/watch").display()
    );
}

// ---------------------------------------------------------------------------------------------
// Main's CI: missed and repeated wake-ups
// ---------------------------------------------------------------------------------------------

/// A failed `main` run re-run to success keeps its `databaseId` and `createdAt` on GitHub, so the
/// `before.run != latest.run` guard (`watch.rs:723`) hides "green again"; `ci.state` is turned to
/// `success` silently (`watch.rs:736-737`).
#[test]
fn adv_a_red_main_run_re_run_to_success_is_reported_green_again() {
    let case = Case::new("rerun");
    let mut watch = case.ci_watch(&["alpha"]);
    let older = run(101, "CI", "completed", "success", "2026-10-07T10:00:00Z");
    case.runs("alpha", &json!([older]));
    assert_eq!(ci_pass(&mut watch), NOTHING, "the first CI pass seeds");

    case.runs(
        "alpha",
        &json!([
            run(102, "CI", "completed", "failure", "2026-10-07T11:00:00Z"),
            older
        ]),
    );
    assert_eq!(
        ci_pass(&mut watch),
        ["CI red on main: alpha / CI (run 102)"]
    );

    case.runs(
        "alpha",
        &json!([
            run(102, "CI", "in_progress", "", "2026-10-07T11:00:00Z"),
            older
        ]),
    );
    assert_eq!(ci_pass(&mut watch), NOTHING, "the re-run is in progress");

    case.runs(
        "alpha",
        &json!([
            run(102, "CI", "completed", "success", "2026-10-07T11:00:00Z"),
            older
        ]),
    );
    assert_eq!(
        ci_pass(&mut watch),
        ["CI green again on main: alpha / CI"],
        "run 102 re-ran green; ci.state now holds {:?}",
        case.kept("ci.state")
    );
}

/// A `cancelled` run (69 workflow files under `~/example-org/*/.github/workflows/` set
/// `cancel-in-progress: true`) replaces the kept conclusion, so red then cancelled then green
/// never says green again, and red then cancelled then red says red twice
/// (`watch.rs:724-734`, `before.conclusion` is `cancelled`).
#[test]
fn adv_a_cancelled_run_between_red_and_green_hides_neither_green_again_nor_repeats_red() {
    let case = Case::new("cancelled");
    let mut watch = case.ci_watch(&["alpha"]);
    let sequence = [
        (101, "success", "2026-10-07T10:00:00Z"),
        (102, "failure", "2026-10-07T10:10:00Z"),
        (103, "cancelled", "2026-10-07T10:20:00Z"),
        (104, "success", "2026-10-07T10:30:00Z"),
        (105, "failure", "2026-10-07T10:40:00Z"),
        (106, "cancelled", "2026-10-07T10:50:00Z"),
        (107, "failure", "2026-10-07T11:00:00Z"),
    ];
    let mut lines = Vec::new();
    for (id, conclusion, at) in sequence {
        case.runs(
            "alpha",
            &json!([run(id, "CI", "completed", conclusion, at)]),
        );
        lines.extend(ci_pass(&mut watch));
    }
    assert_eq!(
        lines,
        [
            "CI red on main: alpha / CI (run 102)",
            "CI green again on main: alpha / CI",
            "CI red on main: alpha / CI (run 105)",
        ],
        "red at 102, green at 104, red at 105; 107 fails again while it is still red"
    );
}

/// Each CI pass runs the run list once per repository when it answers; one that exits non-zero
/// is run twice (`repositories::answer`, reached through `watch.rs:811`), so a missing
/// repository or a GitHub outage doubles the `gh` calls of every CI pass, and with the real
/// 120 s bound a hanging `gh` holds the pass, and the lines it already found, for two bounds per
/// repository.
#[test]
fn adv_a_ci_pass_runs_the_run_list_once_per_repository() {
    let case = Case::new("gh-calls");
    let calls = case.dir.join("calls.txt");
    let mut sources = case.sources(noon);
    sources.repositories = vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()];
    sources.runs = vec![
        "sh".to_owned(),
        "-c".to_owned(),
        "printf '%s\\n' \"$1\" >> \"$0\"; exit 1".to_owned(),
        calls.display().to_string(),
        "{repository}".to_owned(),
    ];
    let mut watch = Watch::new(sources, case.state());
    let mut notes = Vec::new();
    watch.pass(true, &mut notes).expect("the pass ends");
    let called = fs::read_to_string(&calls).unwrap_or_default();
    assert_eq!(
        called.lines().count(),
        3,
        "one run-list call per repository and CI pass; the calls were:\n{called}"
    );
}

// ---------------------------------------------------------------------------------------------
// Sessions: noise conductor causes itself, and an exit hidden for good
// ---------------------------------------------------------------------------------------------

/// The watch exits on its first change, and conductor starts the next one when it has handled
/// it. A session conductor stopped in between, with a dispatch line naming it, is compared
/// against the list kept before the stop, while "explained" looks back only 15 minutes from the
/// new pass (`watch.rs:519-520`), so a restart more than 15 minutes after the stop wakes
/// conductor for its own stop.
#[test]
fn adv_a_session_conductor_stopped_while_no_watch_ran_does_not_wake_it() {
    let case = Case::new("stopped-between-runs");
    case.dispatches(&[json!({
        "id": "DSP-20261007-05", "event": "stopped", "session": "b2b2b2b2",
        "what": "beta stopped after its gate", "at": "2026-10-07T11:05:00Z"
    })]);
    case.sessions(&[alpha(), beta()]);
    let mut first = case.watch(eleven);
    assert_eq!(pass(&mut first), NOTHING, "the list is kept at 11:00");

    case.sessions(&[alpha()]);
    let mut next = case.watch(half_past_eleven);
    assert_eq!(
        pass(&mut next),
        NOTHING,
        "conductor stopped beta at 11:05 and said so; the next watch starts at 11:30"
    );
}

/// "Each session id once" (`exited.seen`, `watch.rs:546-554`) keeps a session stopped at a
/// usage limit, resumed with a pid, and then gone from the list for good from ever being
/// reported: after a night when every session hit the limit, no later exit of any of them is.
#[test]
fn adv_a_session_resumed_after_a_usage_limit_stop_is_reported_when_it_leaves() {
    let case = Case::new("resumed-then-gone");
    case.sessions(&[alpha(), beta()]);
    let mut watch = case.watch(noon);
    assert_eq!(pass(&mut watch), NOTHING);

    case.sessions(&[stopped(alpha()), beta()]);
    assert_eq!(
        pass(&mut watch),
        ["session exited (blocked): alpha a1a1a1a1"]
    );
    case.sessions(&[alpha(), beta()]);
    assert_eq!(pass(&mut watch), NOTHING, "alpha resumed: a pid again");

    case.sessions(&[beta()]);
    assert_eq!(
        pass(&mut watch),
        ["session gone: alpha a1a1a1a1"],
        "alpha, live with a pid, left the list"
    );
}

// ---------------------------------------------------------------------------------------------
// Usage limit: repeated across midnight UTC
// ---------------------------------------------------------------------------------------------

/// The key is `<today> <text>` (`watch.rs:582-585`), and the rate-limit line stays in the 30 min
/// window across midnight UTC, so one limit hit at 23:50Z is reported, and notified, twice.
#[test]
fn adv_a_usage_limit_hit_before_midnight_is_reported_once() {
    let case = Case::new("limit-midnight");
    let text = "You've hit your session limit · resets 2:00am (UTC)";
    case.transcript(
        "/fixture-home/example-org/alpha",
        "a1a1a1a1-0000-4000-8000-000000000001",
        &[
            json!({"type": "user", "timestamp": "2026-10-07T23:49:50.000Z",
                   "message": {"role": "user", "content": "Continue."}}),
            json!({"type": "assistant", "timestamp": "2026-10-07T23:50:00.000Z",
                   "error": "rate_limit", "isApiErrorMessage": true,
                   "message": {"role": "assistant", "model": "<synthetic>",
                               "content": [{"type": "text", "text": text}]}}),
        ],
    );
    case.sessions(&[stopped(alpha())]);
    let mut lines = pass(&mut case.watch(before_midnight));
    lines.extend(pass(&mut case.watch(after_midnight)));
    assert_eq!(
        lines,
        [format!("usage limit: {text}; 1 sessions: alpha")],
        "one limit hit at 23:50Z, read at 23:55Z and at 00:05Z; notified:\n{}",
        case.notified()
    );
}

/// Fixes the instance this test binary's process runs (`config::active`), which the watch reads
/// for its roles and its instances, to a scratch config file of one instance without a prefix,
/// before any case reads it: a config file in the real home (one with a `session_prefix`, say)
/// never decides a case. The instance takes the name `CONDUCTOR_INSTANCE` gives, if any, so the
/// variable selects it.
fn isolate_active() {
    static ISOLATED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ISOLATED.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(concat!("isolated-active-", module_path!()));
        fs::create_dir_all(&dir).expect("create the scratch config's directory");
        let name = std::env::var(conductor_cli::config::INSTANCE_VARIABLE)
            .unwrap_or_else(|_| "example-org".to_owned());
        let file = dir.join("conductor.yaml");
        fs::write(
            &file,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: {name}\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: /fixture-home/example-org, trees: \
                 /fixture-home/.local/state/worktree/trees/example-org}}\n"
            ),
        )
        .expect("write the scratch config");
        conductor_cli::config::set_active(Some(&file)).expect("the scratch config loads");
    });
}
