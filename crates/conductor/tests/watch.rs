//! `story:watch-command`: `conductor watch run` wakes conductor on the first change.
//!
//! The pass cases run [`Watch::pass`] in this process through its one seam, [`Sources`]: the
//! session list, `df` and each repository's run list are the output of `cat` on a file the case
//! writes, the transcripts are fixtures under `tests/fixtures/watch/`, the clock stands at
//! 2026-10-07T12:00:00Z, and the notifier appends its arguments to a file of the case. The loop
//! cases run [`Watch::run`] on a thread with a 1 s interval and read its lines as they are
//! printed. The last cases start the built binary with `--state-dir` and a `PATH` that holds no
//! program at all, or only fake `claude`, `df` and `gh` programs the case writes under its own
//! directory, so it reaches no live session list, transcript, `gh` or `notify-send`. A case that
//! gives the binary an instance writes a config file under its directory and names it in
//! `CONDUCTOR_CONFIG`; a case without one removes that variable, so no case reads the real home's
//! config. A snapshot the watch reads is taken through the library over a fake collector.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::cli::WatchRunArgs;
use conductor_cli::collect::Collector;
use conductor_cli::config;
use conductor_cli::snapshot;
use conductor_cli::store;
use conductor_cli::watch::{Options, Settings, Sources, Watch};
use conductor_model::config::{Gibibytes, Tokens};
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotId, SnapshotState, Visibility,
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
/// dispatch log, the run lists, `df`'s answer, the notifier's record and `state/`.
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

    /// `df -B1 --output=avail` answering `bytes`.
    fn disk(&self, bytes: u64) {
        replace(&self.dir.join("df.txt"), &format!("    Avail\n{bytes}\n"));
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
            disk: cat("df.txt"),
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
// The instance's settings (W4 decisions 2, 3 and 5)
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
        Settings::default(),
        Settings {
            instance: built_in.name.0.clone(),
            context_handover: 300_000,
            disk_low: 100,
            disk_clear: 110,
        },
        "a watch without settings runs on the built-in instance's"
    );
    assert_eq!(Settings::of(&built_in), Settings::default());
}

#[test]
fn the_settings_are_the_instances_name_and_thresholds() {
    let instance =
        acme("\x20   thresholds: {context_handover: 150k, disk_low: 40G, disk_clear: 45G}\n");
    assert_eq!(
        Settings::of(&instance),
        Settings {
            instance: "acme".to_owned(),
            context_handover: 150_000,
            disk_low: 40,
            disk_clear: 45,
        }
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

#[test]
fn the_binary_keeps_watching_without_a_line_while_no_source_answers() {
    let case = Case::new("binary");
    let home = case.dir.join("home");
    let bin = case.dir.join("empty-path");
    let work = case.dir.join("work");
    for dir in [&home, &bin, &work] {
        fs::create_dir_all(dir).expect("create the case's directories");
    }
    let state = case.dir.join("state");
    let mut child = Command::new(env!("CARGO_BIN_EXE_conductor"))
        .arg("--state-dir")
        .arg(&state)
        .args(["watch", "run", "--every", "1", "--ci-every", "1"])
        .current_dir(&work)
        .env("PATH", &bin)
        .env("HOME", &home)
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
        let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
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

/// A fake `df -B1 --output=avail /` that answers `gib` GiB free and appends a line to `calls`
/// each time it runs.
fn fake_df(bin: &Path, gib: u64, calls: &Path) {
    program(
        bin,
        "df",
        &format!(
            "#!/bin/sh\nprintf 'df\\n' >> '{}'\nprintf '    Avail\\n{}\\n'\n",
            calls.display(),
            gib * GIB
        ),
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
        let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
        command
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(["watch", "run"])
            .args(args)
            .current_dir(&work)
            .env("PATH", &bin)
            .env("HOME", &home)
            .env_remove(config::CONFIG_VARIABLE)
            .env_remove(config::INSTANCE_VARIABLE)
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
    let config = case.config(&acme_config(""));
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
/// the default is now 100G.
#[test]
fn the_binary_reports_disk_low_under_100g_without_a_file_and_under_the_instances_disk_low() {
    let without = Case::new("binary-disk-built-in");
    let (_, _, bin) = without.binary_dirs();
    fake_df(&bin, 50, &without.dir.join("df-calls.txt"));
    let built_in = without.start_watch(None, &["--every", "1"]);

    let acme = Case::new("binary-disk-acme");
    let (_, _, bin) = acme.binary_dirs();
    let calls = acme.dir.join("df-calls.txt");
    fake_df(&bin, 50, &calls);
    let config = acme.config(&acme_config(
        "\x20   thresholds: {disk_low: 40G, disk_clear: 45G}\n",
    ));
    let configured = acme.start_watch(Some(&config), &["--every", "1"]);

    let (code, stdout, stderr) = finish(built_in, Duration::from_secs(30));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "disk low: 50G free on /\n", "{stderr}");

    let (code, stdout, stderr) = finish(configured, Duration::from_millis(3500));
    assert_eq!(code, None, "the watch ended: {stdout} {stderr}");
    assert_eq!(
        stdout, "",
        "50G free is not under the instance's 40G: {stderr}"
    );
    assert!(
        sorted_lines(&calls).len() >= 2,
        "the disk was read on every pass: {stderr}"
    );
}

/// W4 decision 2: the interval is the instance's `cadence.watch`, and `--every` overrides it.
#[test]
fn the_binary_passes_at_the_instances_cadence_and_a_flag_overrides_it() {
    let start = |name: &str, extra: Option<&str>, args: &[&str]| {
        let case = Case::new(name);
        let (_, _, bin) = case.binary_dirs();
        fake_df(&bin, 500, &case.dir.join("df-calls.txt"));
        let config = extra.map(|extra| case.config(&acme_config(extra)));
        let watch = case.start_watch(config.as_deref(), args);
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
        passes.push(sorted_lines(&case.dir.join("df-calls.txt")).len());
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
             \x20   thresholds: {{context_handover: 400k}}\n",
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
        let cmds = serde_yaml::to_string(&task["cmds"]).expect("cmds as text");
        assert!(
            cmds.contains("--model {{shellQuote .MODEL}}"),
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
    let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
        .args(["config", "show", "--format", "json"])
        .current_dir(&work)
        .env("HOME", &home)
        .env_remove(config::CONFIG_VARIABLE)
        .env_remove(config::INSTANCE_VARIABLE)
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

/// The templates of `Taskfile.yml` that are command text, not values: `SESSION_ID` is a command
/// line the tasks run inside `$(…)`.
const TEMPLATED_COMMANDS: [&str; 1] = ["{{.SESSION_ID}}"];

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
