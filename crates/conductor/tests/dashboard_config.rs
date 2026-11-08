//! `story:dashboard-config-page`: the dashboard's Configuration page. `GET /api/config` answers
//! the effective config of the instance the server runs, and creates nothing; `POST /api/config`
//! checks or writes the page's staged changes, only from the page's own origin with its token,
//! refuses a result `config validate` would refuse, and for a valid one keeps the old file as a
//! backup, writes the new one and records one class-C decision by the operator with the diff.
//!
//! The cases serve [`dashboard::serve_over`] in this process on a free port of `127.0.0.1`, over
//! a config file and a state directory under this test target's temporary directory; the store is
//! read back with the built binary. Nothing is read from the environment.

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

/// One case's directory: `home/` with the config file and the records, `state/` with the store.
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

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-config")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        fs::write(case.file(), case.text()).expect("write the config file");
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

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    /// The case's config file: one instance, its cadence written, its thresholds not.
    fn text(&self) -> String {
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             \x20   cadence: {{watch: 3m}}\n",
            root = self.home().join("example-org"),
            trees = self.home().join("trees"),
            records = self.records(),
            state = self.state(),
            cache = self.dir.join("cache"),
        )
    }

    /// The backups beside the config file.
    fn backups(&self) -> Vec<PathBuf> {
        fs::read_dir(self.home())
            .expect("read home/")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().ends_with(".bak"))
            .collect()
    }

    /// Every decision of the case's store, as the built binary's `decisions` view lists it; none
    /// when there is no store.
    fn decisions(&self) -> Vec<Value> {
        if !self.state().join("store.sqlite").exists() {
            return Vec::new();
        }
        let output = common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.state())
            .args(["decision", "decisions", "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout)
            .expect("decisions as JSON")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Asserts nothing was written: the file as it was, no backup, no decision.
    #[track_caller]
    fn unchanged(&self) {
        assert_eq!(fs::read_to_string(self.file()).unwrap(), self.text());
        assert!(self.backups().is_empty(), "{:?}", self.backups());
        assert!(self.decisions().is_empty(), "{:?}", self.decisions());
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
        sources.config_file = Some(self.file());
        sources.instance_name = Some("example-org".to_owned());
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

struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {}", self.body))
    }
}

fn exchange(port: u16, raw: &[u8]) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    stream.write_all(raw).expect("send the request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    Answer {
        status,
        body: body.to_owned(),
    }
}

fn get(port: u16, path: &str) -> Answer {
    exchange(
        port,
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

fn token(port: u16) -> String {
    let page = get(port, "/");
    assert_eq!(page.status, 200, "GET /: {}", page.body);
    let marker = r#"<meta name="conductor-token" content=""#;
    let at = page.body.find(marker).expect("the page names its token");
    let rest = &page.body[at + marker.len()..];
    rest[..rest.find('"').expect("the token's closing quote")].to_owned()
}

/// A POST of `body` to `/api/config` with the server's own host and origin, as JSON, each of
/// `headers` replacing the one of its name; `None` leaves it out.
fn post_with(port: u16, body: &Value, headers: &[(&str, Option<String>)]) -> Answer {
    let body = serde_json::to_vec(body).expect("a JSON body");
    let mut named: Vec<(String, Option<String>)> = vec![
        ("Host".to_owned(), Some(format!("127.0.0.1:{port}"))),
        (
            "Origin".to_owned(),
            Some(format!("http://127.0.0.1:{port}")),
        ),
        (
            "Content-Type".to_owned(),
            Some("application/json".to_owned()),
        ),
        ("Content-Length".to_owned(), Some(body.len().to_string())),
        ("Connection".to_owned(), Some("close".to_owned())),
    ];
    for (name, value) in headers {
        match named
            .iter_mut()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
        {
            Some(entry) => entry.1.clone_from(value),
            None => named.push(((*name).to_owned(), value.clone())),
        }
    }
    let mut raw = b"POST /api/config HTTP/1.1\r\n".to_vec();
    for (name, value) in named {
        if let Some(value) = value {
            raw.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
    }
    raw.extend_from_slice(b"\r\n");
    raw.extend_from_slice(&body);
    exchange(port, &raw)
}

fn post(port: u16, body: &Value) -> Answer {
    let token = token(port);
    post_with(port, body, &[("X-Conductor-Token", Some(token))])
}

fn disk_low() -> Value {
    json!({
        "changes": [{"path": "thresholds.disk_low", "value": "200G"}],
        "reason": "the host has a larger disk now",
    })
}

#[test]
fn the_page_reads_every_key_with_its_default_and_creates_nothing() {
    let case = Case::new("view");
    let port = case.start();
    let answer = get(port, "/api/config");
    assert_eq!(answer.status, 200, "{}", answer.body);
    let page = answer.json();
    assert_eq!(page["instance"], "example-org", "{page}");
    let fields = page["fields"].as_array().expect("fields");
    let field = |path: &str| {
        fields
            .iter()
            .find(|field| field["path"] == path)
            .unwrap_or_else(|| panic!("no field {path}: {page}"))
            .clone()
    };
    let watch = field("cadence.watch");
    assert_eq!(watch["value"], "3m");
    assert_eq!(watch["default"], "180s", "{watch}");
    assert_eq!(watch["takes_effect"], "LoopRestart");
    let disk_low = field("thresholds.disk_low");
    assert!(
        disk_low["default"].is_null(),
        "the file writes none: {disk_low}"
    );
    assert_eq!(disk_low["takes_effect"], "WatchTick");
    assert_eq!(field("records")["takes_effect"], "Fixed");
    assert!(
        !case.state().join("store.sqlite").exists(),
        "a view creates no store"
    );
}

#[test]
fn a_post_from_a_foreign_origin_is_refused() {
    let case = Case::new("foreign-origin");
    let port = case.start();
    let token = token(port);
    let answer = post_with(
        port,
        &disk_low(),
        &[
            ("X-Conductor-Token", Some(token)),
            ("Origin", Some("http://example.org".to_owned())),
        ],
    );
    assert_eq!(answer.status, 403, "{}", answer.body);
    case.unchanged();
}

#[test]
fn a_post_naming_a_foreign_host_is_refused() {
    let case = Case::new("foreign-host");
    let port = case.start();
    let token = token(port);
    let answer = post_with(
        port,
        &disk_low(),
        &[
            ("X-Conductor-Token", Some(token)),
            ("Host", Some("example.org".to_owned())),
        ],
    );
    assert_eq!(answer.status, 403, "{}", answer.body);
    case.unchanged();
}

#[test]
fn a_post_without_the_token_is_refused() {
    let case = Case::new("no-token");
    let port = case.start();
    let answer = post_with(port, &disk_low(), &[]);
    assert_eq!(answer.status, 403, "{}", answer.body);
    let wrong = post_with(
        port,
        &disk_low(),
        &[("X-Conductor-Token", Some("0".repeat(64)))],
    );
    assert_eq!(wrong.status, 403, "{}", wrong.body);
    case.unchanged();
}

#[test]
fn an_invalid_result_is_refused_naming_its_yaml_path() {
    let case = Case::new("invalid");
    let port = case.start();
    let answer = post(
        port,
        &json!({
            "changes": [{"path": "cadence.watch", "value": "soon"}],
            "reason": "faster",
        }),
    );
    assert_eq!(answer.status, 422, "{}", answer.body);
    let refused = answer.json();
    assert_eq!(refused["refusal"], "Invalid", "{refused}");
    assert!(
        refused["problems"]
            .as_array()
            .is_some_and(|problems| problems
                .iter()
                .any(|problem| problem["path"] == "instances[0].cadence.watch")),
        "{refused}"
    );
    case.unchanged();
}

#[test]
fn a_fixed_key_or_a_write_without_a_reason_is_refused() {
    let case = Case::new("fixed");
    let port = case.start();
    let fixed = post(
        port,
        &json!({"changes": [{"path": "records", "value": "/tmp/elsewhere"}], "reason": "move"}),
    );
    assert_eq!(fixed.status, 422, "{}", fixed.body);
    let no_reason = post(
        port,
        &json!({"changes": [{"path": "thresholds.disk_low", "value": "200G"}]}),
    );
    assert_eq!(no_reason.status, 422, "{}", no_reason.body);
    assert_eq!(no_reason.json()["refusal"], "ReasonMissing");
    case.unchanged();
}

#[test]
fn a_check_answers_the_diff_and_writes_nothing() {
    let case = Case::new("check");
    let port = case.start();
    let mut body = disk_low();
    body["check"] = json!(true);
    let answer = post(port, &body);
    assert_eq!(answer.status, 200, "{}", answer.body);
    let checked = answer.json();
    assert_eq!(
        checked["diff"], "- thresholds.disk_low: 100G\n+ thresholds.disk_low: 200G",
        "{checked}"
    );
    assert_eq!(checked["problems"], json!([]));
    case.unchanged();
}

#[test]
fn a_valid_write_keeps_a_backup_and_records_one_decision() {
    let case = Case::new("valid");
    let port = case.start();
    let answer = post(port, &disk_low());
    assert_eq!(answer.status, 200, "{}", answer.body);
    let written = answer.json();
    let backups = case.backups();
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), case.text());
    assert!(
        backups[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("conductor.yaml."),
        "{backups:?}"
    );
    let new = fs::read_to_string(case.file()).unwrap();
    assert!(new.contains("disk_low: 200G"), "{new}");
    assert!(new.contains("watch: 3m"), "the rest is kept: {new}");
    let decisions = case.decisions();
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    let decision = &decisions[0];
    assert_eq!(
        decision["decision_id"], written["decision_id"],
        "{decision}"
    );
    assert_eq!(decision["decision_class"], "C", "{decision}");
    assert_eq!(decision["decided_by"], "Operator", "{decision}");
    assert_eq!(decision["recorded_by"], "dashboard", "{decision}");
    assert!(
        decision["evidence"]
            .as_str()
            .is_some_and(|evidence| evidence.contains("+ thresholds.disk_low: 200G")),
        "{decision}"
    );
    let page = get(port, "/api/config").json();
    let field = page["fields"]
        .as_array()
        .and_then(|fields| {
            fields
                .iter()
                .find(|field| field["path"] == "thresholds.disk_low")
        })
        .cloned()
        .unwrap_or_default();
    assert_eq!(field["value"], "200G", "{page}");
    assert_eq!(field["default"], "100G", "{page}");
}

#[test]
fn an_activity_switch_is_written_as_the_operators_mark() {
    let case = Case::new("activity");
    let port = case.start();
    let missing = post(
        port,
        &json!({
            "activity": [{"repository": "alpha", "activity": "Inactive", "reason": " "}],
            "reason": "pause alpha",
        }),
    );
    assert_eq!(missing.status, 422, "{}", missing.body);
    let answer = post(
        port,
        &json!({
            "activity": [{"repository": "alpha", "activity": "Inactive", "reason": "paused"}],
            "reason": "pause alpha",
        }),
    );
    assert_eq!(answer.status, 200, "{}", answer.body);
    assert!(case.backups().is_empty(), "no config change, no backup");
    assert_eq!(case.decisions().len(), 1);
    let page = get(port, "/api/config").json();
    let alpha = page["repositories"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["repository"] == "alpha"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(alpha["activity"], "Inactive", "{page}");
    assert_eq!(alpha["marked_by"], "Operator", "{page}");
    assert_eq!(alpha["reason"], "paused", "{page}");
}
