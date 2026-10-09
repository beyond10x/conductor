//! `story:live-dashboard`: `conductor dashboard serve`, a live, read-only page of what every
//! session works on; `story:dashboard-service`: exited sessions and the watch's lines;
//! `story:disk-waste`: the disk waste across the tracked repositories.
//!
//! The data cases serve [`dashboard::serve`] in this process on a free port of `127.0.0.1`,
//! through its one seam, [`Sources`]: the session list and the two `worktree` dry-runs are the
//! output of `cat` on a fixture, and the home directory, the conductor repository, the CI watch's
//! directory and `/var/tmp` are fixtures under this test target's temporary directory. Nothing is
//! read from the environment. The last case starts the built binary with `--port 0` and reads the
//! port from its first line.

mod active;
mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use conductor_cli::dashboard::{self, Sources};
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// A brief longer than 120 characters, with markup and a character wider than one byte at the
/// cut, so the page must escape it and the cut must fall on a character.
fn brief() -> String {
    format!("Fix <b>bold</b> & \"quoted\" {}", "é".repeat(150))
}

/// A reason longer than 120 characters, with markup.
fn reason() -> String {
    format!("<i>first</i> because {}", "x".repeat(150))
}

fn first(text: &str, characters: usize) -> String {
    text.chars().take(characters).collect()
}

fn rfc3339(instant: OffsetDateTime) -> String {
    instant
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("format an instant")
}

/// A case directory of its own: `home/` with the conductor repository, the CI watch's state
/// file and the session list under it.
struct Case {
    home: PathBuf,
}

/// A case that passed leaves nothing behind: its fixture repositories carry a `.git` that
/// `worktree gc` will not remove from a finished tree. A failed case keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking()
            && let Some(dir) = self.home.parent()
        {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let home = dir.join("home");
        let now = OffsetDateTime::now_utc();
        let millis = |ago: Duration| {
            let at = now - ago;
            i64::try_from(at.unix_timestamp_nanos() / 1_000_000).expect("milliseconds fit")
        };
        let at = |path: &str| home.join(path).display().to_string();
        let sessions = json!([
            {"id": "a1", "name": "alpha", "kind": "background", "state": "working",
             "status": "busy", "startedAt": millis(Duration::from_secs(7200)),
             "cwd": at("example-org/alpha"), "sessionId": "s1", "pid": 1001},
            {"id": "a2", "name": "conductor", "kind": "background", "state": "blocked",
             "startedAt": millis(Duration::from_secs(90)), "cwd": at("example-org/conductor"),
             "sessionId": "s2", "pid": 1002},
            {"name": "outsider", "kind": "interactive", "status": "idle",
             "startedAt": millis(Duration::ZERO), "cwd": at("elsewhere"), "sessionId": "s3"},
            {"name": "near-miss", "kind": "interactive", "status": "idle",
             "startedAt": millis(Duration::ZERO), "cwd": at("example-org-old/x"), "sessionId": "s4"}
        ]);
        write(&home.join("agents.json"), &sessions.to_string());

        let repository = home.join("example-org/conductor");
        let dispatch = json!({"id": "DSP-20261007-01", "to": "alpha", "repository": "alpha",
            "goal": "G1", "brief": brief(), "sent_at": "2026-10-07T00:00:00Z",
            "decision": "DEC-20261007-01"});
        let event = json!({"id": "DSP-20261007-01", "event": "started", "reported_by": "alpha",
            "evidence": "tree alpha-w1", "at": rfc3339(now - Duration::from_secs(300))});
        write(
            &repository.join("dispatches/2026-10.jsonl"),
            &format!("{dispatch}\n{event}\n"),
        );
        let decided = json!({"id": "DEC-20261007-01", "class": "C", "question": "Which first?",
            "options": "A or B", "choice": "A", "reason": reason(), "decided_by": "Conductor",
            "decided_at": "2026-10-07T00:00:00Z"});
        let awaiting = json!({"id": "DEC-20261007-02", "class": "O", "question": "Release now?",
            "options": "A now, B later", "reason": "needs the operator", "decided_by": "Operator",
            "decided_at": "2026-10-07T00:01:00Z"});
        write(
            &repository.join("decisions/2026-10.jsonl"),
            &format!("{decided}\n{awaiting}\n"),
        );
        write(
            &home.join(".cache/conductor-watch/ci.state"),
            "alpha|CI|101|success|2026-10-06T10:00:00Z\n\
             beta|Release|202|failure|2026-10-06T11:00:00Z\n",
        );
        write(
            &home.join("sweep.json"),
            &json!({"version": 4, "ok": true, "items": []}).to_string(),
        );
        write(
            &home.join("gc.json"),
            &json!({"version": 4, "ok": true, "assessments": []}).to_string(),
        );
        fs::create_dir_all(home.join("var-tmp")).expect("create the fixture /var/tmp");
        Self { home }
    }

    fn sources(&self) -> Sources {
        let cat = |file: &str| vec![OsString::from("cat"), self.home.join(file).into_os_string()];
        Sources {
            agents: cat("agents.json"),
            root: self.home.join("example-org/conductor"),
            ci_state: self.home.join(".cache/conductor-watch/ci.state"),
            home: self.home.clone(),
            checkouts: self.home.join("example-org"),
            trees: self.home.join(".local/state/worktree/trees/example-org"),
            targets: self.home.join(".cache/example-org-target"),
            disk: PathBuf::from("/"),
            watch: self.home.join(".cache/conductor-watch"),
            sweep: cat("sweep.json"),
            gc: cat("gc.json"),
            var_tmp: self.home.join("var-tmp"),
            measure_every: Duration::from_secs(600),
            measure_timeout: Duration::from_secs(30),
        }
    }

    /// Adds `row` to the session list.
    fn add_session(&self, row: Value) {
        let path = self.home.join("agents.json");
        let mut rows: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read the session list"))
                .expect("the session list is JSON");
        rows.as_array_mut().expect("a list").push(row);
        write(&path, &rows.to_string());
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
        walk(&self.home, &mut out);
        out
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

/// Serves `sources` in this process on a free port of `127.0.0.1`; the server ends with the test.
fn start(sources: Sources) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
    let port = listener.local_addr().expect("the bound address").port();
    std::thread::spawn(move || dashboard::serve(listener, sources));
    port
}

struct Answer {
    status: u16,
    head: String,
    body: String,
}

/// One request with `Host: <host>` (by default this listener), read to the end.
fn request(port: u16, method: &str, path: &str, host: Option<&str>) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    let host = host.map_or_else(|| format!("127.0.0.1:{port}"), str::to_owned);
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"));
    Answer {
        status,
        head: head.to_owned(),
        body: body.to_owned(),
    }
}

fn data(port: u16) -> Value {
    let answer = request(port, "GET", "/data.json", None);
    assert_eq!(answer.status, 200, "GET /data.json: {}", answer.body);
    assert!(
        answer
            .head
            .to_ascii_lowercase()
            .contains("content-type: application/json"),
        "GET /data.json is not JSON: {}",
        answer.head
    );
    serde_json::from_str(&answer.body).expect("/data.json is JSON")
}

fn ids(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .expect("a list of rows")
        .iter()
        .map(|row| row["id"].as_str().expect("a row id"))
        .collect()
}

#[test]
fn data_json_holds_one_row_per_session_under_the_checkouts_root_with_its_dispatch() {
    let case = Case::new("data");
    let home = case.home.display().to_string();
    let port = start(case.sources());
    let answer = request(port, "GET", "/data.json", None);
    assert!(
        !answer.body.contains(&home),
        "/data.json prints the home directory in full: {}",
        answer.body
    );
    let data = data(port);

    let sessions = data["sessions"].as_array().expect("sessions");
    let names: Vec<&str> = sessions
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["alpha", "conductor"], "{data:#}");
    let alpha = &sessions[0];
    assert_eq!(alpha["kind"], "background");
    assert_eq!(alpha["status"], "busy");
    assert_eq!(alpha["state"], "working");
    assert_eq!(alpha["cwd"], "~/example-org/alpha");
    assert_eq!(alpha["started_ago"], "2h 0m");
    let dispatch = &alpha["dispatch"];
    assert_eq!(dispatch["id"], "DSP-20261007-01");
    assert_eq!(dispatch["goal"], "G1");
    assert_eq!(dispatch["brief"], first(&brief(), 120));
    assert_eq!(dispatch["event"], "started");
    assert_eq!(dispatch["event_ago"], "5m");
    assert_eq!(sessions[1]["dispatch"], Value::Null, "{data:#}");

    assert_eq!(
        data["controllers"], 1,
        "only alpha is named after its checkout"
    );
    assert_eq!(data["build_slot"], "not recorded");
    assert!(
        data["disk"]["free_bytes"]
            .as_u64()
            .is_some_and(|free| free > 0),
        "{data:#}"
    );
    assert_eq!(
        data["red_ci"],
        json!([{"repo": "beta", "workflow": "Release", "run_id": "202",
                "conclusion": "failure", "time": "2026-10-06T11:00:00Z"}])
    );
    assert_eq!(
        ids(&data["decisions"]),
        ["DEC-20261007-02", "DEC-20261007-01"]
    );
    let decided = &data["decisions"][1];
    assert_eq!(decided["class"], "C");
    assert_eq!(decided["choice"], "A");
    assert_eq!(decided["reason"], first(&reason(), 120));
    assert_eq!(ids(&data["awaiting_operator"]), ["DEC-20261007-02"]);
    assert_eq!(data["errors"], json!([]), "{data:#}");
}

#[test]
fn the_page_names_each_session_escapes_every_value_and_reloads_itself() {
    let case = Case::new("page");
    let home = case.home.display().to_string();
    let port = start(case.sources());
    let page = request(port, "GET", "/", None);
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(
        page.head
            .to_ascii_lowercase()
            .contains("content-type: text/html"),
        "{}",
        page.head
    );
    for shown in [
        "alpha",
        "conductor",
        "DSP-20261007-01",
        "DEC-20261007-02",
        "Release",
    ] {
        assert!(page.body.contains(shown), "the page lacks {shown}");
    }
    for hidden in [
        "outsider",
        "near-miss",
        "<b>bold",
        "<i>first",
        "<script",
        &home,
    ] {
        assert!(
            !page.body.contains(hidden),
            "the page holds {hidden:?}:\n{}",
            page.body
        );
    }
    assert!(
        page.body
            .contains("&lt;b&gt;bold&lt;/b&gt; &amp; &quot;quoted&quot;")
    );
    assert!(
        page.body
            .contains(r#"<meta http-equiv="refresh" content="20">"#)
    );
    assert!(page.body.contains("~/example-org/alpha"));
}

#[test]
fn only_get_of_the_page_and_its_data_is_answered() {
    let case = Case::new("methods");
    let port = start(case.sources());
    let refused = request(port, "POST", "/", None);
    assert_eq!(refused.status, 405, "POST /");
    assert!(
        refused.head.contains("Allow: GET"),
        "POST / names no allowed method: {}",
        refused.head
    );
    assert_eq!(request(port, "POST", "/data.json", None).status, 405);
    assert_eq!(request(port, "DELETE", "/data.json", None).status, 405);
    assert_eq!(request(port, "GET", "/nope", None).status, 404);
    assert_eq!(request(port, "POST", "/nope", None).status, 404);
    assert_eq!(request(port, "GET", "/../data.json", None).status, 404);
    assert_eq!(
        request(port, "GET", "/data.json", Some("attacker.example")).status,
        403,
        "a request for another host is refused, so DNS rebinding cannot read the data"
    );
    assert_eq!(
        request(port, "GET", "/", Some(&format!("localhost:{port}"))).status,
        200
    );
}

#[test]
fn every_request_reads_the_sources_afresh_and_writes_nothing() {
    let case = Case::new("fresh");
    let port = start(case.sources());
    let before = case.files();
    assert_eq!(request(port, "GET", "/", None).status, 200);
    let first = data(port);
    assert_eq!(case.files(), before, "serving wrote under the case's home");
    assert_eq!(ids(&first["decisions"])[0], "DEC-20261007-02");

    let decisions = case
        .home
        .join("example-org/conductor/decisions/2026-10.jsonl");
    let mut text = fs::read_to_string(&decisions).expect("read the decisions");
    text.push_str(
        &json!({"id": "DEC-20261007-03", "class": "H", "question": "Plug the disk in?",
                "options": "A", "reason": "hands", "decided_by": "Operator",
                "decided_at": "2026-10-07T00:02:00Z"})
        .to_string(),
    );
    text.push('\n');
    fs::write(&decisions, text).expect("append a decision");
    let second = data(port);
    assert_eq!(ids(&second["decisions"])[0], "DEC-20261007-03");
    assert_eq!(
        ids(&second["awaiting_operator"]),
        ["DEC-20261007-03", "DEC-20261007-02"]
    );
}

#[test]
fn an_unreadable_source_is_named_on_the_page_not_fatal() {
    let case = Case::new("unreadable");
    let mut sources = case.sources();
    sources.agents = vec![OsString::from("false")];
    fs::remove_file(&sources.ci_state).expect("remove the CI state");
    let port = start(sources);
    let data = data(port);
    assert_eq!(data["sessions"], json!([]));
    assert_eq!(
        data["red_ci"],
        Value::Null,
        "no watch state is not an empty one"
    );
    let errors = data["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{data:#}");
    assert!(
        errors[0]
            .as_str()
            .is_some_and(|error| error.contains("false")),
        "{data:#}"
    );
    assert_eq!(
        ids(&data["decisions"]).len(),
        2,
        "the other sources still show"
    );
}

#[test]
fn a_session_without_a_pid_is_exited_and_listed_below_the_live_ones() {
    let case = Case::new("exited");
    case.add_session(json!({"id": "a5", "name": "aardvark", "kind": "background",
        "state": "blocked", "startedAt": 0,
        "cwd": case.home.join("example-org/aardvark").display().to_string(), "sessionId": "s5"}));
    let port = start(case.sources());
    let data = data(port);
    let sessions = data["sessions"].as_array().expect("sessions");
    let names: Vec<&str> = sessions
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["alpha", "conductor", "aardvark"], "{data:#}");
    assert_eq!(sessions[2]["state"], "exited", "{data:#}");
    assert_eq!(sessions[2]["exited"], true, "{data:#}");
    assert_eq!(sessions[0]["exited"], false, "{data:#}");
    assert_eq!(
        sessions[1]["state"], "blocked",
        "a live blocked session stays blocked"
    );
    assert_eq!(
        data["controllers"], 1,
        "an exited session named after its checkout is no running controller"
    );

    let page = request(port, "GET", "/", None).body;
    let row = |name: &str| {
        page.find(&format!("<tr><td>{name}</td>"))
            .unwrap_or_else(|| panic!("the page has no row for {name}:\n{page}"))
    };
    assert!(row("conductor") < row("aardvark"), "{page}");
    assert!(
        page.contains("<tr><td>aardvark</td><td>background</td><td>exited</td>"),
        "{page}"
    );
}

#[test]
fn the_page_shows_the_watch_usage_limit_and_context_lines() {
    let case = Case::new("watch");
    let port = start(case.sources());
    let none = data(port);
    assert_eq!(
        none["watch"],
        json!({"usage_limit": null, "usage_limit_today": null, "context": null}),
        "no watch file is no line: {none:#}"
    );
    assert_eq!(none["errors"], json!([]), "{none:#}");

    let watch = case.home.join(".cache/conductor-watch");
    let limit = "Usage limit reached · resets 15:00";
    write(
        &watch.join("limit.new"),
        &format!("{limit}\talpha\n{limit}\tconductor\n"),
    );
    let today = OffsetDateTime::now_utc().date();
    write(
        &watch.join("limit.seen"),
        &format!("{today} {limit}\n{today} Weekly limit reached\n2026-01-01 Old limit\n"),
    );
    write(&watch.join("context-reported"), "s2\nsold\n");
    let data = data(port);
    assert_eq!(
        data["watch"],
        json!({
            "usage_limit": [{"text": limit, "sessions": ["alpha", "conductor"]}],
            "usage_limit_today": [limit, "Weekly limit reached"],
            "context": [{"session": "s2", "name": "conductor"}],
        }),
        "{data:#}"
    );
    let page = request(port, "GET", "/", None).body;
    for line in [
        format!("usage limit: {limit}; 2 sessions: alpha conductor"),
        "Weekly limit reached".to_owned(),
        "context: conductor over 300k tokens (session s2)".to_owned(),
    ] {
        assert!(page.contains(&line), "the page lacks {line:?}:\n{page}");
    }
    assert!(!page.contains("Old limit"), "{page}");
}

#[test]
fn the_watch_state_is_read_from_the_stopgap_until_a_watch_run_kept_ci_state_in_the_repository() {
    let case = Case::new("watch-dir");
    let root = case.home.join("example-org/conductor");
    let stopgap = case.home.join(".cache/conductor-watch");
    let defaults = Sources::new(case.home.clone(), root.clone());
    assert_eq!(defaults.watch, stopgap, "no state/watch/ in the repository");
    assert_eq!(defaults.ci_state, stopgap.join("ci.state"));

    let watch = root.join("state/watch");
    write(&watch.join("sessions.json"), "{}\n");
    write(
        &watch.join("limit.seen"),
        "2026-10-07 Usage limit reached\n",
    );
    let defaults = Sources::new(case.home.clone(), root.clone());
    assert_eq!(
        defaults.watch, stopgap,
        "state/watch/ without ci.state is not read"
    );
    assert_eq!(defaults.ci_state, stopgap.join("ci.state"));

    write(
        &watch.join("ci.state"),
        "alpha|CI|101|success|2026-10-07T10:00:00Z\n",
    );
    let defaults = Sources::new(case.home.clone(), root);
    assert_eq!(defaults.watch, watch);
    assert_eq!(defaults.ci_state, watch.join("ci.state"));
}

/// The instance a config file names gives the sources: its records root, which `--root` beats,
/// the `watch/` of its state directory, and its checkouts root. Without a file the checkouts
/// root is the built-in instance's.
#[test]
fn the_sources_of_an_instance_are_its_records_its_state_watch_and_its_checkouts() {
    let case = Case::new("instance");
    let home = case.home.clone();
    let config = conductor_cli::config::parse(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: acme\n\
         \x20   sources: [{github: acme}]\n\
         \x20   checkouts: {root: ~/checkouts, trees: ~/trees}\n\
         \x20   records: ~/records\n\
         \x20   state: ~/state\n",
        &home,
    )
    .expect("the config converts");
    let instance = &config.instances[0];

    let sources = Sources::of(home.clone(), None, instance);
    assert_eq!(sources.root, home.join("records"));
    assert_eq!(sources.watch, home.join("state/watch"));
    assert_eq!(sources.ci_state, home.join("state/watch/ci.state"));
    assert_eq!(sources.checkouts, home.join("checkouts"));
    assert_eq!(sources.home, home);

    let elsewhere = home.join("elsewhere");
    let flagged = Sources::of(home.clone(), Some(elsewhere.clone()), instance);
    assert_eq!(
        flagged.root, elsewhere,
        "--root beats the instance's records"
    );
    assert_eq!(flagged.watch, home.join("state/watch"));

    let root = home.join("example-org/conductor");
    let defaults = Sources::new(home.clone(), root.clone());
    assert_eq!(
        defaults.checkouts,
        PathBuf::from(conductor_cli::config::built_in(&home, &root).checkouts.root)
    );
}

/// The sessions shown, and the controllers counted, are those under the checkouts root the
/// sources name, which the page's heading names.
#[test]
fn the_sessions_shown_are_those_under_the_checkouts_root() {
    let case = Case::new("checkouts");
    case.add_session(json!({"id": "a9", "name": "gamma", "kind": "background",
        "state": "working", "startedAt": 0, "sessionId": "s9", "pid": 1009,
        "cwd": case.home.join("acme/gamma").display().to_string()}));
    let mut sources = case.sources();
    sources.checkouts = case.home.join("acme");
    let port = start(sources);

    let data = data(port);
    let names: Vec<&str> = data["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["gamma"], "{data:#}");
    assert_eq!(data["controllers"], 1, "{data:#}");

    let page = request(port, "GET", "/", None);
    assert!(
        page.body
            .contains("<h2>Sessions under ~/acme (1 live, 0 exited)</h2>"),
        "{}",
        page.body
    );
}

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;

/// A file of `bytes` non-zero bytes, so it takes that much of the disk.
fn fill(path: &Path, bytes: u64) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(
        path,
        vec![0xAB_u8; usize::try_from(bytes).expect("a small file")],
    )
    .expect("write a case file");
}

/// One `worktree sweep --dry-run --json` item: an idle tree of `repository` at `tree`, holding
/// `bytes` of recognised cache.
fn sweep_item(repository: &Path, tree: &Path, bytes: u64) -> Value {
    json!({
        "record": {"id": "wt-1", "repository_root": repository, "path": tree, "purpose": "p",
                   "owner": "agent", "lifecycle": "active", "created_at": 0, "last_seen_at": 0,
                   "finished_at": null, "head": null},
        "idle_seconds": 172_800,
        "cache": {"id": "wt-1", "path": tree, "observed_at": 0, "applied": false,
                  "discarded": [{"path": "target", "kind": "cargo-target",
                                 "allocated_bytes": bytes}],
                  "retained_ignored": [], "retained_bytes": 0, "processes_observed": true},
    })
}

/// One `worktree gc --dry-run --json` assessment of the tree at `tree`.
fn assessment(repository: &Path, tree: &Path, eligible: bool) -> Value {
    json!({
        "record": {"id": "wt-2", "repository_root": repository, "path": tree, "purpose": "p",
                   "owner": "agent", "lifecycle": "finished", "created_at": 0, "last_seen_at": 0,
                   "finished_at": 0, "head": null},
        "eligible": eligible,
        "refusal": if eligible { Value::Null } else { json!({"code": "dirty"}) },
        "evidence": null,
    })
}

impl Case {
    /// The waste the disk measurement finds: for the tracked repository `alpha`, 5 GiB of cache in
    /// an idle tree, one tree eligible for removal (its 1 GiB of cache is not counted twice),
    /// archives and the checkout's `target/`; for a repository with no checkout, archives that
    /// are not shown; shared, a build target, rustup toolchains, the agents' scratch and two small
    /// `/var/tmp` entries.
    fn waste(&self) {
        let home = &self.home;
        let alpha = home.join("example-org/alpha");
        write(&alpha.join(".git/HEAD"), "ref: refs/heads/main\n");
        fill(&alpha.join("target/debug/big.bin"), 3 * MIB);
        fill(
            &home.join(".local/state/worktree/archives/alpha/one.tar"),
            MIB,
        );
        fill(
            &home.join(".local/state/worktree/archives/stranger/two.tar"),
            2 * MIB,
        );
        let old = home.join("trees/alpha-old");
        fill(&old.join("src.rs"), MIB / 2);
        let busy = home.join("trees/alpha-busy");
        fill(&busy.join("src.rs"), MIB / 2);
        self.sweep(5 * GIB);
        write(
            &home.join("gc.json"),
            &json!({"version": 4, "ok": true, "assessments": [
                assessment(&alpha, &old, true), assessment(&alpha, &busy, false),
            ]})
            .to_string(),
        );
        fill(
            &home.join(".cache/example-org-target/alpha/lib.rlib"),
            2 * MIB,
        );
        fill(&home.join(".rustup/toolchains/stable/rustc"), MIB);
        fill(&home.join(".cache/claude-tmp/scratch"), 4096);
        fill(&home.join("var-tmp/build-1/out"), 8192);
        fill(&home.join("var-tmp/note"), 4096);
    }

    /// Rewrites the sweep's dry-run with `bytes` of cache in alpha's idle tree.
    fn sweep(&self, bytes: u64) {
        let alpha = self.home.join("example-org/alpha");
        write(
            &self.home.join("sweep.json"),
            &json!({"version": 4, "ok": true, "items": [
                sweep_item(&alpha, &self.home.join("trees/alpha-idle"), bytes),
                sweep_item(&alpha, &self.home.join("trees/alpha-old"), GIB),
            ]})
            .to_string(),
        );
    }
}

/// `/data.json` once its disk measurement is complete and `until` holds, polled for at most 60 s.
fn measured(port: u16, until: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let data = data(port);
        if !data["disk"]["measured_at"].is_null() && until(&data) {
            return data;
        }
        assert!(
            Instant::now() < deadline,
            "no such measurement in 60 s: {data:#}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The disk row of `repository` (none for a shared one) that measures `what`.
fn row<'a>(data: &'a Value, repository: Option<&str>, what: &str) -> &'a Value {
    data["disk"]["rows"]
        .as_array()
        .expect("disk rows")
        .iter()
        .find(|row| row["repository"].as_str() == repository && row["what"] == what)
        .unwrap_or_else(|| panic!("no disk row {repository:?} {what:?}: {data:#}"))
}

fn bytes(row: &Value) -> u64 {
    row["bytes"]
        .as_u64()
        .unwrap_or_else(|| panic!("no bytes in {row:#}"))
}

#[test]
fn the_disk_object_carries_the_waste_rows_largest_first_with_their_total() {
    let case = Case::new("waste");
    case.waste();
    let home = case.home.display().to_string();
    let port = start(case.sources());
    let data = measured(port, |_| true);
    let disk = &data["disk"];
    assert!(
        disk["free_bytes"].as_u64().is_some_and(|free| free > 0),
        "{data:#}"
    );
    let measured_at = disk["measured_at"].as_str().expect("measured_at");
    assert!(
        OffsetDateTime::parse(measured_at, &Rfc3339)
            .is_ok_and(|at| at <= OffsetDateTime::now_utc()),
        "{measured_at}"
    );

    let cache = row(&data, Some("alpha"), "build cache in idle managed trees");
    assert_eq!(
        bytes(cache),
        5 * GIB,
        "the eligible tree's cache is not counted: {data:#}"
    );
    assert_eq!(cache["count"], 1);
    assert_eq!(cache["state"], "measured");
    let target = row(&data, Some("alpha"), "target/ of the checkout");
    assert_eq!(target["path"], "~/example-org/alpha/target");
    assert!((3 * MIB..4 * MIB).contains(&bytes(target)), "{target:#}");
    let eligible = row(&data, Some("alpha"), "trees eligible for removal");
    assert_eq!(eligible["count"], 1, "the tree gc retains is not eligible");
    assert!((MIB / 2..MIB).contains(&bytes(eligible)), "{eligible:#}");
    let archives = row(&data, Some("alpha"), "archives");
    assert_eq!(archives["path"], "~/.local/state/worktree/archives/alpha");
    assert!((MIB..2 * MIB).contains(&bytes(archives)), "{archives:#}");
    let shared = row(&data, None, "shared build target");
    assert_eq!(shared["path"], "~/.cache/example-org-target/alpha");
    row(&data, None, "rustup toolchains");
    row(&data, None, "agent scratch");
    let small = row(&data, None, "smaller /var/tmp entries");
    assert_eq!(small["count"], 2, "{small:#}");

    let rows = disk["rows"].as_array().expect("rows");
    assert!(
        !rows.iter().any(|row| row.to_string().contains("stranger")),
        "a repository without a checkout is not tracked: {data:#}"
    );
    let sizes: Vec<u64> = rows.iter().map(bytes).collect();
    assert!(
        sizes.windows(2).all(|pair| pair[0] >= pair[1]),
        "rows are largest first: {sizes:?}"
    );
    assert_eq!(disk["total_bytes"], sizes.iter().sum::<u64>(), "{data:#}");
    assert!(
        !data.to_string().contains(&home),
        "the disk rows print the home directory in full: {data:#}"
    );

    let page = request(port, "GET", "/", None).body;
    let at = |text: &str| {
        page.find(text)
            .unwrap_or_else(|| panic!("the page lacks {text:?}:\n{page}"))
    };
    assert!(at("Disk waste") < at("build cache in idle managed trees"));
    assert!(at("build cache in idle managed trees") < at("target/ of the checkout"));
    assert!(at("target/ of the checkout") < at("agent scratch"));
    at("total 5.0 GiB");
    at(&format!("measured {measured_at}"));
}

#[test]
fn the_disk_measurement_is_reused_for_ten_minutes() {
    let case = Case::new("reuse");
    let defaults = Sources::new(case.home.clone(), case.home.join("example-org/conductor"));
    assert_eq!(defaults.measure_every, Duration::from_secs(600));
    case.waste();
    let port = start(case.sources());
    let first = measured(port, |_| true);
    let what = "build cache in idle managed trees";
    assert_eq!(bytes(row(&first, Some("alpha"), what)), 5 * GIB);

    case.sweep(7 * GIB);
    for _ in 0..3 {
        let again = data(port);
        assert_eq!(
            again["disk"]["measured_at"], first["disk"]["measured_at"],
            "within ten minutes the measurement is reused: {again:#}"
        );
        assert_eq!(bytes(row(&again, Some("alpha"), what)), 5 * GIB);
        std::thread::sleep(Duration::from_millis(200));
    }

    let mut sources = case.sources();
    sources.measure_every = Duration::ZERO;
    let port = start(sources);
    let before = measured(port, |_| true);
    case.sweep(9 * GIB);
    measured(port, |data| {
        data["disk"]["measured_at"] != before["disk"]["measured_at"]
            && bytes(row(data, Some("alpha"), what)) == 9 * GIB
    });
}

#[test]
fn a_slow_measurement_does_not_hold_the_page_and_a_timed_out_one_says_so() {
    let case = Case::new("slow");
    let mut sources = case.sources();
    sources.sweep = ["sleep", "20"].map(OsString::from).to_vec();
    sources.measure_timeout = Duration::from_secs(3);
    let port = start(sources);
    let first = data(port);
    assert_eq!(
        first["disk"]["measured_at"],
        Value::Null,
        "the page answered before the measurement ended: {first:#}"
    );
    assert_eq!(first["disk"]["measuring"], true, "{first:#}");
    assert!(
        request(port, "GET", "/", None)
            .body
            .contains("measuring, no measurement yet")
    );

    let data = measured(port, |_| true);
    let cache = row(&data, None, "build cache in idle managed trees");
    assert_eq!(cache["bytes"], Value::Null, "{cache:#}");
    assert!(
        cache["state"]
            .as_str()
            .is_some_and(|state| state.starts_with("timed out after 3 s")),
        "{cache:#}"
    );
}

/// Kills and reaps the child when dropped, so a failing case leaves no server running.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn the_binary_listens_on_a_free_port_of_127_0_0_1_and_says_so_first() {
    let case = Case::new("binary");
    // The server measures the disk waste under its home from the start; the fixture home keeps
    // that measurement off the real one.
    let child = common::conductor(&case.home)
        .args(["dashboard", "serve", "--port", "0", "--root"])
        .arg(case.home.join("example-org/conductor"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start conductor dashboard serve");
    let mut server = Server(child);
    let stdout = server
        .0
        .stdout
        .take()
        .expect("the server's standard output");
    let (lines, line) = mpsc::channel();
    std::thread::spawn(move || {
        let mut first = String::new();
        let _ = BufReader::new(stdout).read_line(&mut first);
        let _ = lines.send(first);
    });
    let first = line
        .recv_timeout(Duration::from_secs(30))
        .expect("the server prints a first line");
    let port: u16 = first
        .trim_end()
        .strip_prefix("listening on http://127.0.0.1:")
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("the first line is {first:?}"));
    assert_ne!(port, 0);
    assert_eq!(request(port, "POST", "/", None).status, 405);
    assert_eq!(request(port, "GET", "/nope", None).status, 404);
    drop(server);
}
