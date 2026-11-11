//! conductord's probes and metrics (`story:daemon-probes-and-metrics`): `GET /healthz` answers
//! `200` while the process serves, whatever the store does; `GET /readyz` answers `200` once the
//! store answers and `503` with a one-line reason otherwise; `GET /metrics` answers Prometheus
//! text. All three answer without a bearer, so no body carries a config value, a token, a path,
//! the store URL or a name. The daemon serves while its store does not answer, and takes up its
//! work when it does, in the same process. No case reaches the host's service manager: a daemon
//! started here runs on a `PATH` of the case's own, which holds stand-ins that know and start no
//! unit.

mod active;
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::daemon::api::{self, Route};
use conductor_cli::daemon::metrics;
use conductor_cli::{host, store};
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::config::{DaemonOperator, OperatorGrant, OperatorRole};
use conductor_model::operator::OperatorName;
use serde_json::{Value, json};
use time::OffsetDateTime;

/// The daemon's token of a case, and the token of an instance it proxies to.
const TOKEN: &str = "4444444444444444444444444444444444444444444444444444444444444444";
const INSTANCE_TOKEN: &str = "5555555555555555555555555555555555555555555555555555555555555555";

/// A case's directory, short so a socket in it fits a socket address; removed after a passing
/// run, with the binary the daemon copied into it.
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
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dp")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file: the daemon on `port` with `store` as its `daemon.store`, and the
    /// one instance `alpha`.
    fn config(&self, port: u16, store: &str) {
        let root = self.dir.join("alpha");
        let text = format!(
            "version: conductor.config/2\ndaemon: {{port: {port}, store: {store:?}}}\n\
             instances:\n  - name: alpha\n    placement: box-a\n    session_prefix: alpha\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
             state: {state:?}\n    cache: {cache:?}\n",
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

/// One request to `port`: its status, head and body; none while nothing listens there.
fn ask(port: u16, method: &str, target: &str, host: &str) -> Option<(u16, String, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream
        .write_all(
            format!("{method} {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    let (head, body) = answer.split_once("\r\n\r\n")?;
    let status = head.split(' ').nth(1)?.parse().ok()?;
    Some((status, head.to_owned(), body.to_owned()))
}

/// `GET target` on `port` as the listener's own host.
fn get(port: u16, target: &str) -> (u16, String, String) {
    ask(port, "GET", target, "localhost").unwrap_or_else(|| panic!("no answer to GET {target}"))
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    })
}

/// A `daemon serve` of a case, killed when the case ends, with what it printed.
struct Served {
    child: Child,
    port: u16,
    printed: Arc<Mutex<String>>,
}

impl Served {
    /// Starts `daemon serve` on the case's config file and waits until its listener answers. Its
    /// `PATH` holds two stand-ins and nothing else, and the variables that name the user's
    /// service manager are gone, so its reconcile pass reaches none: a `systemctl` that knows no
    /// unit and a `systemd-run` that starts none.
    fn start(case: &Case, port: u16) -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let programs = case.dir.join("programs");
        fs::create_dir_all(&programs).expect("create the PATH directory");
        for (name, script) in [
            (
                "systemctl",
                "#!/bin/sh\necho LoadState=not-found\necho ActiveState=inactive\n",
            ),
            (
                "systemd-run",
                "#!/bin/sh\necho 'this case starts no unit' >&2\nexit 1\n",
            ),
        ] {
            let program = programs.join(name);
            fs::write(&program, script).expect("write the stand-in");
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
                .expect("make the stand-in runnable");
        }
        let mut child = common::conductor(case.home())
            .arg("--config")
            .arg(case.file())
            .env("PATH", &programs)
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .args(["daemon", "serve", "--state-dir"])
            .arg(case.dir.join("daemon"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run daemon serve");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        let mut served = Self {
            child,
            port,
            printed,
        };
        served.until("the listener answers", |served| {
            ask(served.port, "GET", "/v1/instances", "localhost").is_some()
        });
        served
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }

    fn runs(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Waits until `holds`, while the daemon runs.
    fn until(&mut self, what: &str, holds: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !holds(self) {
            assert!(self.runs(), "the daemon exited: {}", self.printed());
            assert!(
                Instant::now() < deadline,
                "never held: {what}: {}",
                self.printed()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn the_daemon_serves_while_its_store_does_not_answer_and_takes_up_its_work_when_it_does() {
    let case = Case::new("no-store");
    let port = free_port();
    // The store's directory is a file: the database neither opens nor is created.
    let blocked = case.dir.join("blocked");
    fs::write(&blocked, "").expect("write the file in the store's way");
    case.config(
        port,
        &format!("sqlite:{}", blocked.join("shared.sqlite").display()),
    );
    let mut served = Served::start(&case, port);

    let (status, _, body) = get(port, "/healthz");
    assert_eq!(
        status, 200,
        "liveness while the store does not answer: {body}"
    );
    let (status, _, body) = get(port, "/readyz");
    assert_eq!(
        status, 503,
        "readiness while the store does not answer: {body}"
    );
    let rows: Value = serde_json::from_str(&body).expect("readiness answers JSON");
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "{body}");
    assert_eq!(rows[0]["reachable"], Value::Bool(false), "{body}");
    let reason = rows[0]["reason"].as_str().expect("a reason");
    assert!(!reason.is_empty() && !reason.contains('\n'), "{reason:?}");
    assert!(
        !body.contains("blocked") && !body.contains("sqlite") && !body.contains("dp/"),
        "the readiness body names the store: {body}"
    );
    // No instance was read yet: the metrics count none, where `0` would be a reading.
    let (status, _, body) = get(port, "/metrics");
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains("conductor_instances"), "{body}");
    // It started no runtime and recorded nothing.
    let (status, _, body) = get(port, "/v1/instances");
    assert_eq!((status, body.trim()), (200, "[]"));
    assert!(served.runs(), "the daemon exited: {}", served.printed());
    assert!(
        !served.printed().contains("alpha: Start"),
        "it acted without its store: {}",
        served.printed()
    );
    assert!(blocked.is_file(), "the store was written");

    // The store answers: the same process is ready and reconciles.
    fs::remove_file(&blocked).expect("clear the store's way");
    served.until("readiness answers 200", |served| {
        get(served.port, "/readyz").0 == 200
    });
    let (_, _, body) = get(port, "/readyz");
    assert_eq!(body.trim(), r#"[{"reachable":true,"reason":null}]"#);
    served.until("the instance is listed", |served| {
        get(served.port, "/v1/instances").2.contains("\"alpha\"")
    });
    served.until("the pass acted on the instance", |served| {
        served.printed().contains("alpha: Start")
    });
    assert_eq!(get(port, "/healthz").0, 200);
    assert!(served.runs(), "the daemon restarted or exited");
    let store = store::open_url(
        &format!("sqlite:{}", blocked.join("shared.sqlite").display()),
        conductor_cli::daemon::TENANT,
    )
    .expect("open the daemon's store");
    assert!(
        !ReconciliationStorage::list(&store).is_empty(),
        "the pass recorded nothing: {}",
        served.printed()
    );
}

#[test]
fn a_database_server_that_takes_no_connection_is_503_and_the_daemon_lives() {
    let case = Case::new("no-server");
    let port = free_port();
    // Nothing listens on the store's port.
    case.config(
        port,
        &format!("postgres://conductor@127.0.0.1:{}/conductor", free_port()),
    );
    let mut served = Served::start(&case, port);
    assert_eq!(get(port, "/healthz").0, 200);
    let (status, _, body) = get(port, "/readyz");
    assert_eq!(status, 503, "{body}");
    assert!(
        !body.contains("postgres") && !body.contains("127.0.0.1"),
        "the readiness body names the store: {body}"
    );
    assert_eq!(get(port, "/v1/instances").2.trim(), "[]");
    assert!(served.runs(), "the daemon exited: {}", served.printed());
}

#[test]
fn the_three_paths_answer_without_a_bearer_ahead_of_the_app() {
    active::isolate();
    let daemon = Arc::new(api::Api::new("0".repeat(64), "operator".to_owned(), None));
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&daemon);
    thread::spawn(move || api::serve(&listener, &served));

    let (status, head, body) = get(port, "/healthz");
    assert_eq!(status, 200, "{body}");
    assert_eq!(header(&head, "content-type"), Some("application/json"));
    assert_eq!(body.trim(), r#"{"state":"serving"}"#);

    // No store has answered this API: not ready, and alive all the same.
    let (status, _, body) = get(port, "/readyz");
    assert_eq!(status, 503, "{body}");
    assert_eq!(
        body.trim(),
        r#"[{"reachable":false,"reason":"the store has not answered yet"}]"#
    );

    let (status, head, body) = get(port, "/metrics");
    assert_eq!(status, 200, "{body}");
    assert!(
        header(&head, "content-type").is_some_and(|kind| kind.starts_with("text/plain")),
        "{head}"
    );

    // The loopback listener's Host check stays ahead of them.
    for path in ["/healthz", "/readyz", "/metrics"] {
        let (status, _, _) = ask(port, "GET", path, "example-org.test").expect("an answer");
        assert_eq!(status, 403, "{path} answered a foreign Host");
        let (status, head, _) = ask(port, "POST", path, "localhost").expect("an answer");
        assert_eq!(status, 405, "{path} answered a POST");
        assert_eq!(header(&head, "allow"), Some("GET"), "{head}");
    }
}

/// An API of the daemon on a loopback listener of its own; the API and its port.
fn serving(daemon: api::Api) -> (Arc<api::Api>, u16) {
    let daemon = Arc::new(daemon);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&daemon);
    thread::spawn(move || api::serve(&listener, &served));
    (daemon, port)
}

#[test]
fn readiness_follows_what_the_store_last_answered_and_liveness_does_not() {
    active::isolate();
    let (daemon, port) = serving(api::Api::new(TOKEN.to_owned(), "operator".to_owned(), None));
    let readiness = || {
        let (status, _, body) = get(port, "/readyz");
        assert_eq!(get(port, "/healthz").0, 200, "liveness follows the store");
        (status, body.trim().to_owned())
    };

    daemon.publish_store(true);
    assert_eq!(
        readiness(),
        (200, r#"[{"reachable":true,"reason":null}]"#.to_owned())
    );
    assert!(daemon.store_answers());

    daemon.publish_store(false);
    assert_eq!(
        readiness(),
        (
            503,
            r#"[{"reachable":false,"reason":"the store does not answer"}]"#.to_owned()
        )
    );
    assert!(!daemon.store_answers());

    // An answer older than the bound is none: a read that hangs leaves the last one to age.
    let long_ago = Instant::now()
        .checked_sub(metrics::STORE_BOUND + Duration::from_secs(1))
        .expect("an instant before the bound");
    daemon.publish_store_at(true, long_ago);
    assert_eq!(
        readiness(),
        (
            503,
            r#"[{"reachable":false,"reason":"the store has not answered within 10 s"}]"#.to_owned()
        )
    );
    assert_eq!(
        metrics::not_ready(Some((true, long_ago)), long_ago),
        None,
        "an answer within the bound is ready"
    );
}

#[test]
fn while_the_daemon_starts_a_probe_answers_and_any_other_request_waits_for_the_reconciler() {
    active::isolate();
    let starting =
        || api::Api::new(TOKEN.to_owned(), "operator".to_owned(), None).starting(Vec::new());
    let (daemon, port) = serving(starting());
    assert_eq!(get(port, "/healthz").0, 200);
    assert_eq!(get(port, "/readyz").0, 503);
    assert_eq!(get(port, "/metrics").0, 200);
    // A client that connects as soon as the listener is bound reads what the reconciler first
    // publishes, never the nothing before it.
    let held = Duration::from_millis(600);
    let asked = Instant::now();
    thread::spawn(move || {
        thread::sleep(held);
        daemon.publish(
            vec![json!({"name": "example-one", "state": "Running"})],
            BTreeMap::new(),
            Vec::new(),
        );
    });
    let (status, _, body) = get(port, "/v1/instances");
    assert!(
        asked.elapsed() >= held,
        "it answered before the reconciler published: {body}"
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("example-one"), "{body}");

    // Once the store is found not to answer, a request is answered without it.
    let (daemon, port) = serving(starting());
    daemon.started();
    let (status, _, body) = get(port, "/v1/instances");
    assert_eq!((status, body.trim()), (200, "[]"));
}

#[test]
fn a_write_before_the_reconciler_published_needs_the_grant_the_config_file_writes() {
    active::isolate();
    let owner = DaemonOperator {
        name: OperatorName("owner".to_owned()),
        role: OperatorRole("owner".to_owned()),
        grants: vec![OperatorGrant::AnswerDecisions],
    };
    let (daemon, port) =
        serving(api::Api::new(TOKEN.to_owned(), "owner".to_owned(), None).starting(vec![owner]));
    daemon.started();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream
        .write_all(
            format!(
                "PUT /v1/config HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
                 Origin: http://127.0.0.1:{port}\r\nX-Conductor-Token: {TOKEN}\r\n\
                 Content-Type: application/json\r\nContent-Length: 2\r\n\
                 Connection: close\r\n\r\n{{}}"
            )
            .as_bytes(),
        )
        .expect("send");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("an answer");
    let (head, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
    assert!(head.starts_with("HTTP/1.1 403 "), "{head}\n{body}");
    let body: Value = serde_json::from_str(body).expect("a JSON refusal");
    assert_eq!(body["error"], "conductor.operator.GrantMissing", "{body}");
    assert_eq!(body["grant"], "ChangeSettings", "{body}");
}

/// An instance's socket that answers every request with `data` as its `/v1/data.json`.
fn answering(socket: &Path, data: &Value) {
    let listener = UnixListener::bind(socket).expect("bind the instance's socket");
    let body = data.to_string();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") && matches!(stream.read(&mut byte), Ok(1)) {
                head.push(byte[0]);
            }
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                     Connection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
        }
    });
}

/// The samples of `text`: every line that is no comment.
fn samples(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect()
}

#[test]
fn metrics_hold_what_was_read_and_leave_out_what_was_not_within_the_bound() {
    let case = Case::new("metrics");
    case.config(
        7300,
        &format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
    );
    // The first instance answers, the second takes the connection and says nothing, the third
    // has no runtime.
    let (first, second) = (case.dir.join("1.sock"), case.dir.join("2.sock"));
    answering(
        &first,
        &json!({
            "sessions": [
                {"name": "example-org-controller", "repository": "example-repo",
                 "state": "working", "cwd": "/srv/example-org/example-repo"},
                {"name": "example-org-planner", "repository": "example-repo",
                 "state": "working", "cwd": "/srv/example-org/example-repo"},
                {"name": "example-org-reviewer", "repository": "example-repo",
                 "state": "not running", "cwd": null},
                {"name": "example-org-lost", "repository": null, "state": null, "cwd": null},
            ],
            "snapshot": {"id": "0190-example", "started_at": "2026-11-08T09:00:00Z",
                         "age_seconds": 42, "age": "42 s", "cadence_seconds": 300, "stale": false},
        }),
    );
    let silent = UnixListener::bind(&second).expect("bind the silent instance's socket");
    let route = |socket: &Path| Route {
        socket: socket.to_owned(),
        token: INSTANCE_TOKEN.to_owned(),
        remote: None,
    };
    // Two disk requests wait in the host's build ledger, beside the config file.
    let host = conductor_cli::config::host_of(&case.file());
    let mut ledger = host::Ledger::lock(&host).expect("the ledger opens");
    let at = host::now(OffsetDateTime::now_utc());
    for n in 0..2 {
        ledger
            .request_of(
                "example-org",
                Some("example-repo"),
                None,
                Some(&format!("session-{n}")),
                1 << 30,
                "cargo build",
                &at,
            )
            .expect("the request");
    }
    assert_eq!(ledger.session_bound().expect("a pass").len(), 2);
    drop(ledger);

    let bound = Duration::from_secs(1);
    let fresh = || {
        api::Api::new(TOKEN.to_owned(), "operator".to_owned(), None)
            .with_config(
                case.file(),
                case.home(),
                format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
            )
            .with_timeout(bound)
    };

    // Before the reconciler published, nothing of the instances is read: no line, never `0`.
    let (_, unpublished) = serving(fresh());
    let (status, _, body) = get(unpublished, "/metrics");
    assert_eq!(status, 200, "{body}");
    assert_eq!(samples(&body), ["conductor_build_queue_length 2"], "{body}");

    let (daemon, port) = serving(fresh());
    daemon.publish(
        vec![
            json!({"name": "example-one", "state": "Running", "placement": "box-a"}),
            json!({"name": "example-two", "state": "Running", "placement": "box-b"}),
            json!({"name": "example-three", "state": "Stopped", "placement": "box-c"}),
        ],
        [
            ("example-one".to_owned(), route(&first)),
            ("example-two".to_owned(), route(&second)),
        ]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    let asked = Instant::now();
    let (status, head, body) = get(port, "/metrics");
    let took = asked.elapsed();
    drop(silent);
    assert_eq!(status, 200, "{body}");
    assert_eq!(header(&head, "content-type"), Some(metrics::TEXT), "{head}");
    assert!(
        took >= bound.mul_f32(0.9),
        "the silent instance was not waited for: {took:?}"
    );
    assert!(
        took < bound + Duration::from_secs(20),
        "the scrape did not answer within its bound: {took:?}"
    );
    assert_eq!(
        samples(&body),
        [
            r#"conductor_instances{state="purged"} 0"#,
            r#"conductor_instances{state="running"} 2"#,
            r#"conductor_instances{state="starting"} 0"#,
            r#"conductor_instances{state="stopped"} 1"#,
            r#"conductor_instances{state="unreachable"} 0"#,
            r#"conductor_instances{state="upgrading"} 0"#,
            "conductor_instances_unanswered 2",
            r#"conductor_sessions{instance="1",state="not_running"} 1"#,
            r#"conductor_sessions{instance="1",state="unknown"} 1"#,
            r#"conductor_sessions{instance="1",state="working"} 2"#,
            r#"conductor_snapshot_age_seconds{instance="1"} 42"#,
            "conductor_build_queue_length 2",
        ],
        "{body}"
    );
    // Every sample's metric is described, and no body names what only a bearer may read.
    for sample in samples(&body) {
        let name = sample.split(['{', ' ']).next().unwrap_or_default();
        assert!(body.contains(&format!("# TYPE {name} gauge\n")), "{name}");
        assert!(body.contains(&format!("# HELP {name} ")), "{name}");
    }
    let private = [
        "example-one",
        "example-two",
        "example-three",
        "example-org",
        "example-repo",
        "box-a",
        "session-0",
        "cargo build",
        "0190-example",
        "sqlite",
        ".sock",
        "/srv",
        TOKEN,
        INSTANCE_TOKEN,
    ];
    let case_dir = case.dir.display().to_string();
    for path in ["/healthz", "/readyz", "/metrics"] {
        let (_, _, body) = get(port, path);
        for word in private.iter().copied().chain([case_dir.as_str()]) {
            assert!(!body.contains(word), "{path} names {word:?}: {body}");
        }
    }
}

#[test]
fn a_scrape_renders_only_what_it_holds() {
    assert_eq!(metrics::render(&metrics::Scrape::default()), "");
    let text = metrics::render(&metrics::Scrape {
        instances: None,
        readings: Some(vec![
            None,
            Some(metrics::Reading {
                sessions: [("idle".to_owned(), 3)].into_iter().collect(),
                snapshot_age: None,
            }),
            Some(metrics::Reading::default()),
        ]),
        queue: None,
    });
    assert_eq!(
        samples(&text),
        [
            "conductor_instances_unanswered 1",
            r#"conductor_sessions{instance="2",state="idle"} 3"#,
        ],
        "{text}"
    );
    // A state an instance answers becomes a label's letters, whatever it holds.
    assert_eq!(metrics::label("Not Running"), "not_running");
    assert_eq!(metrics::label("a/b\"c\n{d}"), "a_b_c__d_");
    assert_eq!(metrics::label(""), "unknown");
    assert_eq!(metrics::label(&"x".repeat(200)).len(), 40);
}

#[test]
fn the_ledger_s_queue_is_absent_where_it_does_not_read() {
    let case = Case::new("queue");
    let host = case.dir.join("host");
    assert_eq!(
        store::queued(&host),
        Some(0),
        "a host without a ledger has no request waiting"
    );
    fs::create_dir_all(&host).expect("create the host directory");
    fs::write(host.join(store::STORE_FILE), "").expect("plant a store");
    assert_eq!(
        store::queued(&host),
        None,
        "a ledger whose open grants were never written is not read as empty"
    );
    fs::write(host.join("ledger.open"), "not json").expect("plant the open grants");
    assert_eq!(store::queued(&host), None);
    fs::write(
        host.join("ledger.open"),
        json!({"position": 3, "grants": {
            "a": {"state": "Requested", "kind": "Disk"},
            "b": {"state": "Running", "kind": "Disk"},
        }})
        .to_string(),
    )
    .expect("plant the open grants");
    assert_eq!(store::queued(&host), Some(1));
}

#[test]
fn the_tls_listener_answers_the_three_paths_without_a_bearer_and_nothing_else() {
    let case = Case::new("tls");
    let generated =
        rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("a certificate");
    fs::write(case.dir.join("cert.pem"), generated.cert.pem()).expect("write the cert");
    fs::write(case.dir.join("key.pem"), generated.key_pair.serialize_pem()).expect("write the key");
    let tls = api::tls_config(&case.dir.join("cert.pem"), &case.dir.join("key.pem"))
        .expect("the TLS listener's config");
    let daemon = Arc::new(api::Api::new(TOKEN.to_owned(), "operator".to_owned(), None));
    daemon.publish_store(true);
    let secure = TcpListener::bind(("127.0.0.1", 0)).expect("bind the TLS listener");
    let port = secure.local_addr().expect("address").port();
    thread::spawn(move || api::serve_tls(&secure, &daemon, &tls));

    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(generated.cert.der().clone())
        .expect("trust the certificate");
    let config = Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth(),
    );
    let status = |path: &str| {
        let name = rustls::pki_types::ServerName::try_from("localhost").expect("a server name");
        let connection =
            rustls::ClientConnection::new(Arc::clone(&config), name).expect("a TLS connection");
        let tcp = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        tcp.set_read_timeout(Some(Duration::from_secs(60)))
            .expect("a read timeout");
        let mut stream = rustls::StreamOwned::new(connection, tcp);
        stream
            .write_all(
                format!(
                    "GET {path} HTTP/1.1\r\nHost: localhost:{port}\r\n\
                     Connection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .expect("send");
        let mut bytes = Vec::new();
        match stream.read_to_end(&mut bytes) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(error) => panic!("read over TLS: {error}"),
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        text.split(' ')
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
            .unwrap_or(0)
    };
    assert_eq!(status("/healthz"), 200);
    assert_eq!(status("/readyz"), 200);
    assert_eq!(status("/metrics"), 200);
    assert_eq!(
        status("/v1/instances"),
        401,
        "a route answered without a bearer"
    );
}
