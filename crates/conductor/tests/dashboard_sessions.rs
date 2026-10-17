//! `story:sessions-page-coherent`: the Sessions page's data, `/data.json`, holds exactly the
//! instance's sessions with what each works on, and every list on it agrees with the page it
//! summarizes.
//!
//! - The rows are the instance's sessions from its records: its conductor and each controller the
//!   store holds running or paused, by the session name the controller was started under, never
//!   a session the working-directory filter happens to catch.
//! - A controller's dispatch is matched through its record's repository, whether the dispatch
//!   names the repository or the prefixed session name.
//! - "Awaiting the operator" is the Decisions page's open list, less an item a decision in the log
//!   already answered, each with the one action it asks of the operator and a link to it.
//! - A usage-limit line shows only while its limit is in force, with its reset time; a context
//!   line names only the instance's sessions.
//! - "Recent decisions" lists decisions only, newest first, one row per decision.
//!
//! The cases serve [`dashboard::serve_over`] in this process over a store under this test
//! target's temporary directory; the controllers are started and the requests raised with the
//! built binary over that store, under a config file of the case's own whose instance carries a
//! `session_prefix`. Nothing is read from the environment.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use conductor_cli::dashboard::{self, Sources};
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// The prefix the case's instance names its controllers with.
const PREFIX: &str = "ex";

/// One case's directory: `home/` with the session list, the records and the watch's files,
/// `state/` with the store.
struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn rfc3339(instant: OffsetDateTime) -> String {
    instant
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("format an instant")
}

fn millis(at: OffsetDateTime) -> i64 {
    i64::try_from(at.unix_timestamp_nanos() / 1_000_000).expect("milliseconds fit")
}

/// `instant` as a usage-limit line writes its reset, in UTC: `3:05pm`.
fn clock(instant: OffsetDateTime) -> String {
    let instant = instant.to_offset(UtcOffset::UTC);
    let (hour, minute) = (instant.hour(), instant.minute());
    let half = if hour < 12 { "am" } else { "pm" };
    let twelve = match hour % 12 {
        0 => 12,
        other => other,
    };
    format!("{twelve}:{minute:02}{half}")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-sessions")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        write(
            &case.config(),
            &format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example\n\
                 \x20   session_prefix: {PREFIX}\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
                 \x20   records: {records:?}\n\
                 \x20   state: {state:?}\n",
                root = case.home().join("example-org"),
                trees = case.home().join("trees"),
                records = case.records(),
                state = case.state(),
            ),
        );
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Runs the built binary with `args` over the case's store under the case's config file,
    /// asserting it succeeded; its standard output.
    fn run(&self, args: &[&str]) -> String {
        let output = common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--config")
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.state())
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
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Starts the controller of `repository`; its id.
    fn controller(&self, repository: &str) -> String {
        self.run(&[
            "controller",
            "start-controller",
            "--repository",
            repository,
            "--harness",
            "Claude",
        ])
        .trim()
        .to_owned()
    }

    fn sessions(&self, rows: &Value) {
        write(&self.home().join("agents.json"), &rows.to_string());
    }

    fn raise(&self, id: &str, question: &str) {
        self.run(&[
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
            "2026-10-13T09:00:00Z",
        ]);
    }

    fn escalate_yes_no(&self, id: &str) {
        self.run(&[
            "decision",
            "escalate-request",
            "--request-id",
            id,
            "--escalation-class",
            "O",
            "--reason",
            "the operator's call",
            "--kind",
            "yes-no",
            "--lettered-options",
            r#"[{"letter":"Y","label":"release today","cost":"an hour"},{"letter":"N","label":"wait","cost":"a day"}]"#,
            "--recommended-letters",
            "Y",
            "--default-letters",
            "N",
            "--deadline",
            "2026-10-14T09:00:00Z",
        ]);
    }

    fn escalate_hands(&self, id: &str) {
        self.run(&[
            "decision",
            "escalate-request",
            "--request-id",
            id,
            "--escalation-class",
            "H",
            "--reason",
            "only the operator holds the login",
            "--no-dry-run",
            "the step needs the operator's login",
        ]);
    }

    fn sources(&self) -> Sources {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources
    }

    fn start(&self) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || dashboard::serve_over(listener, sources, Some(state)));
        port
    }
}

/// `GET path` on `port`, answered 200 with JSON.
fn get(port: u16, path: &str) -> Value {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    assert!(
        head.starts_with("HTTP/1.1 200"),
        "GET {path}: {head}\n{body}"
    );
    serde_json::from_str(body).unwrap_or_else(|error| panic!("GET {path}: {error}: {body}"))
}

fn names(data: &Value) -> Vec<&str> {
    data["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect()
}

fn row<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}: {data:#}"))
}

/// The conductor session's name for the instance the in-process server runs: the scratch
/// instance `active::isolate` writes, which names no prefix.
const CONDUCTOR: &str = "conductor";

/// Acceptance 1 and 2: the rows are the instance's conductor and its controllers from the
/// store's records, each under its prefixed session name; a session the records do not name is
/// not listed, wherever it works; a controller whose session is not running is listed as not
/// running; the count of running controllers is that of the rows. Each controller row carries
/// its open dispatch, matched through its repository, whether the dispatch names the repository
/// or the session name.
#[test]
fn the_rows_are_the_instance_s_sessions_from_its_records_with_their_open_dispatch() {
    let case = Case::new("rows");
    case.controller("alpha");
    case.controller("beta");
    case.controller("gamma");
    let now = OffsetDateTime::now_utc();
    let started = millis(now - time::Duration::hours(2));
    let home = case.home();
    let at = |path: &str| home.join(path).display().to_string();
    case.sessions(&json!([
        {"name": "ex-alpha", "kind": "background", "status": "busy", "state": "working",
         "startedAt": started, "cwd": at("example-org/alpha"), "sessionId": "s-alpha",
         "pid": 1001},
        {"name": "ex-beta", "kind": "background", "status": "idle", "state": "idle",
         "startedAt": started, "cwd": at("trees/beta/beta-w1"), "sessionId": "s-beta",
         "pid": 1002},
        {"name": CONDUCTOR, "kind": "interactive", "status": "idle", "state": "idle",
         "startedAt": started, "cwd": at("example-org/conductor"), "sessionId": "s-conductor",
         "pid": 1003},
        // Started by hand in the checkouts root: not the instance's.
        {"name": "beta-notes", "kind": "interactive", "status": "busy", "state": "working",
         "startedAt": started, "cwd": at("example-org"), "sessionId": "s-notes", "pid": 1004},
        // Named after its checkout, but no record names it: not the instance's.
        {"name": "delta", "kind": "interactive", "status": "busy", "state": "working",
         "startedAt": started, "cwd": at("example-org/delta"), "sessionId": "s-delta",
         "pid": 1005}
    ]));
    let sent = |minutes: i64| rfc3339(now - time::Duration::minutes(minutes));
    let lines = [
        // alpha's dispatch names the bare repository, as `dispatch send` writes it.
        json!({"at": sent(60), "event": "sent", "id": "DSP-20261013-01", "to": "alpha",
               "repo": "alpha", "goal": "G1", "brief": "refund retries", "priority": 1}),
        json!({"at": sent(5), "event": "started", "id": "DSP-20261013-01", "repo": "alpha"}),
        // beta's first dispatch is done; its second names the prefixed session and is open.
        json!({"at": sent(90), "event": "sent", "id": "DSP-20261013-02", "to": "beta",
               "repo": "beta", "goal": "G2", "brief": "old work", "priority": 1}),
        json!({"at": sent(80), "event": "done", "id": "DSP-20261013-02", "repo": "beta",
               "evidence": "tree beta-w0"}),
        json!({"at": sent(30), "event": "sent", "id": "DSP-20261013-03", "to": "ex-beta",
               "repo": "beta", "goal": "G3", "brief": "new work", "priority": 1}),
        // A dispatch to a session that is not the instance's is shown nowhere.
        json!({"at": sent(20), "event": "sent", "id": "DSP-20261013-04", "to": "delta",
               "repo": "delta", "goal": "G4", "brief": "elsewhere", "priority": 1}),
    ];
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    write(&case.records().join("dispatches/2026-10.jsonl"), &text);
    let port = case.start();
    let data = get(port, "/data.json");

    assert_eq!(
        names(&data),
        [CONDUCTOR, "ex-alpha", "ex-beta", "ex-gamma"],
        "the conductor first, then each controller the store runs, and no other: {data:#}"
    );
    assert_eq!(row(&data, CONDUCTOR)["role"], "conductor", "{data:#}");
    let alpha = row(&data, "ex-alpha");
    assert_eq!(alpha["role"], "controller");
    assert_eq!(alpha["repository"], "alpha");
    assert_eq!(alpha["state"], "working");
    assert_eq!(alpha["exited"], false);
    assert_eq!(alpha["dispatch"]["id"], "DSP-20261013-01", "{data:#}");
    assert_eq!(alpha["dispatch"]["open"], true, "{data:#}");
    assert_eq!(alpha["dispatch"]["goal"], "G1");
    assert_eq!(alpha["dispatch"]["brief"], "refund retries");
    assert_eq!(alpha["dispatch"]["event"], "started");
    assert_eq!(alpha["dispatch"]["event_ago"], "5m");
    let beta = row(&data, "ex-beta");
    assert_eq!(beta["dispatch"]["id"], "DSP-20261013-03", "{data:#}");
    assert_eq!(beta["dispatch"]["open"], true, "{data:#}");
    assert_eq!(beta["dispatch"]["goal"], "G3");
    // The last event is the shown dispatch's own: the open one has none yet, and the closed
    // one's `done` is not shown beside it.
    assert_eq!(beta["dispatch"]["event"], Value::Null, "{data:#}");
    let gamma = row(&data, "ex-gamma");
    assert_eq!(gamma["repository"], "gamma");
    assert_eq!(gamma["exited"], true, "no session runs it: {data:#}");
    assert_eq!(gamma["state"], "not running", "{data:#}");
    assert_eq!(gamma["dispatch"], Value::Null, "{data:#}");
    assert_eq!(
        data["controllers"], 2,
        "the running controller rows, ex-alpha and ex-beta: {data:#}"
    );
    assert!(
        !data.to_string().contains("DSP-20261013-04"),
        "a foreign session's dispatch is not shown: {data:#}"
    );
}

/// Acceptance 3: "Awaiting the operator" is the Decisions page's open list, item for item, less
/// an item a decision in the log answered (whether or not anything closed it in the store); a
/// legacy escalation the log holds and the store does not is not listed. Each row carries the one
/// action it asks: answer, with the options, for class O; do, with the command, for class H; and
/// the link to it on the Decisions page. `/api/unanswered` counts the same items.
#[test]
fn awaiting_the_operator_is_the_decisions_page_s_open_list_less_what_the_log_answered() {
    let case = Case::new("awaiting");
    case.sessions(&json!([]));
    case.raise("REQ-release", "Release alpha today?");
    case.raise(
        "REQ-login",
        "Log in to the registry: run `task registry-login` in the alpha checkout",
    );
    case.raise("REQ-answered", "Vendor the schema?");
    case.raise("REQ-open", "Not escalated yet?");
    case.escalate_yes_no("REQ-release");
    case.escalate_hands("REQ-login");
    case.escalate_yes_no("REQ-answered");
    let log = [
        // A legacy escalation, written by hand into the log and never into the store.
        json!({"at": "2026-10-10T03:43:40Z", "id": "ESC-20261010-01", "class": "O",
               "question": "Clear the shared cache?", "options": "A clear; B keep",
               "decided_by": "Operator"}),
        // The operator's answer to REQ-answered, recorded in the log only: the store still holds
        // the request escalated.
        json!({"at": "2026-10-13T10:00:00Z", "id": "DEC-20261013-05", "class": "O",
               "choice": "Y", "reason": "answered in the conductor session",
               "decided_by": "Operator", "request": ["REQ-answered: vendor the schema"]}),
        // A class-C decision that merely names an open item is no answer to it.
        json!({"at": "2026-10-13T10:01:00Z", "id": "DEC-20261013-06", "class": "C",
               "choice": "A", "reason": "escalated as REQ-release", "decided_by": "Conductor",
               "rests_on": "REQ-release is the operator's"}),
    ];
    let text: String = log.iter().map(|line| format!("{line}\n")).collect();
    write(&case.records().join("decisions/2026-10.jsonl"), &text);
    let port = case.start();

    let decisions = get(port, "/api/decisions");
    let open: Vec<&str> = decisions["open"]
        .as_array()
        .expect("open")
        .iter()
        .map(|row| row["request_id"].as_str().expect("an id"))
        .collect();
    assert_eq!(open, ["REQ-login", "REQ-release"], "{decisions:#}");

    let data = get(port, "/data.json");
    let awaiting = data["awaiting_operator"].as_array().expect("awaiting");
    let ids: Vec<&str> = awaiting
        .iter()
        .map(|row| row["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(ids, open, "the Decisions page's open items: {data:#}");
    assert_eq!(
        awaiting[0],
        json!({"id": "REQ-login", "class": "H", "repository": "alpha",
               "question": "Log in to the registry: run `task registry-login` in the alpha checkout",
               "action": "do: task registry-login",
               "link": "#/decisions/REQ-login"}),
        "{data:#}"
    );
    assert_eq!(
        awaiting[1],
        json!({"id": "REQ-release", "class": "O", "repository": "alpha",
               "question": "Release alpha today?",
               "action": "answer: Y release today · N wait",
               "link": "#/decisions/REQ-release"}),
        "{data:#}"
    );
    let unanswered = get(port, "/api/unanswered");
    assert_eq!(unanswered["count"], 2, "{unanswered:#}");
}

/// Acceptance 4: a usage-limit line shows while its reset is ahead, with that reset, and not
/// after it; a context line names only a session of the instance.
#[test]
fn a_usage_line_shows_while_its_limit_is_in_force_and_context_lines_are_the_instance_s() {
    let case = Case::new("usage");
    case.controller("alpha");
    let now = OffsetDateTime::now_utc();
    let home = case.home();
    let at = |path: &str| home.join(path).display().to_string();
    case.sessions(&json!([
        {"name": "ex-alpha", "kind": "background", "status": "busy", "state": "working",
         "startedAt": millis(now), "cwd": at("example-org/alpha"), "sessionId": "s-alpha-0001",
         "pid": 1001},
        {"name": "beta-notes", "kind": "interactive", "status": "busy", "state": "working",
         "startedAt": millis(now), "cwd": at("example-org"), "sessionId": "s-notes-0002",
         "pid": 1002}
    ]));
    let ahead = now + time::Duration::hours(2);
    let passed = now - time::Duration::hours(2);
    let in_force = format!(
        "You've hit your session limit · resets {} (UTC)",
        clock(ahead)
    );
    let over = format!(
        "You've hit your session limit · resets {} (UTC)",
        clock(passed)
    );
    let watch = case.home().join("watch");
    let today = now.date();
    write(
        &watch.join("limit.seen"),
        &format!("{today} {over}\n{today} {in_force}\n"),
    );
    write(
        &watch.join("limit.reported"),
        &format!(
            "{} {over}\n{} {in_force}\n",
            rfc3339(now - time::Duration::hours(3)),
            rfc3339(now - time::Duration::hours(1))
        ),
    );
    write(&watch.join("limit.new"), &format!("{in_force}\tex-alpha\n"));
    write(
        &watch.join("context-reported"),
        "s-alpha-0001\ns-notes-0002\n",
    );
    let port = case.start();
    let data = get(port, "/data.json");
    let limits = data["watch"]["limits"].as_array().expect("limits");
    assert_eq!(limits.len(), 1, "only the limit in force: {data:#}");
    assert_eq!(limits[0]["text"], in_force.as_str());
    assert_eq!(limits[0]["sessions"], json!(["ex-alpha"]));
    let resets = limits[0]["resets_at"]
        .as_str()
        .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok())
        .unwrap_or_else(|| panic!("no reset time: {data:#}"));
    assert!(
        (resets - ahead).whole_minutes().abs() <= 1,
        "the reset {resets} is the line's {ahead}: {data:#}"
    );
    assert!(!data.to_string().contains(&over), "{data:#}");
    assert_eq!(
        data["watch"]["context"],
        json!([{"session": "s-alpha-", "name": "ex-alpha"}]),
        "{data:#}"
    );
}

/// Takes one complete snapshot, of no repository, started at `started_at`, in the case's store.
fn snapshot(case: &Case, started_at: OffsetDateTime) {
    let store = conductor_cli::store::open(&case.state()).expect("open the store");
    let start = conductor_model::observation::StartSnapshot {
        started_at: conductor_model::primitives::Timestamp(rfc3339(started_at)),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let empty = conductor_cli::collect::Collector::new("empty", |_| Ok(()));
    let taken = conductor_cli::snapshot::take_from(store, start, &[empty])
        .expect("the driver ends the snapshot");
    assert_eq!(
        taken.state,
        conductor_model::observation::SnapshotState::Complete
    );
}

/// The age of the newest complete snapshot, which `conductor status` reads, is on the page, with
/// a warning when it is older than the instance's `cadence.cycle` (2h for the scratch instance).
#[test]
fn the_page_carries_the_newest_snapshot_s_age_and_says_when_it_is_older_than_its_cadence() {
    let case = Case::new("snapshot");
    case.sessions(&json!([]));
    let now = OffsetDateTime::now_utc();
    snapshot(&case, now - time::Duration::hours(7));
    let port = case.start();
    let data = get(port, "/data.json");
    let shown = &data["snapshot"];
    assert_eq!(shown["age"], "7h 0m", "{data:#}");
    assert_eq!(shown["cadence_seconds"], 7200, "{data:#}");
    assert_eq!(shown["stale"], true, "{data:#}");

    snapshot(&case, now - time::Duration::minutes(30));
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let fresh = loop {
        let data = get(port, "/data.json");
        if data["snapshot"]["age"] == "30m" || std::time::Instant::now() > deadline {
            break data;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    assert_eq!(fresh["snapshot"]["age"], "30m", "{fresh:#}");
    assert_eq!(fresh["snapshot"]["stale"], false, "{fresh:#}");
}

/// Acceptance 5: "Recent decisions" lists the log's decisions only, newest first, each once,
/// each with its id, class, choice and reason; a request, an escalation or a line without a
/// choice is not one.
#[test]
fn recent_decisions_are_decisions_only_newest_first_without_empty_rows() {
    let case = Case::new("recent");
    case.sessions(&json!([]));
    let log = [
        json!({"at": "2026-10-13T09:00:00Z", "id": "DEC-20261013-01", "class": "C",
               "choice": "A", "reason": "first", "decided_by": "Conductor"}),
        json!({"at": "2026-10-13T09:01:00Z", "event": "requested", "id": "dr-alpha-01",
               "repo": "alpha", "question": "Which store?", "options": "A; B",
               "recommendation": "A"}),
        json!({"at": "2026-10-13T09:02:00Z", "event": "escalated", "id": "dr-alpha-01",
               "repo": "alpha", "escalation_class": "O", "reason": "the operator's"}),
        json!({"at": "2026-10-13T09:03:00Z", "event": "decided", "id": "DEC-20261013-02",
               "repo": "alpha", "class": "O", "choice": "B", "reason": "second",
               "decided_by": "Operator", "question": "Which store?", "options": "A; B",
               "evidence": "the operator's words"}),
        json!({"at": "2026-10-13T09:04:00Z", "id": "H-20261013-login",
               "reason": "now blocking"}),
        json!({"at": "2026-10-13T09:05:00Z", "event": "decided", "id": "DEC-20261013-02",
               "repo": "alpha", "class": "O", "choice": "B", "reason": "second",
               "decided_by": "Operator", "question": "Which store?", "options": "A; B",
               "evidence": "the operator's words"}),
    ];
    let text: String = log.iter().map(|line| format!("{line}\n")).collect();
    write(&case.records().join("decisions/2026-10.jsonl"), &text);
    let port = case.start();
    let data = get(port, "/data.json");
    assert_eq!(
        data["decisions"],
        json!([
            {"id": "DEC-20261013-02", "at": "2026-10-13T09:05:00Z", "class": "O",
             "choice": "B", "reason": "second"},
            {"id": "DEC-20261013-01", "at": "2026-10-13T09:00:00Z", "class": "C",
             "choice": "A", "reason": "first"},
        ]),
        "{data:#}"
    );
}
