//! `story:control-plane-work-handler`: the instance runtime's work call. `POST /v1/work` takes one
//! unit of work (`conductor.dispatch.WorkRequest`) and sends it as a dispatch with the caller's
//! `reference` and no goal, through the code `dispatch send-dispatch --reference` runs; it answers
//! the unit's state (`conductor.dispatch.WorkState`), `201` for a dispatch it created and `200`
//! for a reference a dispatch already holds. `GET /v1/work/{reference}` answers the same row,
//! its times, its detail and its last report read from the records' dispatch log and the
//! messages handled for the dispatch.
//!
//! Each case runs the built binary's `instance serve` with its loop off over a config file,
//! records and a state directory under this test target's temporary directory, kept short so the
//! socket's path fits, and talks to its socket. The records' workspace file names the
//! repositories `alpha` and `beta`. A report reaches the dispatch as a controller sends one:
//! `conductor message send`, then one watch tick of the loop, run by a child of this test binary
//! ([`child`]) with the case's config file as its instance.

mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::bail;
use conductor_cli::config;
use conductor_cli::run::{self, Headroom, Seams};
use conductor_cli::session::{Deliverer, Delivery};
use serde_json::{Value, json};

/// The variable that makes a run of this test binary the child tick ([`child`]).
const CHILD: &str = "CONDUCTOR_WORK_ROUTE_CHILD";

/// The instance's token, built from pieces: a fixture, not a secret.
fn token() -> String {
    ["work", "route", "fixture"].join("-") + &"7".repeat(40)
}

/// One case's directory, short so `<state>/api.sock` fits a socket address.
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
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("wr")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        let case = Self { dir };
        for sub in ["dispatches", "decisions", ".engineering"] {
            fs::create_dir_all(case.records().join(sub)).expect("create a records directory");
        }
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::create_dir_all(case.state()).expect("create the state directory");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        fs::write(
            case.records().join(".engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers:\n  - name: alpha\n    source: ../alpha\n  \
             - name: beta\n    source: ../beta\n",
        )
        .expect("write the workspace file");
        fs::write(case.file(), case.text()).expect("write the config file");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("h")
    }

    fn records(&self) -> PathBuf {
        self.home().join("r")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("s")
    }

    fn socket(&self) -> PathBuf {
        self.state().join("api.sock")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    fn text(&self) -> String {
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   placement: local\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            root = self.home().join("example-org"),
            trees = self.home().join("trees"),
            records = self.records(),
            state = self.state(),
            cache = self.dir.join("c"),
        )
    }

    /// The runtime of the case, its loop off, serving on its socket once this answers.
    fn serve(&self) -> Running {
        let child = common::conductor(self.home())
            .current_dir(self.state())
            .arg("--config")
            .arg(self.file())
            .args(["instance", "serve"])
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(config::INSTANCE_VARIABLE, "example-org")
            .env(conductor_cli::instance::TOKEN_VARIABLE, token())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("instance serve starts");
        let mut running = Running(child);
        let until = Instant::now() + Duration::from_secs(30);
        while UnixStream::connect(self.socket()).is_err() {
            if let Ok(Some(status)) = running.0.try_wait() {
                let mut stderr = String::new();
                if let Some(mut pipe) = running.0.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                panic!("instance serve exited {status} before it listened: {stderr}");
            }
            assert!(
                Instant::now() < until,
                "no socket at {}",
                self.socket().display()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        running
    }

    /// `conductor <args>` with the case's home and config file, `stdin` on standard input.
    fn conductor(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = common::conductor(self.home())
            .args(args)
            .current_dir(self.records())
            .env(config::CONFIG_VARIABLE, self.file())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("conductor runs");
        child
            .stdin
            .take()
            .expect("its standard input")
            .write_all(stdin.as_bytes())
            .expect("write its standard input");
        child.wait_with_output().expect("conductor ends")
    }

    /// [`Case::conductor`], which must succeed; its standard output.
    fn ok(&self, args: &[&str], stdin: &str) -> String {
        let output = self.conductor(args, stdin);
        assert!(
            output.status.success(),
            "{args:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Every dispatch of the case's store, as `dispatch dispatches` lists them.
    fn dispatches(&self) -> Vec<Value> {
        let output = self.conductor(&["dispatch", "dispatches", "--format", "json"], "");
        if !output.status.success() {
            return Vec::new();
        }
        serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
    }

    /// Every message of the case's store, as `message messages` lists them.
    fn messages(&self) -> Vec<Value> {
        let output = self.conductor(&["message", "messages", "--format", "json"], "");
        assert!(output.status.success(), "message messages: {output:?}");
        serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
    }

    /// The row `message messages` gives the message `id`.
    fn message(&self, id: &str) -> Value {
        let rows = self.messages();
        rows.iter()
            .find(|row| row["message_id"] == id)
            .unwrap_or_else(|| panic!("no message {id}: {rows:#?}"))
            .clone()
    }

    /// A message `first_line` from `alpha`'s controller into conductor's mailbox, not yet handled;
    /// its id.
    fn send(&self, first_line: &str) -> String {
        self.ok(
            &[
                "message",
                "send",
                "--to",
                "conductor",
                "--from",
                "eo-alpha",
                "--text",
                "-",
            ],
            &format!("{first_line}\n"),
        )
        .trim()
        .to_owned()
    }

    /// A report `first_line` from `alpha`'s controller into conductor's mailbox, then one watch
    /// tick that handles it.
    fn report(&self, first_line: &str) {
        self.ok(
            &[
                "message",
                "send",
                "--to",
                "conductor",
                "--from",
                "eo-alpha",
                "--text",
                "-",
            ],
            &format!("{first_line}\n"),
        );
        self.tick();
    }

    /// One watch tick of the loop over the case's instance, in a child of this test binary.
    fn tick(&self) {
        let output = Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "--exact",
                "child",
                "--nocapture",
                "--test-threads",
                "1",
                "-q",
            ])
            .env("HOME", self.home())
            .env("USER", "operator")
            .env_remove(config::INSTANCE_VARIABLE)
            .env_remove(conductor_cli::actor::SESSION_VARIABLE)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env(config::CONFIG_VARIABLE, self.file())
            .env(CHILD, "tick")
            .current_dir(self.records())
            .stdin(Stdio::null())
            .output()
            .expect("the child runs");
        assert!(
            output.status.success(),
            "the tick failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// A child process killed when the case ends.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Running {
    /// Stops the runtime; what it wrote on standard error.
    fn stop(&mut self) -> String {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let mut stderr = String::new();
        if let Some(mut pipe) = self.0.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        stderr
    }
}

/// The deliverer of the child tick: no session runs, so nothing is delivered.
struct Nobody;

impl Deliverer for Nobody {
    fn deliver(&self, name: &str, _text: &str) -> anyhow::Result<Delivery> {
        bail!("no session {name} runs in this case")
    }
}

/// The headroom of the child tick: always.
struct Plenty;

impl Headroom for Plenty {
    fn headroom(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// The child tick ([`Case::tick`]); a plain run of the suite passes at once. It fixes the case's
/// config file as the process's instance and runs one watch tick over no watch line, which reads
/// conductor's mailbox.
#[test]
fn child() {
    if std::env::var(CHILD).is_err() {
        return;
    }
    let file = PathBuf::from(std::env::var_os(config::CONFIG_VARIABLE).expect("a config file"));
    config::set_active(Some(&file)).expect("the case's config loads");
    let host = conductor_cli::confine::Host;
    let program = PathBuf::from(common::conductor(Path::new("/nonexistent")).get_program());
    let seams = Seams {
        providers: &host,
        deliverer: &Nobody,
        headroom: &Plenty,
        handle_request: None,
        program,
        gc: conductor_cli::gc::gc,
    };
    match run::handle_lines(None, None, &[], &seams) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

/// An HTTP answer: its status and its body.
#[derive(Debug)]
struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {self:?}"))
    }
}

/// Sends `head` (the request line and headers, no blank line) and `body` to the case's socket.
fn exchange(case: &Case, head: &str, body: &[u8]) -> Answer {
    let mut stream = UnixStream::connect(case.socket()).expect("connect to the runtime's socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    let mut request = format!("{head}Connection: close\r\n\r\n").into_bytes();
    request.extend_from_slice(body);
    // A runtime that answers before it reads the whole body may close while this still writes.
    let _ = stream.write_all(&request);
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {text:?}"));
    Answer {
        status,
        body: body.to_owned(),
    }
}

/// `POST /v1/work` with `body`, the instance's token when `tokened`, and `headers`.
fn post_raw(case: &Case, body: &[u8], tokened: bool, headers: &[(&str, &str)]) -> Answer {
    let mut head = format!(
        "POST /v1/work HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n",
        body.len()
    );
    if tokened {
        head.push_str(&format!("X-Conductor-Token: {}\r\n", token()));
    }
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    exchange(case, &head, body)
}

/// `POST /v1/work` with `body` as JSON and the instance's token.
fn post(case: &Case, body: &Value) -> Answer {
    post_raw(case, body.to_string().as_bytes(), true, &[])
}

/// `GET /v1/work/<reference>`, as the daemon forwards a read: no token.
fn get(case: &Case, reference: &str) -> Answer {
    exchange(
        case,
        &format!("GET /v1/work/{reference} HTTP/1.1\r\nHost: localhost\r\n"),
        b"",
    )
}

/// A unit of work for `repository` under `reference`.
fn work(repository: &str, reference: &str) -> Value {
    json!({
        "repository": repository,
        "brief": "Add the export button to the report page",
        "reference": reference,
        "start_ref": "main",
        "issue": "https://forge.example.org/example-org/alpha/issues/7",
    })
}

/// `POST` creates one Sent dispatch with the reference and answers `201` with its row; the same
/// reference again creates nothing and answers `200` with the same dispatch, whatever the other
/// fields say; another reference for the repository is `409` naming the open dispatch, and is
/// taken once that one is done.
#[test]
fn a_unit_of_work_is_one_dispatch_held_by_its_reference_and_a_busy_repository_refuses_another() {
    let case = Case::new("send");
    let _runtime = case.serve();

    let created = post(&case, &work("alpha", "WI-1042"));
    assert_eq!(created.status, 201, "{created:?}");
    let row = created.json();
    let id = row["dispatch_id"]
        .as_str()
        .expect("a dispatch id")
        .to_owned();
    assert!(id.starts_with("DSP-"), "{row}");
    assert_eq!(row["reference"], "WI-1042", "{row}");
    assert_eq!(row["repository"], "alpha", "{row}");
    assert_eq!(row["state"], "Sent", "{row}");
    assert!(
        row["sent_at"].as_str().is_some_and(|at| !at.is_empty()),
        "{row}"
    );
    assert_eq!(row["links"], json!([]), "{row}");
    for absent in ["started_at", "ended_at", "detail", "last_report"] {
        assert!(row.get(absent).is_none(), "{absent} on a Sent row: {row}");
    }
    let dispatches = case.dispatches();
    assert_eq!(dispatches.len(), 1, "{dispatches:#?}");
    assert_eq!(dispatches[0]["state"], "Sent");
    assert_eq!(dispatches[0]["reference"], "WI-1042");
    assert_eq!(dispatches[0]["goal_id"], Value::Null);
    assert_eq!(dispatches[0]["start_ref"], "main");
    assert_eq!(dispatches[0]["priority"], 0);

    let again = post(&case, &work("alpha", "WI-1042"));
    assert_eq!(again.status, 200, "{again:?}");
    assert_eq!(again.json()["dispatch_id"], id.as_str(), "{again:?}");
    let mut other = work("beta", "WI-1042");
    other["brief"] = json!("");
    let held = post(&case, &other);
    assert_eq!(
        held.status, 200,
        "a held reference answers whatever else: {held:?}"
    );
    assert_eq!(held.json()["dispatch_id"], id.as_str(), "{held:?}");
    assert_eq!(case.dispatches().len(), 1, "nothing was created");

    let busy = post(&case, &work("alpha", "WI-1043"));
    assert_eq!(busy.status, 409, "{busy:?}");
    let answer = busy.json();
    assert_eq!(
        answer["error"], "conductor.dispatch.RepositoryBusy",
        "{answer}"
    );
    assert_eq!(answer["open"], json!([id]), "{answer}");
    assert_eq!(case.dispatches().len(), 1, "nothing was created");

    case.ok(
        &[
            "dispatch",
            "report-done",
            "--dispatch-id",
            &id,
            "--evidence",
            "merged",
        ],
        "",
    );
    let taken = post(&case, &work("alpha", "WI-1043"));
    assert_eq!(taken.status, 201, "{taken:?}");
    assert_ne!(taken.json()["dispatch_id"], id.as_str());
    assert_eq!(case.dispatches().len(), 2);
}

/// Each request the call does not take is `422` naming the specification's error, and nothing is
/// sent; a body over 1 MiB is `413` before it is read.
#[test]
fn a_request_it_does_not_take_is_422_by_its_declared_name() {
    let case = Case::new("refuse");
    let _runtime = case.serve();
    let with = |member: &str, value: Value| {
        let mut body = work("alpha", "WI-7");
        body[member] = value;
        body
    };
    let named = |answer: &Answer, error: &str| {
        assert_eq!(answer.status, 422, "{error}: {answer:?}");
        assert_eq!(answer.json()["error"], error, "{answer:?}");
    };
    let long = "r".repeat(65);
    for reference in ["a/b", "-lead", "a b", "a%2Fb", "", long.as_str()] {
        named(
            &post(&case, &with("reference", json!(reference))),
            "conductor.dispatch.InvalidReference",
        );
    }
    named(
        &post(&case, &with("repository", json!("gamma"))),
        "conductor.dispatch.UnknownRepository",
    );
    named(
        &post(&case, &with("brief", json!(""))),
        "conductor.dispatch.InvalidBrief",
    );
    named(
        &post(&case, &with("brief", json!("b".repeat(65_537)))),
        "conductor.dispatch.InvalidBrief",
    );
    named(
        &post(&case, &with("goal_id", json!("G-1"))),
        "conductor.dispatch.InvalidWorkRequest",
    );
    let mut missing = work("alpha", "WI-7");
    missing.as_object_mut().expect("an object").remove("brief");
    named(
        &post(&case, &missing),
        "conductor.dispatch.InvalidWorkRequest",
    );
    for body in [
        &b"[1, 2]"[..],
        &b"\"text\""[..],
        &b"not json"[..],
        &b"{\"repository\": "[..],
    ] {
        named(
            &post_raw(&case, body, true, &[]),
            "conductor.dispatch.InvalidWorkRequest",
        );
    }
    let brief = "b".repeat(65_536);
    let largest = post(&case, &with("brief", json!(brief)));
    assert_eq!(
        largest.status, 201,
        "a brief of 65536 bytes is taken: {}",
        largest.status
    );

    // Over 1 MiB: refused from its Content-Length, before the body arrives.
    let head = format!(
        "POST /v1/work HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nX-Conductor-Token: {}\r\n",
        1024 * 1024 + 1,
        token()
    );
    let large = exchange(&case, &head, b"{");
    assert_eq!(large.status, 413, "{large:?}");
    assert_eq!(
        case.dispatches().len(),
        1,
        "only the largest brief was sent"
    );
}

/// `GET` answers the row as the controller reports: each state, when it started and ended, the
/// reason while it is blocked, the links recorded on it and the first line of the last report
/// handled for it; a line of another dispatch never enters it. A reference no dispatch holds is
/// `404`, one that is no reference `422`.
#[test]
fn the_state_of_a_unit_follows_its_reports_links_and_log() {
    let case = Case::new("state");
    let _runtime = case.serve();
    let created = post(&case, &work("alpha", "WI-9"));
    assert_eq!(created.status, 201, "{created:?}");
    let id = created.json()["dispatch_id"]
        .as_str()
        .expect("an id")
        .to_owned();
    let row = |expected: &str| {
        let answer = get(&case, "WI-9");
        assert_eq!(answer.status, 200, "{answer:?}");
        let row = answer.json();
        assert_eq!(row["state"], expected, "{row}");
        assert_eq!(row["dispatch_id"], id.as_str(), "{row}");
        assert_eq!(row["reference"], "WI-9", "{row}");
        row
    };
    assert_eq!(row("Sent"), created.json());

    case.report(&format!("[REPORT alpha {id}] started on the export"));
    let started = row("Started");
    let started_at = started["started_at"]
        .as_str()
        .expect("started_at")
        .to_owned();
    assert!(started.get("ended_at").is_none(), "{started}");
    assert!(started.get("detail").is_none(), "{started}");
    assert_eq!(
        started["last_report"],
        format!("[REPORT alpha {id}] started on the export"),
        "{started}"
    );

    case.report("[REPORT alpha -] blocked on beta's API");
    // A line of another repository under the same id, as another instance's log would hold one.
    let month = &started_at[..7];
    let log = case.records().join(format!("dispatches/{month}.jsonl"));
    let mut text = fs::read_to_string(&log).expect("the dispatch log");
    text.push_str(
        &json!({"at": started_at, "event": "blocked", "repo": "beta", "id": id, "reason": "not this one"})
            .to_string(),
    );
    text.push('\n');
    fs::write(&log, text).expect("append a foreign line");
    let blocked = row("Blocked");
    assert_eq!(blocked["detail"], "on beta's API", "{blocked}");
    assert_eq!(blocked["started_at"], started_at.as_str(), "{blocked}");
    assert_eq!(
        blocked["last_report"], "[REPORT alpha -] blocked on beta's API",
        "{blocked}"
    );

    case.report(&format!("[REPORT alpha {id}] unblocked"));
    let unblocked = row("Started");
    assert!(unblocked.get("detail").is_none(), "{unblocked}");

    case.ok(
        &[
            "dispatch",
            "record-link",
            "--dispatch-id",
            &id,
            "--link",
            "https://forge.example.org/example-org/alpha/pull/12",
        ],
        "",
    );
    assert_eq!(
        row("Started")["links"],
        json!(["https://forge.example.org/example-org/alpha/pull/12"])
    );

    case.report(&format!("[REPORT alpha {id}] done PR 12 merged"));
    let done = row("Done");
    assert!(
        done["ended_at"]
            .as_str()
            .is_some_and(|at| at >= started_at.as_str()),
        "{done}"
    );
    assert_eq!(done["started_at"], started_at.as_str(), "{done}");
    assert!(done.get("detail").is_none(), "{done}");
    assert_eq!(
        done["last_report"],
        format!("[REPORT alpha {id}] done PR 12 merged"),
        "{done}"
    );
    assert_eq!(
        done["links"],
        json!(["https://forge.example.org/example-org/alpha/pull/12"])
    );

    let unknown = get(&case, "WI-10");
    assert_eq!(unknown.status, 404, "{unknown:?}");
    assert_eq!(
        unknown.json()["error"],
        "conductor.dispatch.WorkNotFound",
        "{unknown:?}"
    );
    let long = "r".repeat(65);
    for reference in ["a%2Fb", "..", long.as_str()] {
        let invalid = get(&case, reference);
        assert_eq!(invalid.status, 422, "{reference}: {invalid:?}");
        assert_eq!(
            invalid.json()["error"],
            "conductor.dispatch.InvalidReference",
            "{invalid:?}"
        );
    }
}

/// With the token, `X-Conductor-Actor-Kind: control-plane` records the dispatch as the control
/// plane's; without it, as the operator `X-Conductor-Operator` names, else the Unix user. A
/// request without the token is refused, its headers read for nothing.
#[test]
fn the_control_plane_header_is_honoured_only_with_the_token() {
    let case = Case::new("actor");
    let _runtime = case.serve();
    let body = |repository: &str, reference: &str| work(repository, reference).to_string();

    let refused = post_raw(
        &case,
        body("alpha", "WI-1").as_bytes(),
        false,
        &[("X-Conductor-Actor-Kind", "control-plane")],
    );
    assert_eq!(refused.status, 403, "{refused:?}");
    assert!(case.dispatches().is_empty(), "{:#?}", case.dispatches());

    let control = post_raw(
        &case,
        body("alpha", "WI-1").as_bytes(),
        true,
        &[("X-Conductor-Actor-Kind", "control-plane")],
    );
    assert_eq!(control.status, 201, "{control:?}");
    let named = post_raw(
        &case,
        body("beta", "WI-2").as_bytes(),
        true,
        &[("X-Conductor-Operator", "owner@example-org.test")],
    );
    assert_eq!(named.status, 201, "{named:?}");
    case.ok(
        &[
            "dispatch",
            "cancel-dispatch",
            "--dispatch-id",
            named.json()["dispatch_id"].as_str().expect("an id"),
            "--reason",
            "r",
        ],
        "",
    );
    let unix = post_raw(&case, body("beta", "WI-3").as_bytes(), true, &[]);
    assert_eq!(unix.status, 201, "{unix:?}");

    let actors: Vec<(String, String)> = case
        .dispatches()
        .iter()
        .map(|row| {
            (
                row["reference"].as_str().unwrap_or_default().to_owned(),
                row["actor"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        actors,
        [
            ("WI-1".to_owned(), "control-plane".to_owned()),
            ("WI-2".to_owned(), "human:owner@example-org.test".to_owned()),
            ("WI-3".to_owned(), "human:operator".to_owned()),
        ]
    );
}

/// The dispatch id of a `201` or `200` answer.
fn dispatch_of(answer: &Answer) -> String {
    assert!(matches!(answer.status, 200 | 201), "{answer:?}");
    answer.json()["dispatch_id"]
        .as_str()
        .expect("a dispatch id")
        .to_owned()
}

/// A `[REPORT alpha <beta's id>] done …` is a report of `alpha` that names no dispatch of
/// `alpha`: it is handled as one naming no dispatch. Beta's dispatch does not move, its log gets
/// no line, its row shows no report, and the message holds no dispatch.
#[test]
fn a_report_naming_another_repositorys_dispatch_moves_nothing_of_it() {
    let case = Case::new("foreign");
    let _runtime = case.serve();
    dispatch_of(&post(&case, &work("alpha", "WI-A")));
    let beta = dispatch_of(&post(&case, &work("beta", "WI-B")));

    let message = case.send(&format!("[REPORT alpha {beta}] done merged by alpha"));
    case.tick();

    let row = get(&case, "WI-B").json();
    assert_eq!(row["state"], "Sent", "beta's dispatch moved: {row}");
    for absent in ["started_at", "ended_at", "detail", "last_report"] {
        assert!(row.get(absent).is_none(), "{absent} on beta's row: {row}");
    }
    let held = case.message(&message);
    assert_eq!(held["state"], "Handled", "{held}");
    assert_eq!(held["dispatch_id"], Value::Null, "{held}");
}

/// A controller's `review <url>` report, sent as a controller sends one, reaches the row: its
/// `links` hold the URL, `last_report` the report's first line, and the Sent dispatch is
/// Started. The same report naming another repository's dispatch links nothing of it.
#[test]
fn a_review_report_puts_its_url_on_the_row() {
    let case = Case::new("review");
    let _runtime = case.serve();
    let alpha = dispatch_of(&post(&case, &work("alpha", "WI-A")));
    let beta = dispatch_of(&post(&case, &work("beta", "WI-B")));
    let link = "https://forge.example.org/example-org/alpha/pull/12";
    let first = format!("[REPORT alpha {alpha}] review {link} opened for WI-A");
    case.report(&first);

    let row = get(&case, "WI-A").json();
    assert_eq!(row["state"], "Started", "{row}");
    assert_eq!(row["links"], json!([link]), "{row}");
    assert_eq!(row["last_report"], first.as_str(), "{row}");
    assert!(row.get("started_at").is_some(), "{row}");

    let message = case.send(&format!(
        "[REPORT alpha {beta}] review https://forge.example.org/example-org/beta/pull/3"
    ));
    case.tick();
    let row = get(&case, "WI-B").json();
    assert_eq!(row["state"], "Sent", "{row}");
    assert_eq!(row["links"], json!([]), "{row}");
    assert!(row.get("last_report").is_none(), "{row}");
    assert_eq!(case.message(&message)["state"], "Handled");
    assert_eq!(get(&case, "WI-A").json()["links"], json!([link]));
}

/// `message handle-message --dispatch-id` takes only a dispatch that exists and is of the
/// repository the message's first line names: another repository's dispatch, or one no dispatch
/// has, is refused by its declared name on one line, exit 1, and the message stays unhandled.
/// The message's own repository's dispatch is taken, and becomes the row's last report.
#[test]
fn handle_message_takes_only_a_dispatch_of_the_messages_repository() {
    let case = Case::new("handle");
    let _runtime = case.serve();
    let alpha = dispatch_of(&post(&case, &work("alpha", "WI-A")));
    let beta = dispatch_of(&post(&case, &work("beta", "WI-B")));
    let first = format!("[REPORT alpha {alpha}] progress on alpha's export");
    let message = case.send(&first);

    for named in [beta.as_str(), "DSP-20990101-01"] {
        let output = case.conductor(
            &[
                "message",
                "handle-message",
                "--message-id",
                &message,
                "--dispatch-id",
                named,
            ],
            "",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{named}: {output:?}");
        assert!(
            stderr.starts_with("conductor: message handle-message: MessageDispatchMismatch: ")
                && stderr.contains(named)
                && stderr.lines().count() == 1,
            "{named}: {stderr:?}"
        );
        assert_eq!(case.message(&message)["state"], "Received", "{named}");
    }
    assert!(get(&case, "WI-B").json().get("last_report").is_none());

    case.ok(
        &[
            "message",
            "handle-message",
            "--message-id",
            &message,
            "--dispatch-id",
            &alpha,
        ],
        "",
    );
    let held = case.message(&message);
    assert_eq!(held["state"], "Handled", "{held}");
    assert_eq!(held["dispatch_id"], alpha.as_str(), "{held}");
    assert_eq!(get(&case, "WI-A").json()["last_report"], first.as_str());
}

/// `start_ref` is 1 to 255 bytes of what `git check-ref-format --allow-onelevel` admits, and
/// `issue` 1 to 2048 bytes with no ASCII control character: any other is `422`
/// `InvalidWorkRequest` naming the member, and nothing is sent. An absent member is taken.
/// `dispatch send-dispatch` refuses the same values by the same name.
#[test]
fn a_start_ref_or_an_issue_it_does_not_take_is_422_and_the_command_line_refuses_it_too() {
    let case = Case::new("members");
    let _runtime = case.serve();
    let with = |member: &str, value: &str| {
        let mut body = work("alpha", "WI-M");
        body[member] = json!(value);
        body
    };
    let long_ref = "r".repeat(256);
    let long_issue = "i".repeat(2049);
    let bad_refs = [
        "",
        "-main",
        "a b",
        "a\tb",
        "a\nb",
        "a\u{1b}[31m",
        "a\u{7f}",
        "a~1",
        "a^",
        "a:b",
        "a?",
        "a*",
        "a[b",
        "a\\b",
        "a..b",
        "a@{1}",
        "/a",
        "a/",
        "a//b",
        "a.",
        "a.lock",
        "@{",
        long_ref.as_str(),
    ];
    for start_ref in bad_refs {
        let answer = post(&case, &with("start_ref", start_ref));
        assert_eq!(answer.status, 422, "{start_ref:?}: {answer:?}");
        let refusal = answer.json();
        assert_eq!(
            refusal["error"], "conductor.dispatch.InvalidWorkRequest",
            "{start_ref:?}: {refusal}"
        );
        assert!(
            refusal["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("start_ref")),
            "{start_ref:?}: {refusal}"
        );
    }
    for issue in [
        "",
        "a\nb",
        "a\tb",
        "a\u{7f}",
        "a\u{1b}[2J",
        long_issue.as_str(),
    ] {
        let answer = post(&case, &with("issue", issue));
        assert_eq!(answer.status, 422, "{issue:?}: {answer:?}");
        let refusal = answer.json();
        assert_eq!(
            refusal["error"], "conductor.dispatch.InvalidWorkRequest",
            "{issue:?}: {refusal}"
        );
        assert!(
            refusal["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("issue")),
            "{issue:?}: {refusal}"
        );
    }
    assert!(case.dispatches().is_empty(), "{:#?}", case.dispatches());

    let mut absent = work("alpha", "WI-M");
    let body = absent.as_object_mut().expect("an object");
    body.remove("start_ref");
    body.remove("issue");
    dispatch_of(&post(&case, &absent));
    // Beta is free: each value taken is sent, or meets beta's open dispatch (409), never 422.
    let longest_ref = "r".repeat(255);
    let longest_issue = "i".repeat(2048);
    let good = [
        ("start_ref", "main"),
        ("start_ref", "feature/export-button"),
        ("start_ref", "refs/heads/release-1.2"),
        ("start_ref", "v1.2.3"),
        ("start_ref", "a.b"),
        ("start_ref", longest_ref.as_str()),
        ("issue", "#7"),
        ("issue", longest_issue.as_str()),
    ];
    for (index, (member, value)) in good.into_iter().enumerate() {
        let mut body = work("beta", &format!("WI-G{index}"));
        body[member] = json!(value);
        let answer = post(&case, &body);
        assert!(
            matches!(answer.status, 201 | 409),
            "{member} {value:?}: {answer:?}"
        );
    }

    for (flag, value) in [("--start-ref", "-main"), ("--issue", "a\u{1b}[2J")] {
        let output = case.conductor(
            &[
                "dispatch",
                "send-dispatch",
                "--repository",
                "alpha",
                "--brief",
                "Add the export button",
                "--sent-at",
                "2026-11-12T08:00:00Z",
                "--reference",
                "WI-C",
                &format!("{flag}={value}"),
            ],
            "",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{flag}: {output:?}");
        assert!(
            stderr.starts_with("conductor: dispatch send-dispatch: InvalidWorkRequest: ")
                && stderr.contains(flag),
            "{flag}: {stderr:?}"
        );
    }
    assert!(
        !case
            .dispatches()
            .iter()
            .any(|row| row["reference"] == "WI-C"),
        "the command line sent WI-C"
    );
}

/// A failure of the store or the records answers `500` naming the declared error and nothing of
/// the failure: no path, no error chain. The chain is in the runtime's line for the request, once.
#[test]
fn a_failure_answers_500_by_its_declared_name_and_its_log_line_holds_the_chain() {
    let case = Case::new("failure");
    let mut runtime = case.serve();
    dispatch_of(&post(&case, &work("alpha", "WI-F")));
    // Each month file of the dispatch log becomes a directory: the read of the row fails.
    let log = case.records().join("dispatches");
    for entry in fs::read_dir(&log).expect("the dispatch log") {
        let path = entry.expect("an entry").path();
        fs::remove_file(&path).expect("remove a month file");
        fs::create_dir(&path).expect("a directory in its place");
    }

    let answer = get(&case, "WI-F");
    assert_eq!(answer.status, 500, "{answer:?}");
    let body = answer.json();
    assert_eq!(body["error"], "conductor.dispatch.WorkFailed", "{body}");
    assert_eq!(body["reference"], "WI-F", "{body}");
    for leaked in [
        case.dir.display().to_string(),
        "dispatches".to_owned(),
        "read the".to_owned(),
    ] {
        assert!(!answer.body.contains(&leaked), "{leaked:?} in {body}");
    }

    let stderr = runtime.stop();
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("answered 500"))
        .collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(
        lines[0].contains("GET /v1/work/WI-F") && lines[0].contains("read the dispatch log"),
        "{stderr}"
    );
}
