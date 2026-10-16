//! `story:live-dashboard`: `conductor dashboard serve`, a live, read-only page of what every
//! session works on; `story:dashboard-service`: exited sessions and the watch's lines;
//! `story:disk-waste`: the disk waste across the tracked repositories; `story:dashboard-spa`: the
//! page is the Vue app embedded from `dashboard/dist/`, and every fact it shows is asserted here
//! on `/data.json` (how the app writes each one is the app's own Vitest suite, `dashboard/tests/`).
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
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use conductor_cli::dashboard::{self, Sources};
use conductor_cli::disk;
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
            context_handover: 300_000,
            health: health_view::sources(&self.home),
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

/// Writes `text` to `path` through a sibling file and a rename, so a measurement reading `path`
/// while the test rewrites it sees the old file or the new one, never a partial one.
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    let partial = path.with_extension("partial");
    fs::write(&partial, text).expect("write a case file");
    fs::rename(&partial, path).expect("move the case file into place");
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

/// The app's build, `crates/conductor/dashboard/dist/`, which the binary embeds.
fn dist() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("dashboard/dist")
}

/// The value of the header `name` in `answer`'s head, if it has one.
fn header<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    answer.head.split("\r\n").skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

/// The path `index.html` names in its first `<attribute>="/assets/…<extension>"`.
fn asset(index: &str, attribute: &str, extension: &str) -> String {
    let start = format!("{attribute}=\"/assets/");
    index
        .split(start.as_str())
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(name, _)| name))
        .find(|name| name.ends_with(extension))
        .map(|name| format!("/assets/{name}"))
        .unwrap_or_else(|| panic!("index.html names no {extension} asset:\n{index}"))
}

/// The policy `story:dashboard-spa` sets on every answer.
const POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'";

/// `story:dashboard-spa`: `GET /` answers the app's `index.html` as the build wrote it, which
/// loads its script and its style from this server and carries no data of its own; since
/// `story:decisions-batch-answer` it carries one thing more, the token a batch of answers is taken
/// with, as `<meta name="conductor-token" content="<64 hex digits>">` before `</head>`.
#[test]
fn get_slash_answers_the_embedded_index_html() {
    let case = Case::new("index");
    let home = case.home.display().to_string();
    let port = start(case.sources());
    let index = request(port, "GET", "/", None);
    assert_eq!(index.status, 200, "{}", index.body);
    assert_eq!(
        header(&index, "content-type"),
        Some("text/html; charset=utf-8"),
        "{}",
        index.head
    );
    let built = fs::read_to_string(dist().join("index.html")).expect("read the built index.html");
    let opening = r#"<meta name="conductor-token" content=""#;
    let token: String = index
        .body
        .find(opening)
        .map(|at| {
            index.body[at + opening.len()..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        token.len(),
        64,
        "the page carries no token:\n{}",
        index.body
    );
    let head_end = built
        .find("</head>")
        .expect("the built index.html has a head");
    let expected = format!(
        r#"{}<meta name="conductor-token" content="{token}">{}"#,
        &built[..head_end],
        &built[head_end..]
    );
    assert_eq!(index.body, expected, "GET / is not the embedded index.html");
    assert!(
        index.body.contains("<div id=\"app\">"),
        "index.html mounts no app:\n{}",
        index.body
    );
    for script in index.body.split("<script").skip(1) {
        let tag = script.split_once('>').map_or(script, |(tag, _)| tag);
        assert!(
            tag.contains("src=\"/assets/"),
            "an inline script, which the policy refuses: <script{tag}>"
        );
    }
    for hidden in ["<style", "http://", "https://", "alpha", &home] {
        assert!(
            !index.body.contains(hidden),
            "index.html holds {hidden:?}:\n{}",
            index.body
        );
    }
}

/// `story:dashboard-spa`: each file of the build answers at its path with its content type and
/// the bytes the build wrote.
#[test]
fn each_asset_answers_with_its_content_type() {
    let case = Case::new("assets");
    let port = start(case.sources());
    let index = request(port, "GET", "/", None).body;
    for (path, content_type) in [
        (asset(&index, "src", ".js"), "text/javascript"),
        (asset(&index, "href", ".css"), "text/css"),
    ] {
        let answer = request(port, "GET", &path, None);
        assert_eq!(answer.status, 200, "GET {path}: {}", answer.head);
        assert_eq!(
            header(&answer, "content-type"),
            Some(content_type),
            "GET {path}: {}",
            answer.head
        );
        let built = fs::read_to_string(dist().join(path.trim_start_matches('/')))
            .unwrap_or_else(|error| panic!("read the built {path}: {error}"));
        assert_eq!(answer.body, built, "GET {path} is not the embedded file");
    }
}

/// `story:dashboard-spa`: the app, its data and every refusal carry the one policy.
#[test]
fn every_answer_carries_the_content_security_policy() {
    let case = Case::new("policy");
    let port = start(case.sources());
    let index = request(port, "GET", "/", None).body;
    let script = asset(&index, "src", ".js");
    let style = asset(&index, "href", ".css");
    let own = format!("127.0.0.1:{port}");
    for (method, path, host, status) in [
        ("GET", "/", own.as_str(), 200),
        ("GET", script.as_str(), own.as_str(), 200),
        ("GET", style.as_str(), own.as_str(), 200),
        ("GET", "/data.json", own.as_str(), 200),
        ("GET", "/nope", own.as_str(), 404),
        ("POST", "/", own.as_str(), 405),
        ("GET", "/", "attacker.example", 403),
    ] {
        let answer = request(port, method, path, Some(host));
        assert_eq!(answer.status, status, "{method} {path} for {host}");
        assert_eq!(
            header(&answer, "content-security-policy"),
            Some(POLICY),
            "{method} {path} for {host}: {}",
            answer.head
        );
    }
}

#[test]
fn only_get_of_the_app_and_its_data_is_answered() {
    let case = Case::new("methods");
    let port = start(case.sources());
    let script = asset(&request(port, "GET", "/", None).body, "src", ".js");
    let refused = request(port, "POST", "/", None);
    assert_eq!(refused.status, 405, "POST /");
    assert!(
        refused.head.contains("Allow: GET"),
        "POST / names no allowed method: {}",
        refused.head
    );
    assert_eq!(
        request(port, "POST", &script, None).status,
        405,
        "POST {script}"
    );
    assert_eq!(request(port, "POST", "/data.json", None).status, 405);
    assert_eq!(request(port, "DELETE", "/data.json", None).status, 405);
    assert_eq!(request(port, "GET", "/nope", None).status, 404);
    assert_eq!(request(port, "POST", "/nope", None).status, 404);
    assert_eq!(request(port, "GET", "/../data.json", None).status, 404);
    // Paths outside the app's build: the crate's own files, its sources, a way up out of it.
    for outside in [
        "/assets/nope.js",
        "/assets/../index.html",
        "/./index.html",
        "//index.html",
        "/src/main.ts",
        "/package.json",
        "/dashboard/dist/index.html",
        "/../Cargo.toml",
        "/assets/",
    ] {
        assert_eq!(
            request(port, "GET", outside, None).status,
            404,
            "GET {outside}"
        );
    }
    assert_eq!(
        request(port, "GET", "/data.json", Some("attacker.example")).status,
        403,
        "a request for another host is refused, so DNS rebinding cannot read the data"
    );
    assert_eq!(
        request(port, "GET", &script, Some("attacker.example")).status,
        403,
        "the app is refused to another host too"
    );
    assert_eq!(
        request(port, "GET", "/", Some(&format!("localhost:{port}"))).status,
        200
    );
}

/// Every key path of `fixture` that `served` does not have at the same place, or the other way
/// round: the keys of each pair of objects, and of the first items of each pair of lists both of
/// which hold one. A null on one side and a value on the other is not compared further.
fn key_drift(at: &str, fixture: &Value, served: &Value, out: &mut Vec<String>) {
    match (fixture, served) {
        (Value::Object(fixture), Value::Object(served)) => {
            for key in fixture.keys().filter(|key| !served.contains_key(*key)) {
                out.push(format!("{at}.{key}: in the fixture, not served"));
            }
            for key in served.keys().filter(|key| !fixture.contains_key(*key)) {
                out.push(format!("{at}.{key}: served, not in the fixture"));
            }
            for (key, value) in fixture {
                if let Some(other) = served.get(key) {
                    key_drift(&format!("{at}.{key}"), value, other, out);
                }
            }
        }
        (Value::Array(fixture), Value::Array(served)) => {
            if let (Some(fixture), Some(served)) = (fixture.first(), served.first()) {
                key_drift(&format!("{at}[0]"), fixture, served, out);
            }
        }
        _ => {}
    }
}

/// `story:dashboard-spa`: the fixture the app's tests render, `dashboard/src/fixtures/data.json`,
/// has the keys `/data.json` serves, so the panels are tested on the real shape. The case fills
/// every part the fixture fills: sessions with a dispatch, the watch's three files, red CI,
/// decisions and a disk measurement with rows.
#[test]
fn the_app_fixture_has_the_keys_data_json_serves() {
    let case = Case::new("fixture");
    case.waste();
    let watch = case.home.join(".cache/conductor-watch");
    let today = OffsetDateTime::now_utc().date();
    write(&watch.join("limit.new"), "Usage limit reached\talpha\n");
    write(
        &watch.join("limit.seen"),
        &format!("{today} Weekly limit reached\n"),
    );
    write(&watch.join("context-reported"), "s2\n");
    let port = start(case.sources());
    let served = measured(port, |data| {
        data["disk"]["rows"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
            && !data["usage"]["measured_at"].is_null()
    });
    let fixture: Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("dashboard/src/fixtures/data.json"),
        )
        .expect("read the app's fixture"),
    )
    .expect("the fixture is JSON");
    for (key, filled) in [
        ("sessions", served["sessions"][0]["dispatch"].is_object()),
        ("watch", served["watch"]["context"][0].is_object()),
        ("red_ci", served["red_ci"][0].is_object()),
        (
            "awaiting_operator",
            served["awaiting_operator"][0].is_object(),
        ),
        ("disk", served["disk"]["rows"][0].is_object()),
    ] {
        assert!(filled, "the case serves no {key} to compare: {served:#}");
    }
    let mut drift = Vec::new();
    key_drift("", &fixture, &served, &mut drift);
    assert!(
        drift.is_empty(),
        "the fixture drifted from /data.json:\n{}\nserved: {served:#}",
        drift.join("\n")
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
    // The row the app shows for it: name, kind and state, below the live ones.
    let aardvark = &sessions[2];
    assert_eq!(
        [&aardvark["name"], &aardvark["kind"], &aardvark["state"]],
        [&json!("aardvark"), &json!("background"), &json!("exited")],
        "{data:#}"
    );
    assert_eq!(aardvark["status"], Value::Null, "{data:#}");
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
    // The app writes these as `usage limit: <text>; 2 sessions: alpha conductor` and `context:
    // conductor over 300k tokens (session s2)` (dashboard/tests/watch.spec.ts).
    assert_eq!(data["context_handover"], 300_000, "{data:#}");
    assert!(!data.to_string().contains("Old limit"), "{data:#}");
}

/// The context line names the hand-over threshold the instance configures
/// (`thresholds.context_handover`), not a fixed 300k (story:session-tasks-follow-prefix).
#[test]
fn the_context_line_names_the_configured_hand_over_threshold() {
    let case = Case::new("context-threshold");
    let config = conductor_cli::config::parse(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: acme\n\
         \x20   sources: [{github: acme}]\n\
         \x20   checkouts: {root: ~/checkouts, trees: ~/trees}\n\
         \x20   thresholds: {context_handover: 150k}\n",
        &case.home,
    )
    .expect("the config converts");
    let of = Sources::of(case.home.clone(), None, &config.instances[0]);
    assert_eq!(of.context_handover, 150_000, "the instance's threshold");
    let root = case.home.join("example-org/conductor");
    assert_eq!(
        Sources::new(case.home.clone(), root).context_handover,
        300_000,
        "the built-in instance's threshold"
    );

    let mut sources = case.sources();
    sources.context_handover = of.context_handover;
    let port = start(sources);
    write(
        &case.home.join(".cache/conductor-watch/context-reported"),
        "s2\n",
    );
    let data = data(port);
    // The app names this threshold in the context line, `150k` (dashboard/tests/watch.spec.ts).
    assert_eq!(data["context_handover"], 150_000, "{data:#}");
    assert_eq!(
        data["watch"]["context"],
        json!([{"session": "s2", "name": "conductor"}]),
        "{data:#}"
    );
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

    // The app's heading, `Sessions under ~/acme (1 live, 0 exited)`, from these.
    assert_eq!(data["reading"]["checkouts"], "~/acme", "{data:#}");
    let exited: Vec<&Value> = data["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .map(|row| &row["exited"])
        .collect();
    assert_eq!(exited, [&json!(false)], "1 live, 0 exited: {data:#}");
}

/// `story:dashboard-worktree-repo`: the disk measurement's `worktree sweep --dry-run` and
/// `worktree gc --dry-run` name as `--repo` the first git checkout under the checkouts root, the
/// one `worktree prune-archives` is given (`story:gc-archive-profile`), never the records
/// directory, which no `worktree` profile contains.
///
/// The built binary serves an instance a config file names, its records apart from its checkouts
/// root; a fake `worktree` first on `PATH` appends `<verb><TAB><--repo>` of each call to
/// `repos.log`. Under the checkouts root `aaa` sorts first but is no git checkout, so `alpha` is
/// the one named.
#[test]
fn the_worktree_dry_runs_name_a_checkout_under_the_checkouts_root_not_the_records() {
    let case = Case::new("worktree-repo");
    let home = case.home.clone();
    let dir = home.parent().expect("the case's directory").to_path_buf();
    let checkouts = home.join("checkouts");
    let records = home.join("records");
    fs::create_dir_all(checkouts.join("aaa")).expect("create a directory that is no checkout");
    for name in ["alpha", "beta"] {
        write(
            &checkouts.join(name).join(".git/HEAD"),
            "ref: refs/heads/main\n",
        );
    }
    fs::create_dir_all(records.join("dispatches")).expect("create dispatches/");
    fs::create_dir_all(records.join("decisions")).expect("create decisions/");
    let config = dir.join("conductor.yaml");
    write(
        &config,
        &format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: acme\n\
             \x20   sources: [{{github: acme}}]\n\
             \x20   checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            trees = home.join("trees"),
            state = home.join("state"),
            cache = home.join("cache"),
        ),
    );

    // The library builds the two calls with the checkout; with no git checkout under the root,
    // with the records directory, as before.
    let parsed = conductor_cli::config::parse(
        &fs::read_to_string(&config).expect("read the config"),
        &home,
    )
    .expect("the config converts");
    let repo_of = |command: &[OsString]| {
        command
            .iter()
            .position(|arg| arg == "--repo")
            .and_then(|at| command.get(at + 1))
            .map(PathBuf::from)
    };
    let sources = Sources::of(home.clone(), None, &parsed.instances[0]);
    assert_eq!(repo_of(&sources.sweep), Some(checkouts.join("alpha")));
    assert_eq!(repo_of(&sources.gc), Some(checkouts.join("alpha")));
    let bare = Sources::new(home.clone(), records.clone());
    assert_eq!(
        repo_of(&bare.sweep),
        Some(records.clone()),
        "no checkout under the built-in checkouts root of the case's home"
    );
    assert_eq!(repo_of(&bare.gc), Some(records.clone()));

    // The binary's measurement runs both, and `worktree` sees the checkout in each.
    let bin = dir.join("bin");
    let log = dir.join("repos.log");
    let program = |name: &str, script: &str| {
        let path = bin.join(name);
        write(&path, script);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("make the fake executable");
    };
    program("claude", "#!/bin/sh\nprintf '[]\\n'\n");
    program(
        "worktree",
        &format!(
            "#!/bin/sh\n\
             verb=$1; repo=; prev=\n\
             for arg; do\n\
             \x20 if [ \"$prev\" = --repo ]; then repo=$arg; fi\n\
             \x20 prev=$arg\n\
             done\n\
             printf '%s\\t%s\\n' \"$verb\" \"$repo\" >> '{log}'\n\
             case \"$verb\" in\n\
             sweep) printf '{{\"version\":4,\"ok\":true,\"items\":[]}}\\n' ;;\n\
             gc) printf '{{\"version\":4,\"ok\":true,\"assessments\":[]}}\\n' ;;\n\
             *) printf '{{\"version\":4,\"ok\":true}}\\n' ;;\n\
             esac\n",
            log = log.display(),
        ),
    );
    let child = common::conductor(&home)
        .args(["dashboard", "serve", "--port", "0"])
        .env(conductor_cli::config::CONFIG_VARIABLE, &config)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .current_dir(&records)
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
    // Read on a thread of its own, so a server that never prints fails the case instead of
    // hanging it; `server` kills and reaps the child on every way out.
    let (lines, line) = mpsc::channel();
    std::thread::spawn(move || {
        let mut first = String::new();
        let _ = BufReader::new(stdout).read_line(&mut first);
        let _ = lines.send(first);
    });
    let first = line
        .recv_timeout(Duration::from_secs(30))
        .expect("the server prints a first line within 30 s");
    let port: u16 = first
        .trim_end()
        .strip_prefix("listening on http://127.0.0.1:")
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("the first line is {first:?}"));
    assert_eq!(request(port, "GET", "/data.json", None).status, 200);

    let deadline = Instant::now() + Duration::from_secs(60);
    let calls = loop {
        let logged = fs::read_to_string(&log).unwrap_or_default();
        let mut calls: Vec<(String, PathBuf)> = logged
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .filter(|(verb, _)| ["gc", "sweep"].contains(verb))
            .map(|(verb, repo)| (verb.to_owned(), PathBuf::from(repo)))
            .collect();
        calls.sort();
        calls.dedup();
        if calls.iter().any(|(verb, _)| verb == "gc")
            && calls.iter().any(|(verb, _)| verb == "sweep")
        {
            break calls;
        }
        assert!(
            Instant::now() < deadline,
            "worktree was not called for both dry runs within 60 s; {} holds:\n{logged}",
            log.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    drop(server);
    let alpha = checkouts.join("alpha");
    assert_eq!(
        calls,
        [
            ("gc".to_owned(), alpha.clone()),
            ("sweep".to_owned(), alpha)
        ],
        "each dry run names the checkout, not the records directory {}",
        records.display()
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

    // The order the app's table shows them in, and the total it writes as `total 5.0 GiB`.
    let at = |what: &str| {
        rows.iter()
            .position(|row| row["what"] == what)
            .unwrap_or_else(|| panic!("no disk row {what:?}: {data:#}"))
    };
    assert!(at("build cache in idle managed trees") < at("target/ of the checkout"));
    assert!(at("target/ of the checkout") < at("agent scratch"));
    let total = disk["total_bytes"].as_u64().expect("total_bytes");
    assert_eq!(
        format!("{:.1}", total as f64 / GIB as f64),
        "5.0",
        "{data:#}"
    );
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

/// The bytes `path` itself takes on disk, by its own `lstat`: `st_blocks * 512`.
fn allocated(path: &Path) -> u64 {
    fs::symlink_metadata(path)
        .unwrap_or_else(|error| panic!("lstat {}: {error}", path.display()))
        .blocks()
        * 512
}

/// `story:portable-tree-sizes`: a known tree with a second name for one file and a link to a
/// large file outside it measures the allocated bytes of each of its inodes once, and follows no
/// link; overlapping paths count nothing twice.
#[test]
fn a_known_tree_measures_each_inode_once_and_follows_no_link() {
    let case = Case::new("tree-size");
    let root = case.home.join("tree");
    let sub = root.join("sub");
    fill(&root.join("a.bin"), 64 * 1024);
    fill(&sub.join("b.bin"), 128 * 1024);
    fs::hard_link(root.join("a.bin"), sub.join("a-again.bin")).expect("link a second name");
    let outside = case.home.join("outside/big.bin");
    fill(&outside, 4 * MIB);
    std::os::unix::fs::symlink(&outside, sub.join("big")).expect("link the outside file");

    let expected: u64 = [
        root.clone(),
        sub.clone(),
        root.join("a.bin"),
        sub.join("b.bin"),
        sub.join("big"),
    ]
    .iter()
    .map(PathBuf::as_path)
    .map(allocated)
    .sum();
    let size = disk::tree_size(std::slice::from_ref(&root), None).expect("no deadline");
    assert!(size.unreadable.is_empty(), "{size:?}");
    assert_eq!(size.allocated, expected, "{size:?}");
    assert!(
        (192 * 1024..MIB).contains(&size.allocated),
        "the link to the outside file is not followed: {size:?}"
    );

    let overlapping = disk::tree_size(
        &[
            root.clone(),
            sub.clone(),
            sub.join("b.bin"),
            sub.join("a-again.bin"),
        ],
        None,
    )
    .expect("no deadline");
    assert_eq!(overlapping.allocated, expected, "{overlapping:?}");
}

#[test]
fn a_tree_size_names_what_it_cannot_read_and_stops_at_its_deadline() {
    let case = Case::new("tree-size-unreadable");
    let root = case.home.join("tree");
    fill(&root.join("open/a.bin"), 64 * 1024);
    let missing = root.join("not-there");
    let size = disk::tree_size(&[root.clone(), missing.clone()], None).expect("no deadline");
    assert_eq!(
        size.unreadable
            .iter()
            .map(|unreadable| &unreadable.path)
            .collect::<Vec<_>>(),
        [&missing],
        "{size:?}"
    );
    assert!(size.allocated >= 64 * 1024, "{size:?}");

    let locked = root.join("locked");
    fill(&locked.join("hidden.bin"), 64 * 1024);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock a directory");
    // A privileged reader reads it anyway; the case then has nothing unreadable to show.
    let readable = fs::read_dir(&locked).is_ok();
    let size = disk::tree_size(std::slice::from_ref(&root), None).expect("no deadline");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlock it again");
    if !readable {
        assert_eq!(size.unreadable.len(), 1, "{size:?}");
        assert_eq!(size.unreadable[0].path, locked, "{size:?}");
    }

    let timed_out = disk::tree_size(std::slice::from_ref(&root), Some(Instant::now()));
    assert!(
        matches!(timed_out, Err(disk::TimedOut)),
        "a passed deadline ends the walk: {timed_out:?}"
    );
}

/// `story:portable-tree-sizes`: no product source spawns `du`, whose `-B1` another `du` (BSD's)
/// refuses; read the way `free_space.rs` reads for `df`.
#[test]
fn no_product_source_spawns_du() {
    fn files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read a directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                files(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root");
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("crates")).expect("read crates/") {
        let src = entry.expect("a crate").path().join("src");
        if src.is_dir() {
            files(&src, &mut paths);
        }
    }
    assert!(!paths.is_empty(), "no source file was read");
    let mut offending = Vec::new();
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            if line.contains("\"du\"") || line.contains("\"du ") || line.contains("\"-B1\"") {
                offending.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    at + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(offending.is_empty(), "{offending:#?}");
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
    // The app says `measuring, no measurement yet` for these (dashboard/tests/disk-waste.spec.ts).
    assert_eq!(first["disk"]["measuring"], true, "{first:#}");
    assert_eq!(first["disk"]["rows"], json!([]), "{first:#}");

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

/// `story:dashboard-health`: `GET /api/health`, the health view's data: doctor's rows from a
/// cache, the newest snapshot with its collector runs and member failures, each failure with its
/// class and fix, and the free disk against the instance's floors with every disk-waste row.
///
/// Doctor runs over [`HealthSources`](conductor_cli::dashboard::HealthSources): its `PATH` is the
/// case's `bin/`, so a fake `aep` there counts its runs, and its config is a file of the case,
/// never the operator's. The store is the case's `state/`, filled through the snapshot driver
/// ([`snapshot::take_in`](conductor_cli::snapshot::take_in)) with collectors of the test's own.
mod health_view {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicI64, Ordering};

    use conductor_cli::collect::Collector;
    use conductor_cli::config;
    use conductor_cli::dashboard::{FAILURE_FIXES, HealthSources};
    use conductor_cli::snapshot::{self, Taken};
    use conductor_model::observation::{RecordMemberFailure, RepositoryName};

    use super::*;

    /// The health sources every case of this file serves: the store under the case's `state/`,
    /// a config file of the case that is not there unless a case writes it, an empty `bin/` as
    /// doctor's `PATH`, and the free space of `/`.
    pub(super) fn sources(home: &Path) -> HealthSources {
        let mut health =
            HealthSources::new(home.to_path_buf(), &home.join("example-org/conductor"));
        health.state = home.join("state");
        health.config = Some(home.join("conductor.yaml"));
        health.path = Some(home.join("bin").into_os_string());
        health
    }

    /// `GET /api/health`, as JSON.
    fn health(port: u16) -> Value {
        let answer = request(port, "GET", "/api/health", None);
        assert_eq!(answer.status, 200, "GET /api/health: {}", answer.body);
        assert!(
            answer
                .head
                .to_ascii_lowercase()
                .contains("content-type: application/json"),
            "GET /api/health is not JSON: {}",
            answer.head
        );
        serde_json::from_str(&answer.body).expect("/api/health is JSON")
    }

    /// `/api/health` once `until` holds, polled for at most 60 s.
    fn health_until(port: u16, until: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let health = health(port);
            if until(&health) {
                return health;
            }
            assert!(Instant::now() < deadline, "not so in 60 s: {health:#}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn taken(health: &Value) -> bool {
        !health["doctor"]["taken_at"].is_null()
    }

    impl Case {
        /// An `aep` in the case's `bin/`, doctor's `PATH`, that adds a line to `aep.runs` as it
        /// starts, waits `delay` and prints `aep 9.9.9`; the path of `aep.runs`.
        fn fake_aep(&self, delay: Duration) -> PathBuf {
            let runs = self.home.join("aep.runs");
            let aep = self.home.join("bin/aep");
            write(
                &aep,
                &format!(
                    "#!/bin/sh\necho run >> '{}'\nsleep {}\necho 'aep 9.9.9'\n",
                    runs.display(),
                    delay.as_secs()
                ),
            );
            let mode = <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755);
            fs::set_permissions(&aep, mode).expect("make aep run");
            runs
        }

        fn state(&self) -> PathBuf {
            self.home.join("state")
        }

        /// A snapshot of the case's store whose one collector, `blockers`, records the member
        /// `beta` as a failure with `line`.
        fn failure(&self, line: &'static str) -> Taken {
            snapshot::take_in(&self.state(), &[failing(line)])
                .expect("the driver ends the snapshot")
        }
    }

    /// The collector `blockers`, which records the member `beta` as a failure with `line`.
    fn failing(line: &'static str) -> Collector {
        Collector::new("blockers", move |recorder| {
            let snapshot_id = recorder.snapshot().clone();
            recorder.member_failure(RecordMemberFailure {
                snapshot_id,
                member: RepositoryName("beta".to_owned()),
                collector: "blockers".to_owned(),
                error_line: line.to_owned(),
            })?;
            Ok(())
        })
    }

    fn runs(file: &Path) -> usize {
        fs::read_to_string(file).map_or(0, |text| text.lines().count())
    }

    fn instant(value: &Value) -> OffsetDateTime {
        OffsetDateTime::parse(value.as_str().expect("an instant"), &Rfc3339)
            .unwrap_or_else(|error| panic!("{value} is no RFC 3339 instant: {error}"))
    }

    #[test]
    fn doctor_is_taken_once_at_start_and_served_from_its_cache() {
        let case = Case::new("health-doctor-cached");
        let count = case.fake_aep(Duration::ZERO);
        let port = start(case.sources());
        let first = health_until(port, taken);
        let second = health(port);
        assert_eq!(second["doctor"], first["doctor"], "{second:#}");
        assert_eq!(runs(&count), 1, "doctor ran again for a request");
        let rows = first["doctor"]["rows"].as_array().expect("doctor's rows");
        assert!(
            rows.iter().any(|row| row == "aep: present, aep 9.9.9"),
            "doctor's rows are its command's: {first:#}"
        );
        assert!(
            first["doctor"]["verdict"]
                .as_str()
                .is_some_and(|verdict| verdict == "ready" || verdict.starts_with("not ready: ")),
            "{first:#}"
        );
    }

    /// The fixture clock of [`doctor_is_taken_again_after_its_interval_off_the_request_path`],
    /// in Unix seconds.
    static REFRESH_NOW: AtomicI64 = AtomicI64::new(1_791_000_000);

    fn refresh_clock() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(REFRESH_NOW.load(Ordering::SeqCst))
            .expect("a fixture instant")
    }

    #[test]
    fn doctor_is_taken_again_after_its_interval_off_the_request_path() {
        let case = Case::new("health-doctor-refresh");
        let count = case.fake_aep(Duration::from_secs(3));
        let mut sources = case.sources();
        sources.health.clock = refresh_clock;
        sources.health.doctor_every = Duration::from_secs(60);
        let port = start(sources);
        let first = health_until(port, taken);
        assert_eq!(runs(&count), 1);

        REFRESH_NOW.fetch_add(61, Ordering::SeqCst);
        let asked = Instant::now();
        let after = health(port);
        assert!(
            asked.elapsed() < Duration::from_secs(2),
            "the request waited for doctor: {:?}",
            asked.elapsed()
        );
        assert_eq!(after["doctor"]["taken_at"], first["doctor"]["taken_at"]);
        assert_eq!(after["doctor"]["rows"], first["doctor"]["rows"]);

        let deadline = Instant::now() + Duration::from_secs(30);
        while runs(&count) < 2 {
            assert!(Instant::now() < deadline, "doctor was not taken again");
            std::thread::sleep(Duration::from_millis(50));
        }
        let renewed = health_until(port, |health| {
            health["doctor"]["taken_at"] != first["doctor"]["taken_at"]
        });
        assert!(
            instant(&renewed["doctor"]["taken_at"]) > instant(&first["doctor"]["taken_at"]),
            "{renewed:#}"
        );
        assert_eq!(runs(&count), 2);
    }

    #[test]
    fn the_newest_snapshot_is_served_when_it_failed_with_its_reason() {
        let case = Case::new("health-failed");
        let quiet = Collector::new("repositories", |_| Ok(()));
        snapshot::take_in(&case.state(), &[quiet]).expect("an older snapshot");
        let refused = Collector::new("blockers", |_| anyhow::bail!("fetch refused"));
        let failed = snapshot::take_in(&case.state(), &[refused]).expect("a failed snapshot");
        let port = start(case.sources());
        let health = health(port);
        let snapshot = &health["snapshot"];
        assert_eq!(
            snapshot["snapshot_id"], failed.snapshot_id.0.0,
            "{health:#}"
        );
        assert_eq!(snapshot["state"], "Failed", "{health:#}");
        assert_eq!(snapshot["failure_reason"], "blockers: fetch refused");
    }

    #[test]
    fn the_newest_snapshot_is_served_while_it_is_still_collecting() {
        let case = Case::new("health-collecting");
        let quiet = Collector::new("repositories", |_| Ok(()));
        snapshot::take_in(&case.state(), &[quiet]).expect("an older snapshot");
        let (started, collecting) = mpsc::channel();
        let (release, held) = mpsc::channel::<()>();
        let state = case.state();
        let driver = std::thread::spawn(move || {
            let held = Mutex::new(held);
            let waiting = Collector::new("blockers", move |_| {
                started.send(()).expect("say the collector started");
                held.lock()
                    .expect("the release")
                    .recv_timeout(Duration::from_secs(60))
                    .expect("released");
                Ok(())
            });
            snapshot::take_in(&state, &[waiting]).expect("the snapshot ends")
        });
        collecting
            .recv_timeout(Duration::from_secs(30))
            .expect("the collector started");
        let port = start(case.sources());
        let health = health(port);
        release.send(()).expect("release the collector");
        let taken = driver.join().expect("the driver ends");
        assert_eq!(health["snapshot"]["state"], "Collecting", "{health:#}");
        assert_eq!(health["snapshot"]["snapshot_id"], taken.snapshot_id.0.0);
        assert_eq!(health["snapshot"]["failure_reason"], Value::Null);
    }

    #[test]
    fn the_collector_runs_and_member_failures_are_the_newest_snapshots_rows() {
        let case = Case::new("health-rows");
        case.failure("fatal: an older failure");
        let quiet = Collector::new("repositories", |_| Ok(()));
        let line = "fatal: could not read from remote repository";
        let newest =
            snapshot::take_in(&case.state(), &[quiet, failing(line)]).expect("the newest snapshot");
        let port = start(case.sources());
        let health = health(port);
        assert_eq!(health["snapshot"]["snapshot_id"], newest.snapshot_id.0.0);
        let runs = health["collector_runs"].as_array().expect("collector runs");
        let mut collectors: Vec<&str> = runs
            .iter()
            .map(|run| run["collector"].as_str().expect("a collector"))
            .collect();
        collectors.sort_unstable();
        assert_eq!(collectors, ["blockers", "repositories"], "{health:#}");
        assert!(
            runs.iter()
                .all(|run| run["duration_ms"].is_i64() && run["outcome"] == "Completed"),
            "{health:#}"
        );
        let failures = health["member_failures"]
            .as_array()
            .expect("member failures");
        assert_eq!(failures.len(), 1, "only the newest snapshot's: {health:#}");
        assert_eq!(failures[0]["member"], "beta");
        assert_eq!(failures[0]["collector"], "blockers");
        assert_eq!(failures[0]["error_line"], line);
    }

    /// The class and fix `/api/health` answers for a member failure whose first line is `line`.
    fn classified(name: &str, line: &'static str) -> (String, String) {
        let case = Case::new(name);
        case.failure(line);
        let port = start(case.sources());
        let health = health(port);
        let failure = &health["member_failures"][0];
        assert_eq!(failure["error_line"], line, "{health:#}");
        (
            failure["class"].as_str().expect("a class").to_owned(),
            failure["fix"].as_str().expect("a fix").to_owned(),
        )
    }

    /// The class `name` with the fix the one table in `dashboard.rs` gives it.
    fn class(name: &str) -> (String, String) {
        let (_, _, fix) = FAILURE_FIXES
            .iter()
            .find(|(_, class, _)| *class == name)
            .unwrap_or_else(|| panic!("no fix for {name}"));
        assert!(!fix.is_empty(), "the fix of {name} is empty");
        (name.to_owned(), (*fix).to_owned())
    }

    #[test]
    fn a_prompt_or_a_refused_credential_needs_login() {
        assert_eq!(
            classified(
                "health-class-username",
                "`git fetch --quiet origin` exited 128: fatal: could not read Username for \
                 'https://example.com': terminal prompts disabled"
            ),
            class("needs login")
        );
        assert_eq!(
            classified(
                "health-class-publickey",
                "`git fetch --quiet origin` exited 128: git@example.com: Permission denied \
                 (publickey)."
            ),
            class("needs login")
        );
    }

    #[test]
    fn a_host_that_does_not_resolve_is_unreachable() {
        assert_eq!(
            classified(
                "health-class-host",
                "`git fetch --quiet origin` exited 128: ssh: Could not resolve hostname \
                 git.example.com: Name or service not known"
            ),
            class("unreachable host")
        );
    }

    #[test]
    fn a_command_stopped_at_its_bound_timed_out() {
        assert_eq!(
            classified("health-class-timeout", "`git` gave no answer within 120 s"),
            class("timed out")
        );
    }

    #[test]
    fn an_aep_project_1_store_is_an_old_store() {
        assert_eq!(
            classified(
                "health-class-old-store",
                "error: project document (.engineering/planning/project.json) is not valid: \
                 [unsupported_protocol_version] project.version: this store is aep.project/1, \
                 which this aep no longer reads"
            ),
            class("old store")
        );
    }

    #[test]
    fn any_other_first_line_is_other() {
        assert_eq!(
            classified("health-class-other", "fatal: bad object HEAD"),
            class("other")
        );
    }

    /// The health sources of a fixture config whose instance sets `thresholds.disk_low` to 40 GiB
    /// and `thresholds.disk_admit` to 20 GiB.
    fn configured(home: &Path) -> HealthSources {
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {}, trees: {}}}\n\
             \x20   thresholds: {{disk_low: 40G, disk_admit: 20G}}\n",
            home.join("example-org").display(),
            home.join(".local/state/worktree/trees/example-org")
                .display(),
        );
        let config = config::parse(&text, home)
            .unwrap_or_else(|problems| panic!("the fixture config parses: {problems:?}"));
        let instance = config.instances.into_iter().next().expect("one instance");
        HealthSources::of(home.to_path_buf(), &instance)
    }

    /// The free bytes the file at `path` holds: the fixture's reading.
    fn written_bytes(path: &Path) -> anyhow::Result<u64> {
        Ok(fs::read_to_string(path)?.trim().parse()?)
    }

    #[test]
    fn the_free_disk_is_ok_above_disk_low_low_at_it_and_admit_only_at_disk_admit() {
        let case = Case::new("health-disk");
        let configured = configured(&case.home);
        assert_eq!((configured.disk_low, configured.disk_admit), (40, 20));
        let free = case.home.join("free.txt");
        let mut sources = case.sources();
        sources.health.disk_low = configured.disk_low;
        sources.health.disk_admit = configured.disk_admit;
        sources.health.disk = free.clone();
        sources.health.free_bytes = written_bytes;
        let port = start(sources);
        for (bytes, state) in [
            (40 * GIB + 1, "ok"),
            (40 * GIB, "low"),
            (20 * GIB + 1, "low"),
            (20 * GIB, "admit-only"),
        ] {
            write(&free, &bytes.to_string());
            let health = health(port);
            let disk = &health["disk"];
            assert_eq!(disk["free_bytes"], bytes, "{disk:#}");
            assert_eq!(disk["disk_low_bytes"], 40 * GIB, "{disk:#}");
            assert_eq!(disk["disk_admit_bytes"], 20 * GIB, "{disk:#}");
            assert_eq!(disk["state"], state, "{bytes} bytes free: {disk:#}");
        }
    }

    #[test]
    fn every_instances_fall_is_served() {
        let case = Case::new("health-falls");
        let host = case.home.join("host");
        for (prefix, instance) in [("b", "example-two"), ("a", "example-one")] {
            let file = host.join("falls").join(format!("{prefix}.json"));
            fs::create_dir_all(file.parent().expect("a directory")).expect("falls/");
            fs::write(
                &file,
                format!("{{\"item\":\"disk-{prefix}\",\"instance\":\"{instance}\"}}\n"),
            )
            .expect("a fall");
        }
        let mut sources = case.sources();
        sources.health.host = Some(host);
        let port = start(sources);
        let health = health(port);
        let falls = health["disk"]["falls"].as_array().expect("falls").clone();
        let seen: Vec<(String, String, String)> = falls
            .iter()
            .map(|fall| {
                let text = |key: &str| fall[key].as_str().expect(key).to_owned();
                (text("prefix"), text("instance"), text("item"))
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                (
                    "a".to_owned(),
                    "example-one".to_owned(),
                    "disk-a".to_owned()
                ),
                (
                    "b".to_owned(),
                    "example-two".to_owned(),
                    "disk-b".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn every_disk_waste_row_is_served() {
        let case = Case::new("health-waste");
        for at in 0..85 {
            let entry = format!(".cache/example-org-target/entry-{at:02}/lib.rlib");
            fill(&case.home.join(entry), 4096);
        }
        let port = start(case.sources());
        let health = health_until(port, |health| !health["disk"]["measured_at"].is_null());
        let rows = health["disk"]["rows"].as_array().expect("disk rows");
        assert_eq!(rows.len(), 85, "{health:#}");
        assert!(rows.iter().all(|row| row["what"] == "shared build target"));
    }

    /// Holds the keys of `fixture` to those of `answer` at every level: an object's keys, and
    /// the first item of each list both hold.
    fn same_keys(at: &str, fixture: &Value, answer: &Value) {
        match (fixture, answer) {
            (Value::Object(fixture), Value::Object(answer)) => {
                let keys = |object: &serde_json::Map<String, Value>| {
                    let mut keys: Vec<String> = object.keys().cloned().collect();
                    keys.sort();
                    keys
                };
                assert_eq!(keys(fixture), keys(answer), "the keys of {at}");
                for (key, value) in fixture {
                    same_keys(&format!("{at}.{key}"), value, &answer[key]);
                }
            }
            (Value::Array(fixture), Value::Array(answer)) => {
                if let (Some(fixture), Some(answer)) = (fixture.first(), answer.first()) {
                    same_keys(&format!("{at}[0]"), fixture, answer);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_apps_health_fixture_has_the_keys_the_server_sends() {
        let case = Case::new("health-keys");
        case.fake_aep(Duration::ZERO);
        case.waste();
        case.failure("fatal: could not read Username for 'https://example.com'");
        let port = start(case.sources());
        let health = health_until(port, |health| {
            taken(health) && !health["disk"]["measured_at"].is_null()
        });
        for list in ["collector_runs", "member_failures"] {
            assert!(
                health[list].as_array().is_some_and(|rows| !rows.is_empty()),
                "the case serves {list} rows to hold the fixture to: {health:#}"
            );
        }
        let fixture: Value = serde_json::from_str(
            &fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/dashboard/src/views/__tests__/fixtures/health.json"
            ))
            .expect("read the app's health fixture"),
        )
        .expect("the fixture is JSON");
        same_keys("/api/health", &fixture, &health);
    }

    #[test]
    fn only_get_of_the_health_data_is_answered_with_the_home_written_as_a_tilde() {
        let case = Case::new("health-get");
        let port = start(case.sources());
        assert_eq!(request(port, "POST", "/api/health", None).status, 405);
        assert_eq!(
            request(port, "GET", "/api/health", Some("attacker.example")).status,
            403
        );
        let body = request(port, "GET", "/api/health", None).body;
        assert!(
            !body.contains(&case.home.display().to_string()),
            "the home directory in full: {body}"
        );
    }
}
