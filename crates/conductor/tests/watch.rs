//! `story:watch-command`: `conductor watch run` wakes conductor on the first change.
//!
//! The pass cases run [`Watch::pass`] in this process through its one seam, [`Sources`]: the
//! session list and each repository's run list are the output of `cat` on a file the case
//! writes, the free disk is the number of bytes in a file it writes, the transcripts are fixtures
//! under `tests/fixtures/watch/`, the clock stands at 2026-10-07T12:00:00Z, and the notifier
//! appends its arguments to a file of the case. The loop cases run [`Watch::run`] on a thread with
//! a 1 s interval and read its lines as they are printed. The last cases start the built binary
//! with `--state-dir` and a `PATH` that holds no program at all, or only fake `claude` and `gh`
//! programs the case writes under its own directory, so it reaches no live session list,
//! transcript, `gh` or `notify-send`; the binary reads the free disk of `/` through `statvfs`, so
//! those cases set thresholds around what the case reads there itself. A case that
//! gives the binary an instance writes a config file under its directory and names it in
//! `CONDUCTOR_CONFIG`; a case without one removes that variable, so no case reads the real home's
//! config. A snapshot the watch reads is taken through the library over a fake collector.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::cli::WatchRunArgs;
use conductor_cli::collect::Collector;
use conductor_cli::collect::gitlab::Forge;
use conductor_cli::config;
use conductor_cli::disk;
use conductor_cli::gc::reclaim::{ReclaimReport, floor_bytes};
use conductor_cli::snapshot;
use conductor_cli::store;
use conductor_cli::watch::{Options, Settings, Sources, Watch};
use conductor_model::config::{Gibibytes, Tokens};
use conductor_model::observation::{
    CommitSha, Mergeability, RecordPullRequest, RecordRepository, RepositoryName, SnapshotId,
    SnapshotState, StartSnapshot, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The checkouts root the fixture sessions live under; it does not exist.
const ROOT: &str = "/fixture-home/example-org";

/// The managed trees the fixture sessions live under; they do not exist.
const TREES: &str = "/fixture-home/.local/state/worktree/trees/example-org";

/// The text every fixture rate-limit line carries.
const LIMIT: &str = "You've hit your session limit · resets 3:40pm (UTC)";

const GIB: u64 = 1 << 30;

/// The clock of every pass case.
fn noon() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:00:00Z", &Rfc3339).expect("a fixed instant")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/watch")
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
        "/fixture-home/.local/state/worktree/trees/example-org/ess/beta-tree",
    )
}

fn conductor() -> Value {
    session(
        "c3c3c3c3-0000-4000-8000-000000000003",
        "conductor",
        2103,
        "/fixture-home/example-org/conductor",
    )
}

fn gamma() -> Value {
    session(
        "e5e5e5e5-0000-4000-8000-000000000005",
        "gamma",
        2105,
        "/fixture-home/example-org/gamma",
    )
}

fn outsider() -> Value {
    session(
        "d4d4d4d4-0000-4000-8000-000000000004",
        "outsider-private",
        2104,
        "/fixture-home/elsewhere/private",
    )
}

fn near_miss() -> Value {
    session(
        "f6f6f6f6-0000-4000-8000-000000000006",
        "near-miss-private",
        2106,
        "/fixture-home/example-org-old/x",
    )
}

/// `entry` as the list shows a session stopped at the usage limit: `state: blocked`, no `pid`
/// and no `status`.
fn stopped(mut entry: Value) -> Value {
    let object = entry.as_object_mut().expect("a session entry");
    object.remove("pid");
    object.remove("status");
    object.insert("state".to_owned(), json!("blocked"));
    entry
}

/// One case's directory under this test target's temporary directory: the session list, the
/// dispatch log, the run lists, the free disk, the notifier's record and `state/`.
struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("watch")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("runs")).expect("create the case's directory");
        let case = Self { dir };
        case.disk(100 * GIB);
        case
    }

    /// Replaces the session list, whole, so a pass never reads half of it.
    fn sessions(&self, entries: &[Value]) {
        replace(
            &self.dir.join("agents.json"),
            &Value::from(entries.to_vec()).to_string(),
        );
    }

    fn dispatches(&self, lines: &[Value]) {
        self.dispatches_of("2026-10", lines);
    }

    /// Replaces the dispatch log of `month` (`YYYY-MM`).
    fn dispatches_of(&self, month: &str, lines: &[Value]) {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        let dir = self.dir.join("dispatches");
        fs::create_dir_all(&dir).expect("create dispatches/");
        replace(&dir.join(format!("{month}.jsonl")), &text);
    }

    fn runs(&self, repository: &str, runs: &Value) {
        replace(
            &self.dir.join("runs").join(format!("{repository}.json")),
            &runs.to_string(),
        );
    }

    /// The free disk read as `bytes`.
    fn disk(&self, bytes: u64) {
        self.readings(&[bytes]);
    }

    /// The free disk read as each of `bytes` in turn, the last one from then on.
    fn readings(&self, bytes: &[u64]) {
        let text: String = bytes.iter().map(|bytes| format!("{bytes}\n")).collect();
        replace(&self.dir.join("free.txt"), &text);
    }

    /// The readings [`Case::readings`] wrote that the watch has not taken yet, the last one
    /// always among them.
    fn unread(&self) -> String {
        fs::read_to_string(self.dir.join("free.txt")).expect("read the readings")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state/watch")
    }

    /// A state file of the watch, or an empty text when it is not there.
    fn kept(&self, file: &str) -> String {
        fs::read_to_string(self.state().join(file)).unwrap_or_default()
    }

    /// What the notifier was called with: one argument per line.
    fn notified(&self) -> String {
        fs::read_to_string(self.dir.join("notified.txt")).unwrap_or_default()
    }

    /// The sources of this case, reading transcripts under `fixtures/watch/<projects>` (none
    /// when `projects` is `None`).
    fn sources(&self, projects: Option<&str>) -> Sources {
        let cat = |file: &str| vec![OsString::from("cat"), self.dir.join(file).into_os_string()];
        let notified = self.dir.join("notified.txt");
        Sources {
            agents: cat("agents.json"),
            root: PathBuf::from(ROOT),
            trees: PathBuf::from(TREES),
            records: None,
            projects: projects.map_or_else(|| self.dir.join("no-projects"), |p| fixtures().join(p)),
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
                notified.into_os_string(),
            ],
            clock: noon,
            bound: Duration::from_secs(10),
        }
    }

    fn watch(&self, projects: Option<&str>) -> Watch {
        Watch::new(self.sources(projects), self.state())
    }
}

/// The free disk of a case: the number of bytes on the first line of `file`, which
/// [`Case::readings`] writes. A line after it is the next reading, so the first is taken off.
fn written_bytes(file: &Path) -> anyhow::Result<u64> {
    let text = fs::read_to_string(file)?;
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let rest: String = lines.map(|line| format!("{line}\n")).collect();
    if !rest.is_empty() {
        replace(file, &rest);
    }
    Ok(first.trim().parse()?)
}

/// Writes `text` to `file` through a sibling file and a rename.
fn replace(file: &Path, text: &str) {
    let next = file.with_extension("next");
    fs::write(&next, text).expect("write a case file");
    fs::rename(&next, file).expect("replace a case file");
}

/// One pass without CI; its lines.
fn pass(watch: &mut Watch) -> Vec<String> {
    passed(watch, false).0
}

/// One pass; its lines and what it wrote to standard error.
fn passed(watch: &mut Watch, ci: bool) -> (Vec<String>, String) {
    let mut notes = Vec::new();
    let lines = watch.pass(ci, &mut notes).expect("the pass ends");
    (lines, String::from_utf8(notes).expect("notes are UTF-8"))
}

const NOTHING: [&str; 0] = [];

// ---------------------------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------------------------

#[test]
fn a_session_that_loses_its_pid_and_turns_blocked_is_reported_once() {
    let case = Case::new("blocked");
    case.sessions(&[alpha(), beta()]);
    let mut watch = case.watch(None);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the first pass has nothing to compare"
    );

    case.sessions(&[stopped(alpha()), beta()]);
    assert_eq!(
        pass(&mut watch),
        ["session exited (blocked): alpha a1a1a1a1"]
    );
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the same exit is not reported twice"
    );
    assert_eq!(case.kept("exited.seen"), "a1a1a1a1\n");

    case.sessions(&[alpha(), beta()]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a session coming back is no change"
    );
    assert_eq!(
        case.kept("exited.seen"),
        "",
        "a session listed with a pid again is forgotten"
    );
    case.sessions(&[stopped(alpha()), beta()]);
    assert_eq!(
        pass(&mut watch),
        ["session exited (blocked): alpha a1a1a1a1"],
        "the next exit of a session that came back is reported"
    );
    assert_eq!(pass(&mut watch), NOTHING, "and only once");
    case.sessions(&[beta()]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a session that was already blocked leaving the list is not reported as gone"
    );
}

#[test]
fn a_session_gone_from_the_list_is_reported_and_a_start_is_not() {
    let case = Case::new("gone");
    case.sessions(&[alpha(), beta()]);
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);

    case.sessions(&[alpha(), beta(), gamma()]);
    assert_eq!(pass(&mut watch), NOTHING, "a start is not reported");

    case.sessions(&[alpha(), gamma()]);
    assert_eq!(pass(&mut watch), ["session gone: beta b2b2b2b2"]);
    assert_eq!(pass(&mut watch), NOTHING);
}

#[test]
fn a_dispatch_line_of_the_last_fifteen_minutes_naming_the_session_explains_its_exit() {
    let case = Case::new("explained");
    case.dispatches(&[
        json!({"id": "DSP-20261007-01", "event": "stopped", "what": "alpha a1a1a1a1 stopped",
               "at": "2026-10-07T11:59:00Z"}),
        json!({"id": "DSP-20261007-02", "to": "beta", "brief": "stop pid 2102 after the gate",
               "sent_at": "2026-10-07T11:50:00Z"}),
        json!({"id": "DSP-20261007-03", "event": "stopped", "what": "gamma e5e5e5e5 stopped",
               "at": "2026-10-07T11:40:00Z"}),
        json!({"id": "DSP-20261007-04", "event": "note", "what": "load 21050 and pid_2105",
               "at": "2026-10-07T11:59:30Z"}),
    ]);
    case.sessions(&[alpha(), beta(), gamma()]);
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);

    case.sessions(&[stopped(alpha()), gamma()]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a line of the last minute names alpha's id, and one of ten minutes ago beta's pid"
    );
    assert_eq!(
        case.kept("exited.seen"),
        "",
        "an explained exit is not marked as reported"
    );

    case.sessions(&[stopped(alpha())]);
    assert_eq!(
        pass(&mut watch),
        ["session gone: gamma e5e5e5e5"],
        "a line of twenty minutes ago explains nothing, and 21050 or pid_2105 is not the word 2105"
    );
}

#[test]
fn a_recent_done_line_of_the_dispatch_whose_sent_line_names_the_session_explains_its_exit() {
    let case = Case::new("explained-by-dispatch");
    case.dispatches(&[
        json!({"id": "DSP-20261007-11", "event": "sent", "at": "2026-10-07T11:00:00Z",
               "to": "beta", "session": "b2b2b2b2", "brief": "one wave"}),
        json!({"id": "DSP-20261007-12", "event": "sent", "at": "2026-10-07T11:00:00Z",
               "to": "gamma", "session": "e5e5e5e5", "brief": "one wave"}),
        json!({"id": "DSP-20261007-11", "event": "done", "at": "2026-10-07T11:59:00Z",
               "reported_by": "beta", "evidence": "released; stopped and removed"}),
        json!({"id": "DSP-20261007-13", "event": "done", "at": "2026-10-07T11:59:00Z",
               "reported_by": "gamma", "evidence": "released; stopped and removed"}),
        json!({"id": "DSP-20260930-01", "event": "done", "at": "2026-10-07T11:59:30Z",
               "reported_by": "alpha", "evidence": "released; stopped and removed"}),
        json!({"id": "DSP-20261007-14", "event": "sent", "at": "2026-10-07T11:00:00Z",
               "to": "delta", "brief": "stop pid 2107 after the gate"}),
        json!({"id": "DSP-20261007-14", "event": "done", "at": "2026-10-07T11:59:00Z",
               "reported_by": "delta", "evidence": "released; stopped and removed"}),
    ]);
    let delta = || {
        session(
            "a7a7a7a7-0000-4000-8000-000000000007",
            "delta",
            2107,
            "/fixture-home/example-org/delta",
        )
    };
    case.dispatches_of(
        "2026-09",
        &[
            json!({"id": "DSP-20260930-01", "event": "sent", "at": "2026-09-30T23:00:00Z",
                   "to": "alpha", "session": "a1a1a1a1", "brief": "one wave"}),
        ],
    );
    case.sessions(&[alpha(), beta(), gamma(), delta(), conductor()]);
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);

    case.sessions(&[alpha(), gamma(), delta(), conductor()]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the done line of a minute ago is of the dispatch whose sent line of an hour ago names beta"
    );

    case.sessions(&[gamma(), delta(), conductor()]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the sent line naming alpha is in the previous month's log"
    );

    case.sessions(&[gamma(), conductor()]);
    assert_eq!(
        pass(&mut watch),
        ["session gone: delta a7a7a7a7"],
        "a line of an hour ago that names only delta's pid links no dispatch to delta"
    );

    case.sessions(&[conductor()]);
    assert_eq!(
        pass(&mut watch),
        ["session gone: gamma e5e5e5e5"],
        "no recent line is of the dispatch that names gamma; the recent done line is another's"
    );
}

#[test]
fn a_session_outside_the_workspace_and_its_managed_trees_is_never_named() {
    let case = Case::new("outside");
    case.sessions(&[alpha(), outsider(), near_miss()]);
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);

    case.sessions(&[alpha()]);
    assert_eq!(pass(&mut watch), NOTHING);
    let kept = case.kept("sessions.json");
    assert!(kept.contains("alpha"), "the kept list holds alpha: {kept}");
    assert!(
        !kept.contains("private"),
        "the kept list names a session outside the workspace: {kept}"
    );
}

/// A second session named as the conductor role: one resumed by hand, without the instance's
/// settings, runs no hook and no guard (`story:watch-two-conductors`).
fn second_conductor(name: &str) -> Value {
    session(
        "c8c8c8c8-0000-4000-8000-000000000008",
        name,
        2108,
        "/fixture-home/example-org/conductor",
    )
}

#[test]
fn two_live_sessions_named_as_the_conductor_role_are_reported_once_as_an_incident() {
    let case = Case::new("two-conductors");
    let dev = || {
        session(
            "c9c9c9c9-0000-4000-8000-000000000009",
            "conductor-dev",
            2109,
            "/fixture-home/example-org/conductor",
        )
    };
    case.sessions(&[conductor(), dev()]);
    let mut watch = case.watch(None);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "conductor-dev in conductor's directory is not named as the conductor role"
    );

    case.sessions(&[conductor(), dev(), stopped(second_conductor("conductor"))]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a second conductor without a pid is not live"
    );

    case.sessions(&[conductor(), dev(), second_conductor("conductor")]);
    assert_eq!(
        pass(&mut watch),
        ["conductors: 2 live sessions named conductor: c3c3c3c3 c8c8c8c8"]
    );
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the same two conductors are not reported twice"
    );

    case.sessions(&[conductor(), dev()]);
    assert_eq!(pass(&mut watch), ["session gone: conductor c8c8c8c8"]);
    case.sessions(&[conductor(), dev(), second_conductor("conductor")]);
    assert_eq!(
        pass(&mut watch),
        ["conductors: 2 live sessions named conductor: c3c3c3c3 c8c8c8c8"],
        "a second conductor after one alone is reported again"
    );
}

#[test]
fn the_conductor_role_s_session_name_is_the_instances() {
    let case = Case::new("two-conductors-prefixed");
    let first = session(
        "cacacaca-0000-4000-8000-00000000000a",
        "acme-conductor",
        2110,
        "/fixture-home/example-org/conductor",
    );
    case.sessions(&[first.clone(), conductor(), second_conductor("conductor")]);
    let mut watch = case.watch(None).with_settings(Settings {
        conductor: Some("acme-conductor".to_owned()),
        ..Settings::default()
    });
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "two sessions named `conductor` are not this instance's conductor role"
    );

    case.sessions(&[first, second_conductor("acme-conductor")]);
    assert_eq!(
        pass(&mut watch),
        [
            "session gone: conductor c3c3c3c3",
            "conductors: 2 live sessions named acme-conductor: c8c8c8c8 cacacaca"
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// Usage limit
// ---------------------------------------------------------------------------------------------

#[test]
fn a_rate_limit_of_a_minute_ago_is_reported_once_a_day_with_a_notification() {
    let case = Case::new("limit-recent");
    case.sessions(&[alpha(), beta(), gamma()]);
    let mut watch = case.watch(Some("recent"));
    let line = format!("usage limit: {LIMIT}; 2 sessions: alpha beta");
    assert_eq!(pass(&mut watch), [line.as_str()]);
    assert_eq!(
        case.notified(),
        format!(
            "example-org: usage limit\n{LIMIT}. 2 sessions stopped. After rotating or the reset, \
             tell conductor to resume.\n"
        )
    );
    assert_eq!(
        case.kept("limit.new"),
        format!("{LIMIT}\talpha\n{LIMIT}\tbeta\n")
    );
    assert_eq!(case.kept("limit.seen"), format!("2026-10-07 {LIMIT}\n"));

    assert_eq!(pass(&mut watch), NOTHING, "once per limit text per day");
    assert_eq!(case.notified().lines().count(), 2, "notified once");
}

fn next_day_before_noon() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-08T11:59:00Z", &Rfc3339).expect("a fixed instant")
}

fn next_day_noon() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-08T12:00:00Z", &Rfc3339).expect("a fixed instant")
}

#[test]
fn a_usage_limit_text_is_reported_again_24_hours_after_its_last_report() {
    let case = Case::new("limit-again");
    let projects = case.dir.join("projects");
    let dir = projects.join("-fixture-home-example-org-alpha");
    fs::create_dir_all(&dir).expect("create the transcript's directory");
    let limit_at = |at: &str| {
        let line = json!({"type": "assistant", "timestamp": at, "error": "rate_limit",
                          "message": {"role": "assistant",
                                      "content": [{"type": "text", "text": LIMIT}]}});
        fs::write(
            dir.join("a1a1a1a1-0000-4000-8000-000000000001.jsonl"),
            format!("{line}\n"),
        )
        .expect("write the transcript");
    };
    case.sessions(&[stopped(alpha())]);
    let at = |clock: fn() -> OffsetDateTime| {
        let mut sources = case.sources(None);
        sources.projects.clone_from(&projects);
        sources.clock = clock;
        pass(&mut Watch::new(sources, case.state()))
    };
    let line = format!("usage limit: {LIMIT}; 1 sessions: alpha");

    limit_at("2026-10-07T11:59:00Z");
    assert_eq!(at(noon), [line.as_str()]);
    limit_at("2026-10-08T11:58:00Z");
    assert_eq!(
        at(next_day_before_noon),
        NOTHING,
        "23 h 59 min after its last report"
    );
    assert_eq!(
        at(next_day_noon),
        [line.as_str()],
        "24 h after its last report"
    );
    assert_eq!(case.notified().lines().count(), 4, "notified twice");
    assert_eq!(
        case.kept("limit.reported"),
        format!("2026-10-08T12:00:00Z {LIMIT}\n")
    );
    assert_eq!(
        case.kept("limit.seen"),
        format!("2026-10-07 {LIMIT}\n2026-10-08 {LIMIT}\n")
    );
}

#[test]
fn a_rate_limit_of_two_hours_ago_is_not_reported() {
    let case = Case::new("limit-stale");
    case.sessions(&[alpha()]);
    let mut watch = case.watch(Some("stale"));
    assert_eq!(pass(&mut watch), NOTHING);
    assert_eq!(case.notified(), "");
    assert_eq!(case.kept("limit.new"), "");
}

#[test]
fn a_rate_limit_above_the_last_twenty_lines_is_not_reported() {
    let case = Case::new("limit-buried");
    case.sessions(&[alpha()]);
    let mut watch = case.watch(Some("buried"));
    assert_eq!(pass(&mut watch), NOTHING);
    assert_eq!(case.notified(), "");
}

// ---------------------------------------------------------------------------------------------
// Conductor's context
// ---------------------------------------------------------------------------------------------

#[test]
fn conductor_context_above_300k_tokens_is_reported_once_per_session() {
    let case = Case::new("context");
    case.sessions(&[alpha(), conductor()]);
    let mut watch = case.watch(Some("context"));
    assert_eq!(
        pass(&mut watch),
        ["context: conductor 352k tokens (session c3c3c3c3)"],
        "the last line with a usage reads 10 + 350000 + 2000 tokens"
    );
    assert_eq!(pass(&mut watch), NOTHING);
    assert_eq!(
        case.kept("context-reported"),
        "c3c3c3c3-0000-4000-8000-000000000003\n"
    );

    case.sessions(&[alpha(), stopped(conductor())]);
    let lines = pass(&mut watch);
    assert!(
        lines.iter().all(|line| !line.starts_with("context:")),
        "a conductor without a pid is not read: {lines:?}"
    );
}

/// conductor-dev works in conductor's directory; labelled by its directory it would read
/// `context: conductor` and make conductor hand itself over.
#[test]
fn conductor_dev_in_conductor_s_directory_is_labelled_by_its_role_name() {
    let case = Case::new("context-dev");
    let dev = session(
        "c3c3c3c3-0000-4000-8000-000000000003",
        "conductor-dev",
        2103,
        "/fixture-home/example-org/conductor",
    );
    case.sessions(&[dev]);
    let mut watch = case.watch(Some("context"));
    assert_eq!(
        pass(&mut watch),
        ["context: conductor-dev 352k tokens (session c3c3c3c3)"]
    );
}

#[test]
fn a_long_transcript_is_read_from_its_end_across_chunks() {
    let case = Case::new("long-transcript");
    let projects = case.dir.join("projects");
    let dir = projects.join("-fixture-home-example-org-conductor");
    fs::create_dir_all(&dir).expect("create the transcript's directory");
    let usage = |cache_read: u64| {
        json!({"type": "assistant", "timestamp": "2026-10-07T11:50:00.000Z",
               "message": {"role": "assistant", "content": [{"type": "text", "text": "Done."}],
                           "usage": {"input_tokens": 1, "cache_read_input_tokens": cache_read,
                                     "cache_creation_input_tokens": 0}}})
    };
    let long = json!({"type": "user", "timestamp": "2026-10-07T11:51:00.000Z",
                      "message": {"role": "user", "content": "y".repeat(200 * 1024)}});
    let limit = json!({"type": "assistant", "timestamp": "2026-10-07T11:59:00.000Z",
                       "error": "rate_limit",
                       "message": {"role": "assistant",
                                   "content": [{"type": "text", "text": "Limit read last"}]}});
    let short = json!({"type": "user", "timestamp": "2026-10-07T11:59:10.000Z",
                       "message": {"role": "user", "content": "Wait."}});
    fs::write(
        dir.join("c3c3c3c3-0000-4000-8000-000000000003.jsonl"),
        format!(
            "{}\n{}\n{long}\n{limit}\n\n{short}\n",
            usage(100_000),
            usage(401_000)
        ),
    )
    .expect("write the transcript");
    case.sessions(&[conductor()]);
    let mut sources = case.sources(None);
    sources.projects = projects;
    let mut watch = Watch::new(sources, case.state());
    assert_eq!(
        pass(&mut watch),
        [
            "usage limit: Limit read last; 1 sessions: conductor",
            "context: conductor 401k tokens (session c3c3c3c3)"
        ],
        "the newest usage lies behind a line of 200 KiB, read 64 KiB at a time"
    );
}

// ---------------------------------------------------------------------------------------------
// Every controller's context (story:controller-context; W4 decision 6)
// ---------------------------------------------------------------------------------------------

/// The fixtures under `controllers/`: alpha's newest usage reads 451k tokens, beta's, in a managed
/// tree of `ess`, 602k, conductor's 352k, and gamma's 120k, after an older line of 400k.
#[test]
fn a_controller_over_the_threshold_gives_one_context_line_once_and_one_under_it_none() {
    let case = Case::new("controllers");
    case.sessions(&[stopped(alpha()), beta(), conductor(), gamma()]);
    let mut watch = case.watch(Some("controllers"));
    assert_eq!(
        pass(&mut watch),
        [
            "context: ess 602k tokens (session b2b2b2b2)",
            "context: conductor 352k tokens (session c3c3c3c3)",
        ],
        "a session in a managed tree is named by the tree's repository, conductor's line keeps \
         its wording, alpha without a pid is not read, and gamma's newest usage is under 300k"
    );

    case.sessions(&[alpha(), beta(), conductor(), gamma()]);
    assert_eq!(
        pass(&mut watch),
        ["context: alpha 451k tokens (session a1a1a1a1)"],
        "alpha is read once it runs again; the others were reported already"
    );
    assert_eq!(pass(&mut watch), NOTHING, "each session once");
    assert_eq!(
        case.kept("context-reported"),
        "b2b2b2b2-0000-4000-8000-000000000002\n\
         c3c3c3c3-0000-4000-8000-000000000003\n\
         a1a1a1a1-0000-4000-8000-000000000001\n"
    );
}

#[test]
fn the_context_threshold_is_the_instances() {
    let case = Case::new("controllers-threshold");
    case.sessions(&[gamma()]);
    let mut watch = case.watch(Some("controllers")).with_settings(Settings {
        context_handover: 100_000,
        ..Settings::default()
    });
    assert_eq!(
        pass(&mut watch),
        ["context: gamma 120k tokens (session e5e5e5e5)"]
    );

    let case = Case::new("controllers-threshold-high");
    case.sessions(&[alpha(), beta(), conductor()]);
    let mut watch = case.watch(Some("controllers")).with_settings(Settings {
        context_handover: 500_000,
        ..Settings::default()
    });
    assert_eq!(
        pass(&mut watch),
        ["context: ess 602k tokens (session b2b2b2b2)"]
    );
}

// ---------------------------------------------------------------------------------------------
// A silent controller (story:watch-silent-controller)
// ---------------------------------------------------------------------------------------------

/// One hour after [`noon`], the clock of a later watch.
fn one_pm() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T13:00:00Z", &Rfc3339).expect("a fixed instant")
}

/// `entry` with the list's `state` set to `state`, its `status` and `pid` kept.
fn in_state(mut entry: Value, state: &str) -> Value {
    let object = entry.as_object_mut().expect("a session entry");
    object.insert("state".to_owned(), json!(state));
    entry
}

/// Writes `file` (and its directories), last modified `minutes` before [`noon`]; a negative
/// `minutes` lies after it.
fn written_before_noon(file: &Path, minutes: i64) {
    fs::create_dir_all(file.parent().expect("a file under a directory"))
        .expect("create the transcript's directory");
    fs::write(file, "{\"type\":\"user\"}\n").expect("write the transcript");
    let at = std::time::SystemTime::from(noon() - time::Duration::minutes(minutes));
    fs::File::options()
        .write(true)
        .open(file)
        .and_then(|file| file.set_modified(at))
        .expect("set the transcript's modification time");
}

/// The directory Claude Code keeps the transcripts of the sessions working in `cwd` in, under
/// `projects`.
fn project(projects: &Path, cwd: &str) -> PathBuf {
    let directory: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    projects.join(directory)
}

/// The transcript of the fixture session `entry` under `projects`.
fn transcript_of(projects: &Path, entry: &Value) -> PathBuf {
    let cwd = entry["cwd"].as_str().expect("a cwd");
    let id = entry["sessionId"].as_str().expect("a sessionId");
    project(projects, cwd).join(format!("{id}.jsonl"))
}

/// A sub-agent transcript of the fixture session `entry` under `projects`, at `under` below its
/// `subagents/`.
fn subagent_of(projects: &Path, entry: &Value, under: &str) -> PathBuf {
    transcript_of(projects, entry)
        .with_extension("")
        .join("subagents")
        .join(under)
}

fn delta() -> Value {
    session(
        "d7d7d7d7-0000-4000-8000-000000000007",
        "delta",
        2107,
        "/fixture-home/example-org/delta",
    )
}

/// At noon: alpha (working) last wrote 40 minutes ago and its sub-agent 35; beta (working) 50
/// minutes ago, but a sub-agent of a workflow nested under its `subagents/` 5; conductor 60, but
/// it is the conductor role, not a controller; delta 90, but it is idle; and gamma (waiting) 45.
/// Alpha and gamma are silent, once each, until one of them writes and falls silent again.
#[test]
fn a_live_controller_whose_transcripts_stay_silent_is_reported_once_per_silence() {
    let case = Case::new("silent");
    let projects = case.dir.join("projects");
    let gamma = in_state(gamma(), "waiting");
    let delta = in_state(delta(), "idle");
    written_before_noon(&transcript_of(&projects, &alpha()), 40);
    written_before_noon(&subagent_of(&projects, &alpha(), "agent-a1.jsonl"), 35);
    written_before_noon(&transcript_of(&projects, &beta()), 50);
    written_before_noon(
        &subagent_of(&projects, &beta(), "workflows/run-1/agent-b1.jsonl"),
        5,
    );
    written_before_noon(&transcript_of(&projects, &conductor()), 60);
    written_before_noon(&transcript_of(&projects, &delta), 90);
    written_before_noon(&transcript_of(&projects, &gamma), 45);
    case.sessions(&[alpha(), beta(), conductor(), delta, gamma]);
    let mut sources = case.sources(None);
    sources.projects.clone_from(&projects);
    let mut watch = Watch::new(sources.clone(), case.state());
    assert_eq!(
        pass(&mut watch),
        [
            "session silent: alpha a1a1a1a1 35m",
            "session silent: gamma e5e5e5e5 45m",
        ],
        "alpha's newest write is its sub-agent's; beta's nested sub-agent wrote 5 minutes ago"
    );
    assert_eq!(pass(&mut watch), NOTHING, "each silence once");

    written_before_noon(&subagent_of(&projects, &alpha(), "agent-a1.jsonl"), 1);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "alpha's sub-agent wrote a minute ago"
    );

    let mut later = sources;
    later.clock = one_pm;
    let mut watch = Watch::new(later, case.state());
    assert_eq!(
        pass(&mut watch),
        [
            "session silent: alpha a1a1a1a1 61m",
            "session silent: beta b2b2b2b2 65m",
        ],
        "an hour later alpha has fallen silent anew and beta's sub-agent too; gamma's silence \
         was reported already, however long it lasts"
    );
    assert_eq!(pass(&mut watch), NOTHING);
}

/// The silence is the instance's `thresholds.silent_session`: under 40 minutes alpha's 35 are
/// not reported, gamma's 45 are.
#[test]
fn the_silence_threshold_is_the_instances() {
    let case = Case::new("silent-threshold");
    let projects = case.dir.join("projects");
    let gamma = in_state(gamma(), "waiting");
    written_before_noon(&transcript_of(&projects, &alpha()), 35);
    written_before_noon(&transcript_of(&projects, &gamma), 45);
    case.sessions(&[alpha(), gamma]);
    let mut sources = case.sources(None);
    sources.projects = projects;
    let mut watch = Watch::new(sources, case.state()).with_settings(Settings {
        silent_session: Duration::from_secs(40 * 60),
        ..Settings::default()
    });
    assert_eq!(pass(&mut watch), ["session silent: gamma e5e5e5e5 45m"]);
}

/// A session without a transcript is not reported: nothing says how long it has been silent.
#[test]
fn a_controller_without_a_transcript_is_not_reported() {
    let case = Case::new("silent-none");
    case.sessions(&[alpha(), in_state(gamma(), "waiting")]);
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);
}
// ---------------------------------------------------------------------------------------------

/// A config of one instance, `acme`, holding `extra` as further keys of the instance.
fn acme_config(extra: &str) -> String {
    acme_config_in(Path::new("/srv/acme"), Path::new("/srv/acme-trees"), extra)
}

/// [`acme_config`] with its checkouts root `root` and its managed trees `trees`.
fn acme_config_in(root: &Path, trees: &Path, extra: &str) -> String {
    format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: acme\n\
         \x20   sources: [{{github: acme}}]\n\
         \x20   checkouts: {{root: {}, trees: {}}}\n\
         {extra}",
        root.display(),
        trees.display()
    )
}

fn acme(extra: &str) -> conductor_model::config::Instance {
    let parsed = config::parse(&acme_config(extra), Path::new("/fixture-home"))
        .unwrap_or_else(|problems| panic!("the fixture config parses: {problems:?}"));
    parsed.instances.into_iter().next().expect("one instance")
}

#[test]
fn the_built_in_instance_keeps_100g_free_and_wakes_from_110g() {
    let built_in = config::built_in(Path::new("/fixture-home"), Path::new("/work"));
    assert_eq!(built_in.thresholds.disk_low, Gibibytes(100));
    assert_eq!(built_in.thresholds.disk_clear, Gibibytes(110));
    assert_eq!(built_in.thresholds.context_handover, Tokens(300_000));
    assert_eq!(
        built_in.thresholds.silent_session,
        conductor_model::primitives::Duration("PT30M".to_owned()),
        "a controller silent for 30 minutes is reported"
    );
    assert_eq!(
        Settings::default(),
        Settings {
            instance: built_in.name.0.clone(),
            context_handover: 300_000,
            disk_low: 100,
            disk_clear: 110,
            disk_admit: 20,
            gate_stop: 15,
            gate_resume: 17,
            silent_session: Duration::from_secs(30 * 60),
            conductor: Some("conductor".to_owned()),
            max_open_reviews: 3,
        },
        "a watch without settings runs on the built-in instance's"
    );
    assert_eq!(Settings::of(&built_in), Settings::default());
}

#[test]
fn the_settings_are_the_instances_name_and_thresholds() {
    let instance = acme(
        "\x20   thresholds: {context_handover: 150k, disk_low: 40G, disk_clear: 45G, \
         disk_admit: 30G, gate_stop: 10G, gate_resume: 12G, silent_session: 2h}\n\
         \x20   controllers: {max_open_reviews: 2}\n",
    );
    assert_eq!(
        Settings::of(&instance),
        Settings {
            instance: "acme".to_owned(),
            context_handover: 150_000,
            disk_low: 40,
            disk_clear: 45,
            disk_admit: 30,
            gate_stop: 10,
            gate_resume: 12,
            silent_session: Duration::from_secs(2 * 3600),
            conductor: Some("conductor".to_owned()),
            max_open_reviews: 2,
        }
    );
    assert_eq!(
        Settings::of(&acme("\x20   session_prefix: acme\n")).conductor,
        Some("acme-conductor".to_owned()),
        "the conductor role's session name carries the instance's session_prefix"
    );
}

#[test]
fn the_intervals_are_the_instances_cadence_and_a_flag_overrides_each() {
    let args = |every: Option<u64>, ci_every: Option<u64>| WatchRunArgs {
        every,
        ci_every,
        follow: None,
    };
    let instance = acme("\x20   cadence: {watch: 2m, ci: 1h}\n");
    assert_eq!(
        Options::of(&instance, &args(None, None)),
        Options {
            every: Duration::from_secs(120),
            ci_every: Duration::from_secs(3600),
            follow: false,
        }
    );
    assert_eq!(
        Options::of(&instance, &args(Some(5), None)),
        Options {
            every: Duration::from_secs(5),
            ci_every: Duration::from_secs(3600),
            follow: false,
        }
    );
    assert_eq!(
        Options::of(&instance, &args(None, Some(7))).ci_every,
        Duration::from_secs(7)
    );
    let built_in = config::built_in(Path::new("/fixture-home"), Path::new("/work"));
    assert_eq!(
        Options::of(&built_in, &args(None, None)),
        Options::default(),
        "the built-in cadence is the watch's default, 180 s and 900 s"
    );
    assert_eq!(Options::default().every, Duration::from_secs(180));
    assert_eq!(Options::default().ci_every, Duration::from_secs(900));
}

#[test]
fn the_usage_limit_notification_names_the_instance() {
    let case = Case::new("limit-instance");
    case.sessions(&[alpha()]);
    let mut watch = case.watch(Some("recent")).with_settings(Settings {
        instance: "acme".to_owned(),
        ..Settings::default()
    });
    assert_eq!(
        pass(&mut watch),
        [format!("usage limit: {LIMIT}; 1 sessions: alpha")]
    );
    assert_eq!(
        case.notified().lines().next(),
        Some("acme: usage limit"),
        "{}",
        case.notified()
    );
}

// ---------------------------------------------------------------------------------------------
// Main's CI
// ---------------------------------------------------------------------------------------------

fn run(id: u64, workflow: &str, status: &str, conclusion: &str, at: &str) -> Value {
    json!({"databaseId": id, "workflowName": workflow, "status": status,
           "conclusion": conclusion, "createdAt": at})
}

#[test]
fn main_ci_turning_red_and_green_again_is_reported_on_a_ci_pass() {
    let case = Case::new("ci");
    let mut sources = case.sources(None);
    sources.repositories = vec!["alpha".to_owned(), "beta".to_owned()];
    let mut watch = Watch::new(sources, case.state());
    case.runs(
        "alpha",
        &json!([
            run(102, "CI", "in_progress", "", "2026-10-07T11:00:00Z"),
            run(101, "CI", "completed", "success", "2026-10-07T10:00:00Z"),
            run(
                90,
                "Release",
                "completed",
                "failure",
                "2026-10-06T10:00:00Z"
            ),
        ]),
    );
    let (lines, notes) = passed(&mut watch, true);
    assert_eq!(lines, NOTHING, "the first CI pass has nothing to compare");
    assert!(
        notes.contains("beta"),
        "a repository whose run list fails is named on standard error: {notes:?}"
    );
    assert_eq!(
        case.kept("ci.state"),
        "alpha|CI|101|success|2026-10-07T10:00:00Z\n\
         alpha|Release|90|failure|2026-10-06T10:00:00Z\n"
    );

    case.runs(
        "alpha",
        &json!([
            run(102, "CI", "completed", "failure", "2026-10-07T11:00:00Z"),
            run(101, "CI", "completed", "success", "2026-10-07T10:00:00Z"),
            run(
                90,
                "Release",
                "completed",
                "failure",
                "2026-10-06T10:00:00Z"
            ),
        ]),
    );
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a pass without CI does not read it"
    );
    assert_eq!(
        passed(&mut watch, true).0,
        ["CI red on main: alpha / CI (run 102)"]
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING);

    case.runs(
        "alpha",
        &json!([run(
            100,
            "CI",
            "completed",
            "failure",
            "2026-10-07T09:00:00Z"
        )]),
    );
    assert_eq!(
        passed(&mut watch, true).0,
        NOTHING,
        "a run older than the one kept changes nothing"
    );
    assert!(
        case.kept("ci.state").contains("alpha|CI|102|failure|"),
        "the newer run is kept: {}",
        case.kept("ci.state")
    );
    assert!(
        case.kept("ci.state").contains("alpha|Release|90|failure|"),
        "a workflow missing from the list keeps its last state: {}",
        case.kept("ci.state")
    );

    case.runs(
        "alpha",
        &json!([run(
            103,
            "CI",
            "completed",
            "success",
            "2026-10-07T11:30:00Z"
        )]),
    );
    assert_eq!(
        passed(&mut watch, true).0,
        ["CI green again on main: alpha / CI"]
    );
}

/// One pipeline, as GitLab's `pipelines` answers it.
fn pipeline(id: u64, status: &str, at: &str) -> Value {
    json!({"id": id, "status": status, "ref": "main", "created_at": at})
}

impl Case {
    /// A fake `glab` in the case's directory, its words logged to `glab.log`: it answers the
    /// group `example-org`'s projects with `alpha` (id 101, default branch `main`) and the
    /// archived `beta` (id 102), and alpha's pipelines with `pipelines.json`, which
    /// [`Case::pipelines`] replaces; anything else fails. Answers the forge that reads it.
    fn gitlab(&self) -> Forge {
        replace(
            &self.dir.join("projects.json"),
            &json!([
                {"id": 101, "path_with_namespace": "example-org/alpha", "archived": false,
                 "visibility": "private", "default_branch": "main", "open_issues_count": 0,
                 "web_url": "https://gitlab.example.com/example-org/alpha"},
                {"id": 102, "path_with_namespace": "example-org/beta", "archived": true,
                 "visibility": "private", "default_branch": "main", "open_issues_count": 0,
                 "web_url": "https://gitlab.example.com/example-org/beta"},
            ])
            .to_string(),
        );
        let dir = self.dir.display();
        let glab = self.dir.join("glab");
        fs::write(
            &glab,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{dir}/glab.log'\n\
                 for last in \"$@\"; do :; done\n\
                 case \"$last\" in\n\
                 \x20 groups/example-org/projects\\?*) exec cat '{dir}/projects.json' ;;\n\
                 \x20 projects/101/pipelines\\?*) exec cat '{dir}/pipelines.json' ;;\n\
                 esac\n\
                 exit 1\n"
            ),
        )
        .expect("write the fake glab");
        fs::set_permissions(&glab, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        Forge {
            glab: vec![glab.display().to_string()],
            host: "gitlab.example.com".to_owned(),
            group: "example-org".to_owned(),
            exclude: Vec::new(),
            bound: Duration::from_secs(10),
        }
    }

    /// Replaces alpha's pipelines, newest first.
    fn pipelines(&self, pipelines: &Value) {
        replace(&self.dir.join("pipelines.json"), &pipelines.to_string());
    }
}

/// `story:watch-gitlab-ci`: for the repositories of a `gitlab` source, the CI pass reads the
/// newest default-branch pipeline that decided something with `glab api --hostname <host>`, maps
/// its status by the `gitlab` collector's mapping, and prints the red line when it turns from
/// `success` to `failed` and the green line when it turns back; `gh`'s run list is not read for
/// them.
#[test]
fn a_gitlab_default_branch_turning_red_and_green_again_is_reported() {
    // The case's directory names neither the forge nor the repository, which a note would show.
    let case = Case::new("ci-pipelines");
    let forge = case.gitlab();
    let mut sources = case.sources(None);
    sources.organization = String::new();
    sources.repositories = vec!["alpha".to_owned()];
    let mut watch = Watch::new(sources, case.state()).with_gitlab(vec![forge]);

    case.pipelines(&json!([pipeline(601, "success", "2026-10-07T10:00:00Z")]));
    let (lines, notes) = passed(&mut watch, true);
    assert_eq!(lines, NOTHING, "the first CI pass has nothing to compare");
    assert!(
        !notes.contains("alpha") && !notes.contains("gitlab"),
        "the forge and alpha's pipelines answered: {notes:?}"
    );
    assert_eq!(
        case.kept("ci.state"),
        "alpha|pipeline|601|success|2026-10-07T10:00:00Z\n"
    );

    case.pipelines(&json!([
        pipeline(603, "running", "2026-10-07T11:30:00Z"),
        pipeline(602, "failed", "2026-10-07T11:00:00Z"),
        pipeline(601, "success", "2026-10-07T10:00:00Z"),
    ]));
    assert_eq!(
        passed(&mut watch, true).0,
        ["CI red on main: alpha / pipeline (run 602)"]
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING, "reported once");

    case.pipelines(&json!([
        pipeline(604, "success", "2026-10-07T11:45:00Z"),
        pipeline(603, "canceled", "2026-10-07T11:30:00Z"),
        pipeline(602, "failed", "2026-10-07T11:00:00Z"),
    ]));
    assert_eq!(
        passed(&mut watch, true).0,
        ["CI green again on main: alpha / pipeline"]
    );

    let asked = fs::read_to_string(case.dir.join("glab.log")).expect("glab was asked");
    for line in asked.lines() {
        assert!(
            line.starts_with("api --hostname gitlab.example.com "),
            "{line}"
        );
    }
    assert!(
        asked.contains(
            "groups/example-org/projects?include_subgroups=true&with_shared=false&per_page=100&\
             page=1"
        ),
        "{asked}"
    );
    assert!(
        asked.contains("projects/101/pipelines?ref=main&per_page=20"),
        "{asked}"
    );
}

#[test]
fn a_run_that_decides_nothing_keeps_the_last_run() {
    let case = Case::new("ci-undecided");
    let mut sources = case.sources(None);
    sources.repositories = vec!["alpha".to_owned()];
    let mut watch = Watch::new(sources, case.state());
    let red = run(102, "CI", "completed", "failure", "2026-10-07T10:00:00Z");
    case.runs("alpha", &json!([red]));
    assert_eq!(
        passed(&mut watch, true).0,
        NOTHING,
        "the first CI pass seeds"
    );
    let kept = case.kept("ci.state");

    for (id, conclusion) in [
        (103, "skipped"),
        (104, ""),
        (106, "cancelled"),
        (107, "neutral"),
        (108, "action_required"),
        (109, "stale"),
        (110, "a_conclusion_github_adds"),
    ] {
        case.runs(
            "alpha",
            &json!([
                run(id, "CI", "completed", conclusion, "2026-10-07T11:00:00Z"),
                red
            ]),
        );
        assert_eq!(passed(&mut watch, true).0, NOTHING, "{conclusion:?}");
        assert_eq!(case.kept("ci.state"), kept, "{conclusion:?} keeps run 102");
    }

    case.runs(
        "alpha",
        &json!([run(
            105,
            "CI",
            "completed",
            "success",
            "2026-10-07T11:30:00Z"
        )]),
    );
    assert_eq!(
        passed(&mut watch, true).0,
        ["CI green again on main: alpha / CI"]
    );
}

/// The lines of one CI pass per run of `sequence` (run id, conclusion, `createdAt`), each the
/// only completed run of alpha's list, from no kept state.
fn ci_sequence(case: &Case, sequence: &[(u64, &str, &str)]) -> Vec<String> {
    let mut sources = case.sources(None);
    sources.repositories = vec!["alpha".to_owned()];
    let mut watch = Watch::new(sources, case.state());
    let mut lines = Vec::new();
    for (id, conclusion, at) in sequence {
        case.runs(
            "alpha",
            &json!([run(*id, "CI", "completed", conclusion, at)]),
        );
        lines.extend(passed(&mut watch, true).0);
    }
    lines
}

#[test]
fn a_timed_out_run_after_a_failure_stays_red_and_success_turns_it_green_again() {
    let case = Case::new("ci-timed-out");
    let lines = ci_sequence(
        &case,
        &[
            (101, "success", "2026-10-07T10:00:00Z"),
            (102, "failure", "2026-10-07T10:10:00Z"),
            (103, "timed_out", "2026-10-07T10:20:00Z"),
            (104, "success", "2026-10-07T10:30:00Z"),
        ],
    );
    assert_eq!(
        lines,
        [
            "CI red on main: alpha / CI (run 102)",
            "CI green again on main: alpha / CI"
        ]
    );
}

#[test]
fn a_neutral_run_after_success_decides_nothing_and_a_failure_turns_it_red() {
    let case = Case::new("ci-neutral");
    let lines = ci_sequence(
        &case,
        &[
            (101, "success", "2026-10-07T10:00:00Z"),
            (102, "neutral", "2026-10-07T10:10:00Z"),
            (103, "failure", "2026-10-07T10:20:00Z"),
        ],
    );
    assert_eq!(lines, ["CI red on main: alpha / CI (run 103)"]);
}

#[test]
fn each_red_conclusion_turns_a_kept_green_red_and_success_turns_it_green_again() {
    for conclusion in ["failure", "timed_out", "startup_failure"] {
        let case = Case::new(&format!("ci-red-{conclusion}"));
        let lines = ci_sequence(
            &case,
            &[
                (101, "success", "2026-10-07T10:00:00Z"),
                (102, conclusion, "2026-10-07T10:10:00Z"),
                (103, "success", "2026-10-07T10:20:00Z"),
            ],
        );
        assert_eq!(
            lines,
            [
                "CI red on main: alpha / CI (run 102)",
                "CI green again on main: alpha / CI"
            ],
            "{conclusion}"
        );
        assert!(
            case.kept("ci.state").contains("alpha|CI|103|success|"),
            "{conclusion}: {}",
            case.kept("ci.state")
        );
    }
}

#[test]
fn a_repository_whose_run_list_fails_is_named_once_and_the_others_are_read() {
    let case = Case::new("ci-unread");
    case.sessions(&[alpha()]);
    let mut sources = case.sources(None);
    sources.repositories = vec!["alpha".to_owned(), "beta".to_owned()];
    let mut watch = Watch::new(sources, case.state());
    case.runs(
        "alpha",
        &json!([run(
            101,
            "CI",
            "completed",
            "success",
            "2026-10-07T10:00:00Z"
        )]),
    );
    let (_, notes) = passed(&mut watch, true);
    assert_eq!(notes.lines().count(), 1, "{notes:?}");
    assert!(
        notes.starts_with("conductor watch: main CI of beta not read: `cat "),
        "{notes:?}"
    );
    assert_eq!(
        case.kept("ci.state"),
        "alpha|CI|101|success|2026-10-07T10:00:00Z\n"
    );

    case.runs(
        "alpha",
        &json!([run(
            102,
            "CI",
            "completed",
            "failure",
            "2026-10-07T11:00:00Z"
        )]),
    );
    let (lines, notes) = passed(&mut watch, true);
    assert_eq!(lines, ["CI red on main: alpha / CI (run 102)"]);
    assert_eq!(notes, "", "beta is named once until it answers again");

    case.runs("beta", &json!([]));
    assert_eq!(passed(&mut watch, true).1, "");
    fs::remove_file(case.dir.join("runs/beta.json")).expect("remove beta's run list");
    assert_eq!(
        passed(&mut watch, true).1.lines().count(),
        1,
        "named again once it answered in between"
    );
}

/// One repository's state in a snapshot, with `real_commits_7d` real commits in 7 days.
fn observation(name: &str, real_commits_7d: i64, archived: bool) -> RecordRepository {
    RecordRepository {
        snapshot_id: SnapshotId(Uuid("00000000-0000-0000-0000-000000000000".to_owned())),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Private,
        archived,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: real_commits_7d + 2,
        real_commits_7d,
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
        gates_hooks_version: None,
        gates_version: None,
        wave_width: None,
        wave_ready: None,
        wave_reason: None,
    }
}

/// Takes a snapshot into the store under the state directory `state`, through the library, over
/// a fake collector that records each `(repository, real_commits_7d, archived)` of `observed`;
/// `answers: false` makes the collector fail, and the snapshot with it.
fn take_snapshot(state: &Path, observed: &[(&str, i64, bool)], answers: bool) -> SnapshotState {
    let inputs: Vec<RecordRepository> = observed
        .iter()
        .map(|&(name, real, archived)| observation(name, real, archived))
        .collect();
    let collector = Collector::new("repositories", move |recorder| {
        for input in &inputs {
            recorder.repository(input.clone())?;
        }
        if !answers {
            anyhow::bail!("the fixture source does not answer");
        }
        Ok(())
    });
    let store = store::open(state).expect("open the case's store");
    snapshot::take(store, &[collector])
        .expect("the driver ends the snapshot")
        .state
}

/// alpha and beta are Active in it; gamma has no real commit and delta is archived.
const OBSERVED: [(&str, i64, bool); 4] = [
    ("alpha", 3, false),
    ("beta", 1, false),
    ("gamma", 0, false),
    ("delta", 5, true),
];

/// Sources whose run list records the `{organization}/{repository}` it is asked about, one per
/// line in `calls`, and answers an empty list; their repositories are the Active ones of the
/// store under `state`.
fn recording_runs(case: &Case, state: &Path, calls: &Path) -> Sources {
    let mut sources = case.sources(None);
    sources.store = Some(state.to_owned());
    sources.repositories = vec!["never-read".to_owned()];
    sources.runs = vec![
        "sh".to_owned(),
        "-c".to_owned(),
        "printf '%s\\n' \"$1\" >> \"$0\"; printf '[]\\n'".to_owned(),
        calls.display().to_string(),
        "{organization}/{repository}".to_owned(),
    ];
    sources
}

/// The lines of `file`, sorted; none when it is not there.
fn sorted_lines(file: &Path) -> Vec<String> {
    let mut lines: Vec<String> = fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

/// W4 decision 1: the repositories whose `main` CI is read are the Active ones of the newest
/// complete snapshot, where they were a fixed list of 30.
#[test]
fn a_ci_pass_asks_about_exactly_the_active_repositories_of_the_newest_complete_snapshot() {
    let case = Case::new("ci-active");
    case.sessions(&[]);
    let state = case.dir.join("state");
    assert_eq!(
        take_snapshot(&state, &OBSERVED, true),
        SnapshotState::Complete
    );
    let calls = case.dir.join("calls.txt");
    let mut watch = Watch::new(recording_runs(&case, &state, &calls), case.state());
    let (lines, notes) = passed(&mut watch, true);
    assert_eq!(lines, NOTHING);
    assert_eq!(notes, "");
    assert_eq!(
        sorted_lines(&calls),
        ["fixture-org/alpha", "fixture-org/beta"]
    );
}

#[test]
fn without_a_complete_snapshot_no_main_ci_is_read_and_that_is_said_once() {
    let case = Case::new("ci-no-snapshot");
    case.sessions(&[]);
    let state = case.dir.join("state");
    let calls = case.dir.join("calls.txt");
    let mut watch = Watch::new(recording_runs(&case, &state, &calls), case.state());
    let (lines, notes) = passed(&mut watch, true);
    assert_eq!(lines, NOTHING);
    assert_eq!(notes.lines().count(), 1, "{notes:?}");
    assert!(
        notes.starts_with("conductor watch: no snapshot yet"),
        "{notes:?}"
    );
    assert_eq!(passed(&mut watch, true).1, "", "said once");

    assert_eq!(
        take_snapshot(&state, &OBSERVED, false),
        SnapshotState::Failed
    );
    assert_eq!(passed(&mut watch, true), (Vec::new(), String::new()));
    assert_eq!(
        sorted_lines(&calls),
        Vec::<String>::new(),
        "no run list is asked for without a complete snapshot"
    );

    assert_eq!(
        take_snapshot(&state, &OBSERVED[..1], true),
        SnapshotState::Complete
    );
    assert_eq!(passed(&mut watch, true), (Vec::new(), String::new()));
    assert_eq!(sorted_lines(&calls), ["fixture-org/alpha"]);
}

// ---------------------------------------------------------------------------------------------
// Review (story:review-limit)
// ---------------------------------------------------------------------------------------------

/// Takes a complete snapshot started at `started_at` into the store under `state`, in which the
/// Active `alpha` holds `waiting` pull requests that wait for review, and one draft and one with a
/// failing check that do not.
fn take_reviews(state: &Path, started_at: &str, waiting: i64) {
    let collector = Collector::new("reviews", move |recorder| {
        let snapshot = recorder.snapshot().clone();
        recorder.repository(RecordRepository {
            snapshot_id: snapshot.clone(),
            ..observation("alpha", 3, false)
        })?;
        for (number, draft, failing_checks) in (1..=waiting)
            .map(|number| (number, false, 0))
            .chain([(waiting + 1, true, 0), (waiting + 2, false, 1)])
        {
            recorder.pull_request(RecordPullRequest {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName("alpha".to_owned()),
                number,
                title: format!("Change {number}"),
                draft,
                mergeability: Mergeability::Mergeable,
                failing_checks,
                failing_check_names: Vec::new(),
                opened_at: Timestamp(format!("2026-10-0{}T09:00:00Z", 1 + number % 6)),
                updated_at: Timestamp("2026-10-07T09:00:00Z".to_owned()),
                url: None,
            })?;
        }
        Ok(())
    });
    let store = store::open(state).expect("open the case's store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let taken = snapshot::take_from(store, start, &[collector]).expect("the snapshot ends");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
}

/// A CI pass reads the newest complete snapshot: `alpha` reaching its limit of 2 prints
/// `review full: alpha 2/2` once; the next snapshot, at 1/2, prints `review open: alpha` once.
#[test]
fn a_repository_reaching_its_review_limit_is_reported_once_and_once_more_under_it() {
    let case = Case::new("review-limit");
    case.sessions(&[]);
    let state = case.dir.join("state");
    let calls = case.dir.join("calls.txt");
    let mut watch =
        Watch::new(recording_runs(&case, &state, &calls), case.state()).with_settings(Settings {
            max_open_reviews: 2,
            ..Settings::default()
        });

    take_reviews(&state, "2026-10-07T10:00:00Z", 1);
    assert_eq!(passed(&mut watch, true), (Vec::new(), String::new()));

    take_reviews(&state, "2026-10-07T11:00:00Z", 2);
    assert_eq!(
        passed(&mut watch, true),
        (vec!["review full: alpha 2/2".to_owned()], String::new())
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING, "once");

    take_reviews(&state, "2026-10-07T11:30:00Z", 3);
    assert_eq!(
        passed(&mut watch, true).0,
        NOTHING,
        "still full: no new line"
    );

    take_reviews(&state, "2026-10-07T12:00:00Z", 1);
    assert_eq!(
        passed(&mut watch, true).0,
        ["review open: alpha"],
        "under the limit again"
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING, "once");
}

/// With `max_open_reviews: 0` there is no limit, and no review line however many wait.
#[test]
fn without_a_review_limit_no_review_line_is_printed() {
    let case = Case::new("review-no-limit");
    case.sessions(&[]);
    let state = case.dir.join("state");
    let calls = case.dir.join("calls.txt");
    let mut watch =
        Watch::new(recording_runs(&case, &state, &calls), case.state()).with_settings(Settings {
            max_open_reviews: 0,
            ..Settings::default()
        });
    take_reviews(&state, "2026-10-07T10:00:00Z", 5);
    assert_eq!(passed(&mut watch, true), (Vec::new(), String::new()));
}

// ---------------------------------------------------------------------------------------------
// Gates hooks (story:gates-hooks-fresh)
// ---------------------------------------------------------------------------------------------

/// Takes a complete snapshot started at `started_at` into the store under `state`, in which each
/// `(repository, hook version)` of `hooks` was observed with `b10x-gates` 0.1.17 on `PATH`.
fn take_hooks(state: &Path, started_at: &str, hooks: &[(&'static str, Option<&'static str>)]) {
    let hooks = hooks.to_vec();
    let collector = Collector::new("repositories", move |recorder| {
        let snapshot = recorder.snapshot().clone();
        for (name, hook) in &hooks {
            recorder.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                gates_hooks_version: hook.map(str::to_owned),
                gates_version: Some("0.1.17".to_owned()),
                ..observation(name, 1, false)
            })?;
        }
        Ok(())
    });
    let store = store::open(state).expect("open the case's store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let taken = snapshot::take_from(store, start, &[collector]).expect("the snapshot ends");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
}

/// A pass prints `gates hooks stale: <n> checkouts` once per new complete snapshot that finds a
/// checkout whose hook differs from the `b10x-gates` on `PATH`; a checkout without a hook is not
/// counted, and a snapshot that finds none prints nothing.
#[test]
fn a_new_snapshot_finding_stale_gates_hooks_is_reported_once() {
    let case = Case::new("gates-hooks");
    case.sessions(&[]);
    let state = case.dir.join("state");
    let calls = case.dir.join("calls.txt");
    let mut watch = Watch::new(recording_runs(&case, &state, &calls), case.state());

    take_hooks(
        &state,
        "2026-10-07T10:00:00Z",
        &[
            ("alpha", Some("0.1.14")),
            ("beta", Some("0.1.17")),
            ("gamma", None),
        ],
    );
    assert_eq!(
        passed(&mut watch, true),
        (
            vec!["gates hooks stale: 1 checkouts".to_owned()],
            String::new()
        )
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING, "once per snapshot");
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "a pass without CI reads no snapshot"
    );

    take_hooks(
        &state,
        "2026-10-07T11:00:00Z",
        &[("alpha", Some("0.1.14")), ("beta", Some("0.1.15"))],
    );
    assert_eq!(
        passed(&mut watch, true).0,
        ["gates hooks stale: 2 checkouts"],
        "a new snapshot that finds stale hooks"
    );
    assert_eq!(passed(&mut watch, true).0, NOTHING, "once");

    take_hooks(
        &state,
        "2026-10-07T12:00:00Z",
        &[("alpha", Some("0.1.17")), ("beta", Some("0.1.17"))],
    );
    assert_eq!(
        passed(&mut watch, true).0,
        NOTHING,
        "a snapshot that finds none"
    );
}

// ---------------------------------------------------------------------------------------------
// Disk
// ---------------------------------------------------------------------------------------------

/// W4 decision 5, keep 100G free: the built-in `disk_low` is 100G and
/// `disk_clear` 110G, where they were 15G and 18G.
#[test]
fn free_disk_under_100g_is_reported_once_per_fall() {
    let case = Case::new("disk");
    let mut watch = case.watch(None);
    let mut at = |gib: u64| {
        case.disk(gib * GIB);
        pass(&mut watch)
    };
    assert_eq!(at(120), NOTHING);
    assert_eq!(at(100), NOTHING, "100G free is not under 100G");
    assert_eq!(at(99), ["disk low: 99G free on /"]);
    assert_eq!(at(98), NOTHING, "once per fall");
    assert_eq!(at(109), NOTHING);
    assert_eq!(at(98), NOTHING, "not rearmed under 110G");
    assert_eq!(at(110), NOTHING);
    assert_eq!(at(99), ["disk low: 99G free on /"], "rearmed at 110G");
}

#[test]
fn free_disk_is_reported_under_the_instances_disk_low_and_rearmed_at_its_disk_clear() {
    let case = Case::new("disk-instance");
    let mut watch = case.watch(None).with_settings(Settings {
        disk_low: 40,
        disk_clear: 45,
        ..Settings::default()
    });
    let mut at = |gib: u64| {
        case.disk(gib * GIB);
        pass(&mut watch)
    };
    assert_eq!(
        at(50),
        NOTHING,
        "50G free is low only under the built-in 100G"
    );
    assert_eq!(at(39), ["disk low: 39G free on /"]);
    assert_eq!(at(44), NOTHING, "not rearmed under 45G");
    assert_eq!(at(39), NOTHING);
    assert_eq!(at(45), NOTHING);
    assert_eq!(at(39), ["disk low: 39G free on /"], "rearmed at 45G");
}

/// `story:watch-disk-floors`: each floor is reported once per fall, `disk_low` (100G) rearmed at
/// `disk_clear` (110G), `disk_admit` (20G) at itself and `gate_stop` (15G) at `gate_resume`
/// (17G). Each reading is written twice, so a jump of more than 20G is read again as itself.
#[test]
fn each_disk_floor_is_reported_once_per_fall_low_admit_and_gate_stop() {
    let case = Case::new("disk-floors");
    let mut watch = case.watch(None);
    let mut at = |gib: u64| {
        case.readings(&[gib * GIB, gib * GIB]);
        pass(&mut watch)
    };
    assert_eq!(at(120), NOTHING);
    assert_eq!(at(90), ["disk low: 90G free on /"]);
    assert_eq!(at(18), ["disk admit: 18G free on / (builds wait)"]);
    assert_eq!(at(14), ["disk gate stop: 14G free on / (gates stop)"]);
    assert_eq!(at(14), NOTHING, "once per fall");
    assert_eq!(at(16), NOTHING, "not rearmed under gate_resume, 17G");
    assert_eq!(
        at(18),
        NOTHING,
        "gate stop rearmed at 17G; admit not at 18G"
    );
    assert_eq!(
        at(14),
        ["disk gate stop: 14G free on / (gates stop)"],
        "rearmed at 17G"
    );
}

/// A fake reclaim run under the state directory `state`: appends the bytes it was asked for to
/// `state/reclaim.calls`, and reaches them, freeing 7G, when `state/reclaim.reached` is there;
/// else it frees 1G and leaves 6G.
fn fake_reclaim(state: &Path, need: u64) -> anyhow::Result<ReclaimReport> {
    let calls = state.join("reclaim.calls");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&calls)?;
    writeln!(log, "{need}")?;
    let reached = state.join("reclaim.reached").exists();
    Ok(ReclaimReport {
        reclaimed_bytes: if reached { 7 * GIB } else { GIB },
        steps: Vec::new(),
        left: if reached {
            Vec::new()
        } else {
            vec![(format!("{TREES}/alpha/busy"), 6 * GIB)]
        },
        reached,
    })
}

/// A reclaim run that fails.
fn failing_reclaim(_: &Path, _: u64) -> anyhow::Result<ReclaimReport> {
    anyhow::bail!("the store did not open")
}

impl Case {
    /// The directory the fake reclaim keeps its calls in, created.
    fn reclaim_dir(&self) -> PathBuf {
        let dir = self.dir.join("reclaim");
        fs::create_dir_all(&dir).expect("create the reclaim directory");
        dir
    }

    /// The bytes each reclaim run was asked for, in order.
    fn reclaims(&self) -> Vec<u64> {
        fs::read_to_string(self.dir.join("reclaim/reclaim.calls"))
            .unwrap_or_default()
            .lines()
            .map(|line| line.parse().expect("a count of bytes"))
            .collect()
    }
}

/// `story:disk-reclaim`, `story:disk-escalation`: the watch runs reclaim once per fall under
/// `disk_admit` (20G), not on every pass, to `disk_admit` plus the margin; it reports `disk floor
/// not reached: <n>G short` once per fall, by the reading after the run, and `disk reclaimed:
/// <n>G` when the run reached the floor.
#[test]
fn reclaim_runs_once_per_fall_under_disk_admit_and_reports_its_shortfall_once() {
    let case = Case::new("disk-reclaim");
    let state = case.reclaim_dir();
    let mut watch = case.watch(None).with_reclaim(state.clone(), fake_reclaim);
    let mut at = |readings: &[u64]| {
        let bytes: Vec<u64> = readings.iter().map(|gib| gib * GIB).collect();
        case.readings(&bytes);
        pass(&mut watch)
    };
    let floor = floor_bytes(20);
    assert!(floor > 20 * GIB, "the floor is over disk_admit");
    let short = (floor - 19 * GIB).div_ceil(GIB);

    assert_eq!(at(&[30]), ["disk low: 30G free on /"]);
    assert_eq!(
        case.reclaims(),
        Vec::<u64>::new(),
        "no reclaim over disk_admit"
    );

    // The fall under disk_admit: one run, and the shortfall by the reading after it, 19G.
    assert_eq!(
        at(&[18, 19]),
        [
            "disk admit: 18G free on / (builds wait)".to_owned(),
            format!("disk floor not reached: {short}G short"),
        ]
    );
    assert_eq!(case.reclaims(), [floor]);
    assert_eq!(at(&[18]), NOTHING, "once per fall");
    assert_eq!(at(&[19]), NOTHING);
    assert_eq!(
        case.reclaims(),
        [floor],
        "not on every pass under the floor"
    );

    // Back at disk_admit, then a new fall: one more run, which reaches the floor.
    assert_eq!(at(&[20]), NOTHING);
    fs::write(state.join("reclaim.reached"), "").expect("write the marker");
    assert_eq!(
        at(&[17]),
        [
            "disk admit: 17G free on / (builds wait)",
            "disk reclaimed: 7G"
        ]
    );
    assert_eq!(at(&[18]), NOTHING);
    assert_eq!(case.reclaims(), [floor, floor]);
}

/// A watch without a reclaim runs none, and a reclaim that fails is named once on standard error
/// and reports nothing; the fall is still reported.
#[test]
fn a_failing_reclaim_is_named_on_standard_error_and_the_fall_still_reported() {
    let case = Case::new("disk-reclaim-fails");
    let mut watch = case
        .watch(None)
        .with_reclaim(case.reclaim_dir(), failing_reclaim);
    case.disk(18 * GIB);
    let (lines, notes) = passed(&mut watch, false);
    assert_eq!(
        lines,
        [
            "disk low: 18G free on /",
            "disk admit: 18G free on / (builds wait)"
        ]
    );
    assert!(
        notes.contains("conductor watch: reclaim disk under disk_admit: the store did not open"),
        "{notes}"
    );
}

/// The floors are the instance's: `disk_admit` rearmed at itself, `gate_stop` at `gate_resume`;
/// a fall past all three in one pass reports each, in the order low, admit, gate stop.
#[test]
fn the_disk_floors_are_the_instances_and_one_fall_reports_each_crossed() {
    let case = Case::new("disk-floors-instance");
    let mut watch = case.watch(None).with_settings(Settings {
        disk_low: 40,
        disk_clear: 45,
        disk_admit: 30,
        gate_stop: 10,
        gate_resume: 12,
        ..Settings::default()
    });
    let mut at = |gib: u64| {
        case.readings(&[gib * GIB, gib * GIB]);
        pass(&mut watch)
    };
    assert_eq!(at(50), NOTHING);
    assert_eq!(
        at(9),
        [
            "disk low: 9G free on /",
            "disk admit: 9G free on / (builds wait)",
            "disk gate stop: 9G free on / (gates stop)",
        ]
    );
    assert_eq!(
        at(29),
        NOTHING,
        "admit not rearmed under 30G; gate stop at 12G"
    );
    assert_eq!(at(30), NOTHING);
    assert_eq!(at(29), ["disk admit: 29G free on / (builds wait)"]);
    assert_eq!(at(11), NOTHING, "11G is over the 10G gate stop");
    assert_eq!(at(9), ["disk gate stop: 9G free on / (gates stop)"]);
}

/// `story:watch-disk-reading`: a reading more than 20G from the watch's previous one is read
/// again before it is reported, and the second reading stands.
#[test]
fn a_reading_more_than_20g_from_the_previous_one_is_read_again_before_it_is_reported() {
    let case = Case::new("disk-jump");
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING, "100G free is not under 100G");

    case.readings(&[2 * GIB, 100 * GIB]);
    assert_eq!(
        pass(&mut watch),
        NOTHING,
        "the second reading, 100G, stands"
    );
    assert_eq!(case.unread(), format!("{}\n", 100 * GIB), "read twice");

    case.readings(&[2 * GIB, 2 * GIB]);
    assert_eq!(
        pass(&mut watch),
        [
            "disk low: 2G free on /",
            "disk admit: 2G free on / (builds wait)",
            "disk gate stop: 2G free on / (gates stop)",
        ]
    );
}

/// A reading within 20G of the previous one, here exactly 20G under it, is read once and
/// reported as it stands.
#[test]
fn a_reading_within_20g_of_the_previous_one_is_read_once() {
    let case = Case::new("disk-step");
    let mut watch = case.watch(None);
    assert_eq!(pass(&mut watch), NOTHING);

    case.readings(&[80 * GIB, 100 * GIB]);
    assert_eq!(pass(&mut watch), ["disk low: 80G free on /"]);
    assert_eq!(case.unread(), format!("{}\n", 100 * GIB), "read once");
}

/// The previous reading is the last pass's, kept under the state directory, so a watch started
/// again after it ended on a change compares with it too.
#[test]
fn a_new_watch_compares_with_the_reading_the_last_one_kept() {
    let case = Case::new("disk-kept");
    assert_eq!(pass(&mut case.watch(None)), NOTHING);

    case.readings(&[2 * GIB, 100 * GIB]);
    assert_eq!(pass(&mut case.watch(None)), NOTHING);
}

// ---------------------------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------------------------

/// Standard output of a watch on a thread: each complete line, with the instant it was written,
/// to the test. Writing fails once the test stops listening.
struct Lines {
    sender: Sender<(Instant, String)>,
    partial: Vec<u8>,
}

impl Lines {
    fn new(sender: Sender<(Instant, String)>) -> Self {
        Self {
            sender,
            partial: Vec::new(),
        }
    }
}

impl Write for Lines {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.partial.extend_from_slice(bytes);
        while let Some(end) = self.partial.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.partial.drain(..=end).collect();
            let text = String::from_utf8_lossy(&line[..end]).into_owned();
            self.sender
                .send((Instant::now(), text))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "the test stopped"))?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Waits until the watch kept its first session list.
fn first_pass_done(case: &Case) {
    let started = Instant::now();
    while !case.state().join("sessions.json").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the first pass kept no session list"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_watch_exits_within_three_seconds_of_a_session_leaving_and_not_while_nothing_changes() {
    let case = Case::new("first-change");
    case.sessions(&[alpha(), beta()]);
    let mut watch = case.watch(None);
    let (sender, lines) = mpsc::channel();
    let running = thread::spawn(move || {
        let options = Options {
            every: Duration::from_secs(1),
            ci_every: Duration::from_secs(900),
            follow: false,
        };
        let ended = watch.run(&options, &mut Lines::new(sender), &mut io::sink());
        (ended.map_err(|error| format!("{error:#}")), Instant::now())
    });

    assert_eq!(
        lines.recv_timeout(Duration::from_secs(5)),
        Err(RecvTimeoutError::Timeout),
        "it printed while nothing changed"
    );
    assert!(!running.is_finished(), "it ended while nothing changed");
    first_pass_done(&case);

    case.sessions(&[alpha(), beta(), gamma()]);
    assert_eq!(
        lines.recv_timeout(Duration::from_millis(2500)),
        Err(RecvTimeoutError::Timeout),
        "it printed a start"
    );
    assert!(!running.is_finished(), "it ended on a start");

    let left = Instant::now();
    case.sessions(&[alpha(), gamma()]);
    let (_, line) = lines
        .recv_timeout(Duration::from_secs(3))
        .expect("a line within 3 s of beta leaving");
    assert_eq!(line, "session gone: beta b2b2b2b2");
    let (ended, at) = running.join().expect("the watch thread ends");
    assert_eq!(ended, Ok(()));
    assert!(
        at.duration_since(left) <= Duration::from_secs(3),
        "it ended {:?} after beta left",
        at.duration_since(left)
    );
    assert_eq!(
        lines.recv_timeout(Duration::from_millis(100)),
        Err(RecvTimeoutError::Disconnected),
        "it printed more than the one line"
    );
}

#[test]
fn follow_prints_each_change_and_goes_on() {
    let case = Case::new("follow");
    case.sessions(&[alpha(), beta(), gamma()]);
    let mut watch = case.watch(None);
    let (sender, lines) = mpsc::channel();
    let running = thread::spawn(move || {
        let options = Options {
            every: Duration::from_secs(1),
            ci_every: Duration::from_secs(900),
            follow: true,
        };
        watch
            .run(&options, &mut Lines::new(sender), &mut io::sink())
            .map_err(|error| format!("{error:#}"))
    });
    first_pass_done(&case);

    case.sessions(&[alpha(), gamma()]);
    let (_, first) = lines
        .recv_timeout(Duration::from_secs(3))
        .expect("a line for beta");
    assert_eq!(first, "session gone: beta b2b2b2b2");
    case.sessions(&[gamma()]);
    let (_, second) = lines
        .recv_timeout(Duration::from_secs(3))
        .expect("a line for alpha");
    assert_eq!(second, "session gone: alpha a1a1a1a1");
    assert!(!running.is_finished(), "--follow ended after a change");

    drop(lines);
    case.disk(10 * GIB);
    let ended = running.join().expect("the watch thread ends");
    assert!(
        ended.is_err(),
        "with no reader left, the next line fails the watch: {ended:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The binary
// ---------------------------------------------------------------------------------------------

/// No program answers; the free disk of `/`, which `statvfs` reads, is over the instance's 1G.
#[test]
fn the_binary_keeps_watching_without_a_line_while_no_source_answers() {
    let case = Case::new("binary");
    let home = case.dir.join("home");
    let bin = case.dir.join("empty-path");
    let work = case.dir.join("work");
    for dir in [&home, &bin, &work] {
        fs::create_dir_all(dir).expect("create the case's directories");
    }
    let config = case.config(&acme_config(
        "\x20   thresholds: {disk_low: 1G, disk_clear: 2G, disk_admit: 1G, gate_stop: 1G, \
         gate_resume: 2G}\n",
    ));
    let state = case.dir.join("state");
    let mut child = common::conductor(&home)
        .arg("--state-dir")
        .arg(&state)
        .args(["watch", "run", "--every", "1", "--ci-every", "1"])
        .current_dir(&work)
        .env("PATH", &bin)
        .env(config::CONFIG_VARIABLE, &config)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the conductor binary starts");
    thread::sleep(Duration::from_millis(3500));
    let still = child.try_wait().expect("ask after the watch");
    if still.is_none() {
        child.kill().expect("stop the watch");
    }
    let output = child
        .wait_with_output()
        .expect("collect the watch's output");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        still.is_none(),
        "the watch ended with nothing to report: exit {:?}, stderr {stderr:?}",
        output.status.code()
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "",
        "it printed a line"
    );
    assert_eq!(
        stderr.matches("start `claude`").count(),
        1,
        "a source that does not answer is named once, not on every pass: {stderr:?}"
    );
    assert!(
        stderr
            .lines()
            .all(|line| line.starts_with("conductor watch: ")),
        "{stderr:?}"
    );
    assert_eq!(
        stderr.matches("no snapshot yet").count(),
        1,
        "a CI pass every second without a snapshot says so once: {stderr:?}"
    );
    assert!(
        state.join("watch").is_dir(),
        "its state is kept under <state>/watch/"
    );
    assert!(!state.join("tree").exists(), "it wrote a store record");
    assert_eq!(
        fs::read_dir(&work).expect("read work/").count(),
        0,
        "it planted something in the working directory"
    );
}

#[test]
fn the_binary_refuses_an_interval_of_zero() {
    let case = Case::new("zero");
    for flag in ["--every", "--ci-every"] {
        let output = common::conductor(case.dir.join("home"))
            .arg("--state-dir")
            .arg(case.dir.join("state"))
            .args(["watch", "run", flag, "0"])
            .env("PATH", case.dir.join("empty-path"))
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{flag} 0: {stderr}");
        assert!(output.stdout.is_empty(), "{flag} 0");
        assert!(
            stderr.contains("invalid value '0'"),
            "{flag} 0 is refused as a value: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The binary on an instance (W4 decisions 1, 2, 5 and 6)
// ---------------------------------------------------------------------------------------------

/// Writes the fake program `name` into `bin`.
fn program(bin: &Path, name: &str, script: &str) {
    let path = bin.join(name);
    fs::write(&path, script).expect("write a fake program");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// A fake `claude` whose session list is empty and which appends a line to `calls` each time it
/// runs: once a pass.
fn fake_claude(bin: &Path, calls: &Path) {
    program(
        bin,
        "claude",
        &format!(
            "#!/bin/sh\nprintf 'claude\\n' >> '{}'\nprintf '[]\\n'\n",
            calls.display()
        ),
    );
}

/// The free GiB on `/`, rounded up as the watch rounds them, read the way the binary reads them.
fn free_gib() -> u64 {
    disk::space(Path::new("/"))
        .expect("read the free space of /")
        .available
        .div_ceil(GIB)
}

/// `stdout` is the one line `disk low: <n>G free on /`, `<n>` within 5G of `free`: the binary
/// reads `/` a moment after the case did, and other processes write to it meanwhile.
fn assert_disk_low(stdout: &str, free: u64, stderr: &str) {
    let read = stdout
        .strip_prefix("disk low: ")
        .and_then(|rest| rest.strip_suffix("G free on /\n"))
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or_else(|| panic!("not one disk-low line: {stdout:?}; {stderr}"));
    assert!(
        read.abs_diff(free) <= 5,
        "{read}G against {free}G: {stderr}"
    );
}

impl Case {
    /// The binary's home directory `home/`, working directory `work/` and `bin/`, the only
    /// directory on its `PATH`; each created.
    fn binary_dirs(&self) -> (PathBuf, PathBuf, PathBuf) {
        let dirs = (
            self.dir.join("home"),
            self.dir.join("work"),
            self.dir.join("bin"),
        );
        for dir in [&dirs.0, &dirs.1, &dirs.2] {
            fs::create_dir_all(dir).expect("create the case's directories");
        }
        dirs
    }

    /// Writes `text` as the case's config file, `conductor.yaml`.
    fn config(&self, text: &str) -> PathBuf {
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the case's config file");
        file
    }

    /// Starts `conductor --state-dir <case>/state watch run <args>` from `work/`, with `HOME`
    /// naming `home/` and `PATH` naming `bin/`. `CONDUCTOR_CONFIG` names `config` when one is
    /// given and is removed otherwise; `CONDUCTOR_INSTANCE` is removed.
    fn start_watch(&self, config: Option<&Path>, args: &[&str]) -> Child {
        let (home, work, bin) = self.binary_dirs();
        let mut command = common::conductor(&home);
        command
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(["watch", "run"])
            .args(args)
            .current_dir(&work)
            .env("PATH", &bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(config) = config {
            command.env(config::CONFIG_VARIABLE, config);
        }
        command.spawn().expect("the conductor binary starts")
    }
}

/// Waits up to `wait` for `child` to exit, and stops it when it has not: its exit code (`None`
/// when it was stopped), standard output and standard error.
fn finish(mut child: Child, wait: Duration) -> (Option<i32>, String, String) {
    let started = Instant::now();
    let mut exited = child.try_wait().expect("ask after the watch");
    while exited.is_none() && started.elapsed() < wait {
        thread::sleep(Duration::from_millis(50));
        exited = child.try_wait().expect("ask after the watch");
    }
    if exited.is_none() {
        child.kill().expect("stop the watch");
    }
    let output = child
        .wait_with_output()
        .expect("collect the watch's output");
    (
        exited.and_then(|status| status.code()),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Waits up to 30 s for `done`.
fn wait_for(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "waited 30 s for {what}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Acceptance: with a config whose instance is `acme` and a snapshot holding two Active
/// repositories, one CI pass asks `gh` about exactly those two.
#[test]
fn the_binary_on_an_acme_config_asks_gh_about_exactly_the_two_active_repositories() {
    let case = Case::new("binary-acme-ci");
    let (_, _, bin) = case.binary_dirs();
    assert_eq!(
        take_snapshot(&case.dir.join("state"), &OBSERVED, true),
        SnapshotState::Complete
    );
    let calls = case.dir.join("gh-calls.txt");
    program(
        &bin,
        "gh",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nprintf '[]\\n'\n",
            calls.display()
        ),
    );
    let config = case.config(&acme_config(
        "\x20   thresholds: {disk_low: 1G, disk_clear: 2G, disk_admit: 1G, gate_stop: 1G, \
         gate_resume: 2G}\n",
    ));
    let watch = case.start_watch(Some(&config), &[]);
    wait_for("two run-list calls", || sorted_lines(&calls).len() >= 2);
    thread::sleep(Duration::from_millis(500));
    let (code, stdout, stderr) = finish(watch, Duration::ZERO);
    assert_eq!(code, None, "the watch ended: {stderr}");
    assert_eq!(stdout, "", "{stderr}");
    let asked: Vec<String> = sorted_lines(&calls)
        .iter()
        .map(|call| {
            let words: Vec<&str> = call.split_whitespace().collect();
            let at = words
                .iter()
                .position(|word| *word == "-R")
                .unwrap_or_else(|| panic!("`gh {call}` names no repository"));
            words[at + 1].to_owned()
        })
        .collect();
    assert_eq!(asked, ["acme/alpha", "acme/beta"], "{stderr}");
    assert!(!stderr.contains("no snapshot"), "{stderr}");
}

/// Acceptance: the config's thresholds change when the watch reports disk low; without a file
/// the default is now 100G. The binary reads `/` through `statvfs`, so each threshold is set
/// around the free GiB this case reads there: 1000G above it is reported, 1G is not, and the
/// built-in 100G is reported exactly when `/` has less free.
#[test]
fn the_binary_reports_disk_low_under_100g_without_a_file_and_under_the_instances_disk_low() {
    let free = free_gib();
    let without = Case::new("binary-disk-built-in");
    let built_in = without.start_watch(None, &["--every", "1"]);

    let under = Case::new("binary-disk-acme-under");
    let config = under.config(&acme_config(&format!(
        "\x20   thresholds: {{disk_low: {}G, disk_clear: {}G, \
         disk_admit: 1G, gate_stop: 1G, gate_resume: 2G}}\n",
        free + 1000,
        free + 1001
    )));
    let reported = under.start_watch(Some(&config), &["--every", "1"]);

    let acme = Case::new("binary-disk-acme");
    let (_, _, bin) = acme.binary_dirs();
    let calls = acme.dir.join("claude-calls.txt");
    fake_claude(&bin, &calls);
    let config = acme.config(&acme_config(
        "\x20   thresholds: {disk_low: 1G, disk_clear: 2G, disk_admit: 1G, gate_stop: 1G, \
         gate_resume: 2G}\n",
    ));
    let configured = acme.start_watch(Some(&config), &["--every", "1"]);

    if free < 100 {
        let (code, stdout, stderr) = finish(built_in, Duration::from_secs(30));
        assert_eq!(code, Some(0), "{stderr}");
        // Under the built-in 20G and 15G the same pass reports `disk admit` and `disk gate
        // stop` after it (`story:watch-disk-floors`).
        let (low, floors) = stdout.split_at(stdout.find('\n').map_or(stdout.len(), |at| at + 1));
        assert_disk_low(low, free, &stderr);
        // Under `disk_admit` the watch reclaims and reports what it reached
        // (`story:disk-reclaim`); with no `worktree` on `PATH` it frees nothing here.
        assert!(
            floors.lines().all(|line| line.starts_with("disk admit: ")
                || line.starts_with("disk gate stop: ")
                || line.starts_with("disk floor not reached: ")
                || line.starts_with("disk reclaimed: ")),
            "{stdout:?}: {stderr}"
        );
    } else {
        let (code, stdout, stderr) = finish(built_in, Duration::from_millis(3500));
        assert_eq!(code, None, "the watch ended: {stdout} {stderr}");
        assert_eq!(stdout, "", "{free}G free is not under 100G: {stderr}");
    }

    let (code, stdout, stderr) = finish(reported, Duration::from_secs(30));
    assert_eq!(code, Some(0), "{stderr}");
    assert_disk_low(&stdout, free, &stderr);

    let (code, stdout, stderr) = finish(configured, Duration::from_millis(3500));
    assert_eq!(code, None, "the watch ended: {stdout} {stderr}");
    assert_eq!(
        stdout, "",
        "{free}G free is not under the instance's 1G: {stderr}"
    );
    assert!(
        sorted_lines(&calls).len() >= 2,
        "the watch passed more than once: {stderr}"
    );
}

/// W4 decision 2: the interval is the instance's `cadence.watch`, and `--every` overrides it.
/// Each watch follows, so a disk-low line on the real `/` does not end it, and a pass is a run
/// of the session list.
#[test]
fn the_binary_passes_at_the_instances_cadence_and_a_flag_overrides_it() {
    let start = |name: &str, extra: Option<&str>, args: &[&str]| {
        let case = Case::new(name);
        let (_, _, bin) = case.binary_dirs();
        fake_claude(&bin, &case.dir.join("claude-calls.txt"));
        let config = extra.map(|extra| case.config(&acme_config(extra)));
        let watch = case.start_watch(config.as_deref(), &[args, &["--follow"]].concat());
        (case, watch)
    };
    let cases = [
        start(
            "binary-cadence-file",
            Some("\x20   cadence: {watch: 1s}\n"),
            &[],
        ),
        start("binary-cadence-none", None, &[]),
        start(
            "binary-cadence-flag",
            Some("\x20   cadence: {watch: 1h}\n"),
            &["--every", "1"],
        ),
    ];
    thread::sleep(Duration::from_millis(3500));
    let mut passes = Vec::new();
    for (case, watch) in cases {
        let (code, stdout, stderr) = finish(watch, Duration::ZERO);
        assert_eq!(code, None, "the watch ended: {stdout} {stderr}");
        passes.push(sorted_lines(&case.dir.join("claude-calls.txt")).len());
    }
    assert!(passes[0] >= 2, "cadence.watch 1s: {passes:?}");
    assert_eq!(passes[1], 1, "without a file, 180 s: {passes:?}");
    assert!(
        passes[2] >= 2,
        "--every 1 over cadence.watch 1h: {passes:?}"
    );
}

/// Writes the transcript of the session `id` working in `cwd` under `home`, one assistant line
/// whose usage sums to `tokens`.
fn transcript(home: &Path, cwd: &Path, id: &str, tokens: u64) {
    let directory: String = cwd
        .display()
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = home.join(".claude/projects").join(directory);
    fs::create_dir_all(&dir).expect("create the transcript's directory");
    let line = json!({"type": "assistant", "timestamp": "2026-10-07T11:55:00.000Z",
                      "sessionId": id,
                      "message": {"role": "assistant", "content": [{"type": "text", "text": "Done."}],
                                  "usage": {"input_tokens": 1,
                                            "cache_read_input_tokens": tokens - 1,
                                            "cache_creation_input_tokens": 0}}});
    fs::write(dir.join(format!("{id}.jsonl")), format!("{line}\n")).expect("write the transcript");
}

/// W4 decision 6 through the binary: the sessions watched are those in the instance's checkouts
/// root, its managed trees and its records, each over the instance's `context_handover` once.
#[test]
fn the_binary_reports_the_context_of_the_sessions_in_the_instance_over_its_threshold() {
    let case = Case::new("binary-acme-context");
    let (home, _, bin) = case.binary_dirs();
    let root = home.join("acme");
    let trees = home.join("acme-trees");
    let records = case.dir.join("records");
    let sessions = [
        (
            "a1a1a1a1-0000-4000-8000-000000000001",
            "alpha",
            root.join("alpha"),
            451_000,
        ),
        (
            "b2b2b2b2-0000-4000-8000-000000000002",
            "beta",
            trees.join("beta/tree-1"),
            352_000,
        ),
        (
            "c3c3c3c3-0000-4000-8000-000000000003",
            "conductor",
            records.clone(),
            500_000,
        ),
        (
            "d4d4d4d4-0000-4000-8000-000000000004",
            "outsider",
            home.join("example-org/x"),
            900_000,
        ),
    ];
    let mut list = Vec::new();
    for (pid, (id, name, cwd, tokens)) in (2101_u64..).zip(&sessions) {
        transcript(&home, cwd, id, *tokens);
        list.push(session(id, name, pid, &cwd.display().to_string()));
    }
    // The fake prints the list with the shell's own printf: `PATH` holds no `cat`.
    program(
        &bin,
        "claude",
        &format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", Value::from(list)),
    );
    let config = case.config(&acme_config_in(
        &root,
        &trees,
        &format!(
            "\x20   records: {}\n\
             \x20   thresholds: {{context_handover: 400k, disk_low: 1G, disk_clear: 2G, \
             disk_admit: 1G, gate_stop: 1G, gate_resume: 2G}}\n",
            records.display()
        ),
    ));
    let (code, stdout, stderr) = finish(
        case.start_watch(Some(&config), &[]),
        Duration::from_secs(30),
    );
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        stdout,
        "context: alpha 451k tokens (session a1a1a1a1)\n\
         context: conductor 500k tokens (session c3c3c3c3)\n",
        "beta's 352k is under the instance's 400k, and the outsider works outside the \
         instance: {stderr}"
    );
    let kept = fs::read_to_string(case.dir.join("state/watch/sessions.json"))
        .expect("the watch kept the session list");
    assert!(!kept.contains("outsider"), "{kept}");
}

// ---------------------------------------------------------------------------------------------
// The session tasks and the organization's literals (W4 decision 4; wiring-common)
// ---------------------------------------------------------------------------------------------

fn taskfile() -> serde_yaml::Value {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Taskfile.yml");
    let text = fs::read_to_string(&file).expect("read Taskfile.yml");
    serde_yaml::from_str(&text).expect("Taskfile.yml is YAML")
}

/// The session tasks of `Taskfile.yml`.
const SESSION_TASKS: [&str; 6] = [
    "sessions",
    "conductor:start",
    "conductor:attach",
    "conductor:restart",
    "dev:start",
    "dev:attach",
];

#[test]
fn the_session_tasks_start_their_role_on_the_harness_and_model_config_show_names() {
    let taskfile = taskfile();
    for (name, role) in [
        ("conductor:start", "conductor"),
        ("dev:start", "conductor-dev"),
    ] {
        let task = &taskfile["tasks"][name];
        for (var, field) in [("HARNESS", "harness"), ("MODEL", "model")] {
            let sh = task["vars"][var]["sh"]
                .as_str()
                .unwrap_or_else(|| panic!("{name} has no `sh:` var {var}"));
            assert_eq!(
                sh,
                format!(
                    "conductor config show --format json | jq -r '.instances[0].roles[] | \
                     select(.role == \"{role}\") | .{field}'"
                ),
                "{name}"
            );
        }
        // The start reads the role's model itself (`story:session-envelope-seam`): the same
        // `conductor config show` role the variables read.
        let cmds = serde_yaml::to_string(&task["cmds"]).expect("cmds as text");
        assert!(
            cmds.contains(&format!("conductor session start --role {role}")),
            "{name}: {cmds}"
        );
        assert!(!cmds.contains("--model opus"), "{name}: {cmds}");
        let preconditions =
            serde_yaml::to_string(&task["preconditions"]).expect("preconditions as text");
        assert!(
            preconditions.contains("test {{shellQuote .HARNESS}} = claude"),
            "{name} starts claude only for a role on claude: {preconditions}"
        );
        assert!(
            preconditions.contains("test -n {{shellQuote .MODEL}}"),
            "{name} refuses a role the instance does not name: {preconditions}"
        );
    }

    // What the tasks read without a file: the built-in roles, on claude and opus, as today.
    let case = Case::new("taskfile-roles");
    let (home, work, _) = case.binary_dirs();
    let output = common::conductor(&home)
        .args(["config", "show", "--format", "json"])
        .current_dir(&work)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let shown: Value = serde_json::from_slice(&output.stdout).expect("config show prints JSON");
    for role in ["conductor", "conductor-dev"] {
        let found = shown["instances"][0]["roles"]
            .as_array()
            .and_then(|roles| roles.iter().find(|entry| entry["role"] == role))
            .unwrap_or_else(|| panic!("no role {role}: {shown:#}"));
        assert_eq!(found["harness"], "claude", "{role}");
        assert_eq!(found["model"], "opus", "{role}");
    }
}

/// The templates of `Taskfile.yml` that are command text, not values: `SESSION_ID` and
/// `SESSION_COUNT` are command lines the tasks run inside `$(…)`.
const TEMPLATED_COMMANDS: [&str; 2] = ["{{.SESSION_ID}}", "{{.SESSION_COUNT}}"];

/// Every value `Taskfile.yml` writes into a shell command (a `cmds:` line or a precondition's
/// `sh:`) is written `{{shellQuote .X}}`, so a value read from the config, from a session list or
/// from go-task itself is one shell word whatever it holds; the only other template is the command
/// text of [`TEMPLATED_COMMANDS`].
#[test]
fn every_value_the_taskfile_writes_into_a_shell_command_is_shell_quoted() {
    fn shell_lines(task: &serde_yaml::Value, out: &mut Vec<String>) {
        for cmd in task["cmds"].as_sequence().into_iter().flatten() {
            if let Some(text) = cmd.as_str() {
                out.push(text.to_owned());
            }
        }
        for precondition in task["preconditions"].as_sequence().into_iter().flatten() {
            if let Some(text) = precondition["sh"].as_str() {
                out.push(text.to_owned());
            }
        }
    }
    let taskfile = taskfile();
    let mut unquoted = Vec::new();
    let mut quoted = 0;
    for (name, task) in taskfile["tasks"].as_mapping().expect("tasks is a mapping") {
        let name = name.as_str().expect("a task name");
        let mut lines = Vec::new();
        shell_lines(task, &mut lines);
        for line in lines {
            let mut rest = line.as_str();
            while let Some(start) = rest.find("{{") {
                let end = rest[start..]
                    .find("}}")
                    .map(|end| start + end + 2)
                    .unwrap_or_else(|| panic!("{name}: an unclosed template in {line:?}"));
                let template = &rest[start..end];
                let value = template
                    .strip_prefix("{{shellQuote .")
                    .and_then(|inner| inner.strip_suffix("}}"))
                    .is_some_and(|var| var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
                if value {
                    quoted += 1;
                    let before = rest[..start].chars().next_back();
                    let after = rest[end..].chars().next();
                    if matches!(before, Some('"' | '\'')) || matches!(after, Some('"' | '\'')) {
                        unquoted.push(format!("{name}: {template} inside quotes in {line:?}"));
                    }
                } else if !TEMPLATED_COMMANDS.contains(&template) {
                    unquoted.push(format!("{name}: {template} in {line:?}"));
                }
                rest = &rest[end..];
            }
        }
    }
    assert!(
        unquoted.is_empty(),
        "values reach a shell unquoted:\n  - {}",
        unquoted.join("\n  - ")
    );
    assert!(quoted >= 10, "the tasks quote {quoted} values");
}

#[test]
fn no_organization_literal_is_left_in_the_watch_or_the_session_tasks() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/watch.rs"))
        .expect("read src/watch.rs");
    let taskfile = taskfile();
    let tasks: Vec<(String, String)> = SESSION_TASKS
        .iter()
        .map(|name| {
            let task = &taskfile["tasks"][*name];
            assert!(!task.is_null(), "Taskfile.yml has no task {name}");
            (
                format!("Taskfile.yml task {name}"),
                serde_yaml::to_string(task).expect("a task as text"),
            )
        })
        .collect();
    for word in ["example-org"] {
        for (at, line) in source.lines().enumerate() {
            assert!(
                !line.contains(word),
                "src/watch.rs:{}: {word}: {line}",
                at + 1
            );
        }
        for (what, text) in &tasks {
            assert!(!text.contains(word), "{what}: {word}: {text}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Escalated decision requests (story:decisions-notify)
// ---------------------------------------------------------------------------------------------

/// 29 minutes after [`noon`]: an escalation seen at noon is not yet 30 minutes unanswered.
fn twelve_twenty_nine() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:29:00Z", &Rfc3339).expect("a fixed instant")
}

/// 30 minutes after [`noon`]: an escalation seen at noon is 30 minutes unanswered.
fn twelve_thirty() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:30:00Z", &Rfc3339).expect("a fixed instant")
}

impl Case {
    /// Runs the built binary with `args` over the case's store, `state/`, with `HOME` the case's
    /// `home/`; asserts it succeeded.
    fn decide(&self, args: &[&str]) {
        let home = self.dir.join("home");
        fs::create_dir_all(&home).expect("create home/");
        let output = common::conductor(&home)
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "`{}`: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Raises the decision request `id` of `alpha` asking `question`, raised at 11:00.
    fn raise(&self, id: &str, question: &str) {
        self.decide(&[
            "decision",
            "raise-decision-request",
            "--request-id",
            id,
            "--repository",
            "alpha",
            "--question",
            question,
            "--options",
            "A: now; B: later",
            "--recommendation",
            "A",
            "--raised-at",
            "2026-10-07T11:00:00Z",
        ]);
    }

    /// Escalates the request `id` as `class` (`O` or `H`; an `H` item carries why it has no dry
    /// run, and an `O` item what the operator chooses from, with a deadline two days on).
    fn escalate(&self, id: &str, class: &str) {
        self.escalate_until(id, class, (class == "O").then_some("2026-10-09T12:00:00Z"));
    }

    /// Escalates the request `id` as `class` with the deadline `deadline`, when given, and what
    /// the operator chooses from (`story:decisions-page`).
    fn escalate_until(&self, id: &str, class: &str, deadline: Option<&str>) {
        let mut args = vec![
            "decision",
            "escalate-request",
            "--request-id",
            id,
            "--escalation-class",
            class,
            "--reason",
            "the operator's call",
        ];
        if class == "H" {
            args.extend(["--no-dry-run", "the step needs the operator's login"]);
        }
        if let Some(deadline) = deadline {
            args.extend([
                "--kind",
                "yes-no",
                "--lettered-options",
                r#"[{"letter":"Y","label":"yes","cost":"an hour"},{"letter":"N","label":"no","cost":"nothing"}]"#,
                "--recommended-letters",
                "Y",
                "--default-letters",
                "N",
                "--deadline",
                deadline,
            ]);
        }
        self.decide(&args);
    }

    /// A watch of this case reading the escalated requests of its store, at `clock`.
    fn decisions_watch(&self, clock: fn() -> OffsetDateTime) -> Watch {
        let mut sources = self.sources(None);
        sources.clock = clock;
        Watch::new(sources, self.state()).with_decisions(self.dir.join("state"))
    }
}

/// Acceptance 1: `decision escalate-request` makes the next pass print `escalated <O|H>
/// <request-id> <question>`, once; a request that is only open prints nothing.
#[test]
fn an_escalated_request_is_printed_at_once_and_once_only() {
    let case = Case::new("decisions-escalated");
    case.sessions(&[]);
    case.raise("REQ-1", "Release the pilot now?");
    case.raise("REQ-2", "Rotate the deploy key?");
    case.raise("REQ-3", "Which token scope?");
    assert_eq!(pass(&mut case.decisions_watch(noon)), NOTHING);

    case.escalate("REQ-1", "O");
    case.escalate("REQ-2", "H");
    assert_eq!(
        pass(&mut case.decisions_watch(noon)),
        [
            "escalated O REQ-1 Release the pilot now?",
            "escalated H REQ-2 Rotate the deploy key?",
        ]
    );
    assert_eq!(pass(&mut case.decisions_watch(noon)), NOTHING);
}

/// Acceptance 1: an escalated request is printed `unanswered <O|H> <request-id> <minutes>` once
/// it has waited 30 minutes from the pass that printed its escalation, and once only; a request
/// withdrawn before then prints nothing more.
#[test]
fn an_escalated_request_unanswered_for_30_minutes_is_printed_once() {
    let case = Case::new("decisions-unanswered");
    case.sessions(&[]);
    case.raise("REQ-1", "Release the pilot now?");
    case.raise("REQ-2", "Rotate the deploy key?");
    case.escalate("REQ-1", "O");
    case.escalate("REQ-2", "H");
    assert_eq!(pass(&mut case.decisions_watch(noon)).len(), 2);

    assert_eq!(pass(&mut case.decisions_watch(twelve_twenty_nine)), NOTHING);
    case.decide(&[
        "decision",
        "withdraw-request",
        "--request-id",
        "REQ-2",
        "--reason",
        "the controller found another way",
    ]);
    assert_eq!(
        pass(&mut case.decisions_watch(twelve_thirty)),
        ["unanswered O REQ-1 30"]
    );
    assert_eq!(pass(&mut case.decisions_watch(one_pm)), NOTHING);
}

fn twelve_nine() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:09:00Z", &Rfc3339).expect("a fixed instant")
}

fn twelve_ten() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:10:00Z", &Rfc3339).expect("a fixed instant")
}

/// `story:decisions-page`: an escalated request whose escalation records a deadline is printed
/// `unanswered <O|H> <request-id> <minutes>` once, in the first pass at or after its deadline,
/// with the minutes since the pass that printed its escalation; its 30-minute line is still
/// printed once. A request without a deadline prints only the 30-minute line.
#[test]
fn an_escalated_request_is_printed_unanswered_once_at_its_deadline() {
    let case = Case::new("decisions-deadline");
    case.sessions(&[]);
    case.raise("REQ-1", "Release the pilot now?");
    case.raise("REQ-2", "Rotate the deploy key?");
    case.raise("REQ-3", "Which token scope?");
    case.escalate_until("REQ-1", "O", Some("2026-10-07T12:10:00Z"));
    case.escalate_until("REQ-2", "H", Some("2026-10-07T12:10:00Z"));
    case.escalate_until("REQ-3", "H", None);
    assert_eq!(pass(&mut case.decisions_watch(noon)).len(), 3);

    assert_eq!(pass(&mut case.decisions_watch(twelve_nine)), NOTHING);
    assert_eq!(
        pass(&mut case.decisions_watch(twelve_ten)),
        ["unanswered O REQ-1 10", "unanswered H REQ-2 10"]
    );
    assert_eq!(pass(&mut case.decisions_watch(twelve_twenty_nine)), NOTHING);
    assert_eq!(
        pass(&mut case.decisions_watch(twelve_thirty)),
        [
            "unanswered O REQ-1 30",
            "unanswered H REQ-2 30",
            "unanswered H REQ-3 30",
        ]
    );
    assert_eq!(pass(&mut case.decisions_watch(one_pm)), NOTHING);
}

/// `story:decisions-page`: a deadline that has already passed when the escalation is first seen
/// prints the escalation and its unanswered line in that same pass.
#[test]
fn an_escalation_seen_after_its_deadline_is_printed_unanswered_in_the_same_pass() {
    let case = Case::new("decisions-deadline-passed");
    case.sessions(&[]);
    case.raise("REQ-1", "Release the pilot now?");
    case.escalate_until("REQ-1", "O", Some("2026-10-07T11:30:00Z"));
    assert_eq!(
        pass(&mut case.decisions_watch(noon)),
        [
            "escalated O REQ-1 Release the pilot now?",
            "unanswered O REQ-1 0",
        ]
    );
    assert_eq!(pass(&mut case.decisions_watch(twelve_ten)), NOTHING);
}

/// `story:decisions-batch-answer`: a printed escalated request that is answered is printed
/// `answered <request-id> <choice>` once, in the next pass, whether the operator answered it on
/// the Decisions page or conductor relayed it (`answer-escalated-as-operator`, or
/// `record-operator-decision` with `answer-escalated-request`); the choice is printed on one
/// line. A withdrawn request prints nothing.
#[test]
fn an_answered_request_is_printed_once_and_a_withdrawn_one_not_at_all() {
    let case = Case::new("decisions-answered");
    case.sessions(&[]);
    case.raise("REQ-1", "Release the pilot now?");
    case.raise("REQ-2", "Rotate the deploy key?");
    case.raise("REQ-3", "Which token scope?");
    case.raise("REQ-4", "Drop the old job?");
    case.escalate("REQ-1", "O");
    case.escalate("REQ-2", "H");
    case.escalate("REQ-3", "O");
    case.escalate("REQ-4", "O");
    assert_eq!(pass(&mut case.decisions_watch(noon)).len(), 4);

    case.decide(&[
        "decision",
        "answer-escalated-as-operator",
        "--request-id",
        "REQ-1",
        "--choice",
        "Y",
        "--recorded-by",
        "dashboard",
        "--operator-words",
        "chosen on the Decisions page",
    ]);
    case.decide(&[
        "decision",
        "answer-escalated-as-operator",
        "--request-id",
        "REQ-2",
        "--choice",
        "done",
        "--recorded-by",
        "conductor",
        "--operator-words",
        "done, the key is rotated",
    ]);
    case.decide(&[
        "decision",
        "record-operator-decision",
        "--decision-id",
        "DEC-20261007-01",
        "--decision-class",
        "O",
        "--question",
        "Which token scope?",
        "--options",
        "A: now; B: later",
        "--choice",
        "B\nthen A",
        "--reason",
        "he said so",
        "--evidence",
        "his message",
        "--decided-at",
        "2026-10-07T12:05:00Z",
        "--recorded-by",
        "conductor",
        "--operator-words",
        "B, then A",
    ]);
    case.decide(&[
        "decision",
        "answer-escalated-request",
        "--request-id",
        "REQ-3",
        "--decision-id",
        "DEC-20261007-01",
    ]);
    case.decide(&[
        "decision",
        "withdraw-request",
        "--request-id",
        "REQ-4",
        "--reason",
        "the controller found another way",
    ]);
    assert_eq!(
        pass(&mut case.decisions_watch(twelve_ten)),
        [
            "answered REQ-1 Y",
            "answered REQ-2 done",
            "answered REQ-3 B then A",
        ]
    );
    assert_eq!(pass(&mut case.decisions_watch(twelve_thirty)), NOTHING);
}
