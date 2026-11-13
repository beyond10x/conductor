//! `story:instance-api` (design architecture-next § 5): the instance runtime serves the dashboard's
//! routes as the instance API under `/v1/` on `<state>/api.sock`, `config` renamed `settings`; the
//! daemon proxies `/v1/instances/{name}/<route>` to it, serves the dashboard app at `/` on its
//! loopback listener and, at the same time, on a TLS listener with Identity bearers; every write on
//! the daemon's API carries an operator, a bearer without the grant is refused by name, and the
//! record of an accepted write names the operator.
//!
//! The first case runs the built binary's `instance serve`; the others serve the instance API and
//! the daemon's listeners in this process, over a config file, records and a state directory under
//! this test target's temporary directory, kept short so the socket's path fits. The fake Identity
//! service answers `GET /v1/access-authority` from a table of bearers. Nothing is read from the
//! environment.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use conductor_cli::daemon::api::{self, Api, Identity, Route};
use conductor_cli::dashboard::{self, Sources};
use conductor_model::config::{DaemonOperator, OperatorGrant, OperatorRole};
use conductor_model::operator::OperatorName;
use serde_json::{Value, json};

/// The token the daemon minted for the instance in these cases.
const INSTANCE_TOKEN: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// The daemon's own token, the one its page carries.
const DAEMON_TOKEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

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
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("ia")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::create_dir_all(case.state()).expect("create the state directory");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
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

    /// The instance API of this case, served in this process on its socket.
    fn serve_instance(&self) {
        let listener = UnixListener::bind(self.socket()).expect("bind the instance's socket");
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || {
            dashboard::serve_unix(listener, sources, Some(state), INSTANCE_TOKEN.to_owned())
        });
    }

    /// The daemon's API over this case's instance, with `operators` and the loopback operator
    /// `owner`.
    fn api(&self, operators: Vec<DaemonOperator>, identity: Option<Identity>) -> Arc<Api> {
        let api = Arc::new(Api::new(
            DAEMON_TOKEN.to_owned(),
            "owner".to_owned(),
            identity,
        ));
        api.publish(
            vec![json!({"name": "example-org", "state": "Running"})],
            [(
                "example-org".to_owned(),
                Route {
                    socket: self.socket(),
                    token: INSTANCE_TOKEN.to_owned(),
                    remote: None,
                    tcp: None,
                },
            )]
            .into_iter()
            .collect(),
            operators,
        );
        api
    }

    /// Every decision of the case's store, as the built binary lists them.
    fn decisions(&self) -> Vec<Value> {
        let output = common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.state())
            .args(["decision", "decisions", "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        if !output.status.success() {
            return Vec::new();
        }
        serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
    }
}

fn operator(name: &str, grants: &[OperatorGrant]) -> DaemonOperator {
    DaemonOperator {
        name: OperatorName(name.to_owned()),
        role: OperatorRole("owner".to_owned()),
        grants: grants.to_vec(),
    }
}

/// An HTTP answer: its status, its head and its body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {} {}", self.head, self.body))
    }

    fn header(&self, name: &str) -> Option<String> {
        self.head.split("\r\n").find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    }
}

fn parse(bytes: &[u8]) -> Answer {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    Answer {
        status,
        head: head.to_owned(),
        body: body.to_owned(),
    }
}

/// A raw request: `method target`, the headers given, and a JSON body when one is given.
fn raw(method: &str, target: &str, headers: &[(&str, &str)], body: Option<&Value>) -> Vec<u8> {
    let body = body.map(|body| serde_json::to_vec(body).expect("a JSON body"));
    let mut raw = format!("{method} {target} HTTP/1.1\r\nConnection: close\r\n").into_bytes();
    for (name, value) in headers {
        raw.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    if let Some(body) = &body {
        raw.extend_from_slice(
            format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            )
            .as_bytes(),
        );
    }
    raw.extend_from_slice(b"\r\n");
    if let Some(body) = body {
        raw.extend_from_slice(&body);
    }
    raw
}

fn over_socket(socket: &Path, request: &[u8]) -> Answer {
    let mut stream = UnixStream::connect(socket).expect("connect to the instance's socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    stream.write_all(request).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    parse(&bytes)
}

fn over_tcp(port: u16, request: &[u8]) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the daemon");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    stream.write_all(request).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    parse(&bytes)
}

/// Sends `request` until its answer is not `503` (an answer still computed), for up to 30 s.
fn settled(send: impl Fn() -> Answer) -> Answer {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        let answer = send();
        if answer.status != 503 || Instant::now() > until {
            return answer;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn disk_low(check: bool) -> Value {
    json!({
        "changes": [{"path": "thresholds.disk_low", "value": "200G"}],
        "reason": "the host has a larger disk now",
        "check": check,
    })
}

/// A child process killed when the case ends.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A runtime in a substrate box: substrate ends its session at the session's output bound, so
/// the runtime writes what it says to the file `CONDUCTOR_API_LOG` names, creating its
/// directory, and nothing to its standard output or error.
#[test]
fn instance_serve_writes_its_output_to_the_log_file_it_is_handed() {
    let case = Case::new("log");
    let log = case.state().join("sessions").join("runtime.log");
    let child = common::conductor(case.home())
        .current_dir(case.state())
        .arg("--config")
        .arg(case.file())
        .args(["instance", "serve"])
        .env(conductor_cli::instance::LOOP_VARIABLE, "off")
        .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
        .env(conductor_cli::instance::TOKEN_VARIABLE, INSTANCE_TOKEN)
        .env(conductor_cli::instance::LOG_VARIABLE, &log)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("instance serve starts");
    let mut running = Running(child);
    let until = Instant::now() + Duration::from_secs(30);
    while !fs::read_to_string(&log).is_ok_and(|text| text.contains("serving /v1/")) {
        if let Ok(Some(status)) = running.0.try_wait() {
            panic!("instance serve exited {status} before it logged");
        }
        assert!(
            Instant::now() < until,
            "nothing logged at {}",
            log.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = running.0.kill();
    let _ = running.0.wait();
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut pipe) = running.0.stdout.take() {
        let _ = pipe.read_to_string(&mut out);
    }
    if let Some(mut pipe) = running.0.stderr.take() {
        let _ = pipe.read_to_string(&mut err);
    }
    assert_eq!(out, "", "nothing on standard output");
    assert_eq!(err, "", "nothing on standard error");
    let mode = fs::metadata(&log).expect("the log").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the log is the owner's only");
}

/// A runtime that cannot start says why in the log file it is handed: in a box its standard
/// error is substrate's session output, which the daemon does not read
/// (`story:box-crash-loop-reads-failed`).
#[test]
fn instance_serve_writes_an_error_before_it_serves_to_the_log_file_it_is_handed() {
    let case = Case::new("log-early");
    let log = case.state().join("sessions").join("runtime.log");
    let broken = case.home().join("broken.yaml");
    fs::write(&broken, "version: conductor.config/2\ninstances: 7\n").expect("write the file");
    let output = common::conductor(case.home())
        .current_dir(case.state())
        .arg("--config")
        .arg(&broken)
        .args(["instance", "serve"])
        .env(conductor_cli::instance::LOOP_VARIABLE, "off")
        .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
        .env(conductor_cli::instance::TOKEN_VARIABLE, INSTANCE_TOKEN)
        .env(conductor_cli::instance::LOG_VARIABLE, &log)
        .stdin(Stdio::null())
        .output()
        .expect("instance serve runs");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        text.contains("conductor: ")
            && text.contains("instance serve")
            && text.contains("broken.yaml"),
        "the log holds the error: {text:?}; standard error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "nothing on standard error"
    );
}

#[test]
fn instance_serve_answers_the_v1_routes_on_its_socket_with_settings_and_the_token_rule() {
    let case = Case::new("serve");
    let child = common::conductor(case.home())
        .current_dir(case.state())
        .arg("--config")
        .arg(case.file())
        .args(["instance", "serve"])
        .env(conductor_cli::instance::LOOP_VARIABLE, "off")
        .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
        .env(conductor_cli::instance::TOKEN_VARIABLE, INSTANCE_TOKEN)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("instance serve starts");
    let mut running = Running(child);
    let until = Instant::now() + Duration::from_secs(30);
    while UnixStream::connect(case.socket()).is_err() {
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
            case.socket().display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let socket = case.socket();
    let get = |target: &str| over_socket(&socket, &raw("GET", target, &[], None));

    let settings = get("/v1/settings");
    assert_eq!(settings.status, 200, "{}", settings.body);
    assert_eq!(settings.json()["instance"], "example-org");
    let board = settled(|| get("/v1/board"));
    assert_eq!(board.status, 200, "{}", board.body);
    let decisions = settled(|| get("/v1/decisions"));
    assert_eq!(decisions.status, 200, "{}", decisions.body);
    for gone in [
        "/api/board",
        "/api/config",
        "/v1/config",
        "/v1/instances",
        "/",
    ] {
        assert_eq!(
            get(gone).status,
            404,
            "{gone} is no route of the instance API"
        );
    }
    let post = over_socket(
        &socket,
        &raw("POST", "/v1/settings", &[], Some(&disk_low(true))),
    );
    assert_eq!(post.status, 405, "{}", post.body);
    let untokened = over_socket(
        &socket,
        &raw("PUT", "/v1/settings", &[], Some(&disk_low(true))),
    );
    assert_eq!(untokened.status, 403, "{}", untokened.body);
    let checked = over_socket(
        &socket,
        &raw(
            "PUT",
            "/v1/settings",
            &[("X-Conductor-Token", INSTANCE_TOKEN)],
            Some(&disk_low(true)),
        ),
    );
    assert_eq!(checked.status, 200, "{}", checked.body);
    assert!(
        checked.json()["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("+ thresholds.disk_low: 200G")),
        "{}",
        checked.body
    );
}

#[test]
fn the_per_instance_dashboard_server_is_gone() {
    let case = Case::new("gone");
    let output = common::conductor(case.home())
        .args(["dashboard", "serve"])
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand"),
        "dashboard serve is no command any more: {stderr}"
    );
    let taskfile = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Taskfile.yml"))
        .expect("read the Taskfile");
    assert!(
        !taskfile.contains("dashboard serve") && !taskfile.contains("-conductor-dashboard"),
        "no task starts a per-instance dashboard or its unit"
    );
}

#[test]
fn the_daemon_proxies_an_instance_route_and_a_browser_holds_the_daemons_token_only() {
    let case = Case::new("proxy");
    case.serve_instance();
    let api = case.api(Vec::new(), None);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&api);
    std::thread::spawn(move || api::serve(&listener, &served));
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://127.0.0.1:{port}");

    let page = over_tcp(port, &raw("GET", "/", &[("Host", &host)], None));
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(page.body.contains("<div id=\"app\">") || page.body.contains("id=\"app\""));
    assert!(
        page.body.contains(&format!(
            r#"<meta name="conductor-token" content="{DAEMON_TOKEN}">"#
        )),
        "the page carries the daemon's token: {}",
        page.body
    );
    assert!(!page.body.contains(INSTANCE_TOKEN));

    let direct = over_socket(&case.socket(), &raw("GET", "/v1/settings", &[], None));
    // The case's file is a `conductor.config/2` one: its settings are that version's keys, not
    // those of the version-1 file the page builds to read each key's default.
    let shown = direct.json();
    let paths: Vec<&str> = shown["fields"]
        .as_array()
        .expect("the settings' fields")
        .iter()
        .filter_map(|field| field["path"].as_str())
        .filter(|path| ["placement", "dashboard_port"].contains(path))
        .collect();
    assert_eq!(paths, ["placement"], "{}", direct.body);
    let proxied = over_tcp(
        port,
        &raw(
            "GET",
            "/v1/instances/example-org/settings",
            &[("Host", &host)],
            None,
        ),
    );
    assert_eq!(proxied.status, 200, "{}", proxied.body);
    assert_eq!(proxied.json(), direct.json());
    let unknown = over_tcp(
        port,
        &raw(
            "GET",
            "/v1/instances/other/settings",
            &[("Host", &host)],
            None,
        ),
    );
    assert_eq!(unknown.status, 404, "{}", unknown.body);
    let rebound = over_tcp(
        port,
        &raw(
            "GET",
            "/v1/instances/example-org/settings",
            &[("Host", "attacker.example:80")],
            None,
        ),
    );
    assert_eq!(rebound.status, 403, "{}", rebound.body);

    // The instance's token is the daemon's to hold: a browser that presents it is refused.
    let write = |token: &str| {
        over_tcp(
            port,
            &raw(
                "PUT",
                "/v1/instances/example-org/settings",
                &[
                    ("Host", &host),
                    ("Origin", &origin),
                    ("X-Conductor-Token", token),
                ],
                Some(&disk_low(false)),
            ),
        )
    };
    assert_eq!(write(INSTANCE_TOKEN).status, 403);
    let written = write(DAEMON_TOKEN);
    assert_eq!(written.status, 200, "{}", written.body);
    let decisions = case.decisions();
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    assert_eq!(decisions[0]["actor"], "human:owner", "{}", decisions[0]);
}

#[test]
fn a_loopback_write_by_an_operator_without_the_grant_is_refused_by_name() {
    let case = Case::new("grant");
    case.serve_instance();
    let api = case.api(
        vec![operator("owner", &[OperatorGrant::AnswerDecisions])],
        None,
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&api);
    std::thread::spawn(move || api::serve(&listener, &served));
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://127.0.0.1:{port}");
    let refused = over_tcp(
        port,
        &raw(
            "PUT",
            "/v1/instances/example-org/settings",
            &[
                ("Host", &host),
                ("Origin", &origin),
                ("X-Conductor-Token", DAEMON_TOKEN),
            ],
            Some(&disk_low(false)),
        ),
    );
    assert_eq!(refused.status, 403, "{}", refused.body);
    let body = refused.json();
    assert_eq!(body["error"], "conductor.operator.GrantMissing", "{body}");
    assert_eq!(body["operator"], "owner", "{body}");
    assert_eq!(body["grant"], "ChangeSettings", "{body}");
    assert!(case.decisions().is_empty());
}

/// The fake Identity service: `GET /v1/access-authority` answers the authority of the bearers
/// `owner`, `reader` and `robot`, and 401 for any other.
fn identity() -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the fake Identity");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                match stream.read(&mut byte) {
                    Ok(1) => head.push(byte[0]),
                    _ => break,
                }
            }
            let head = String::from_utf8_lossy(&head).to_lowercase();
            let bearer = head
                .lines()
                .find_map(|line| line.strip_prefix("authorization: bearer "))
                .map(str::trim)
                .unwrap_or_default()
                .to_owned();
            let audience = head.contains("x-b10x-audience: conductor");
            let authority = |email: &str, kind: &str| {
                json!({
                    "iss": "https://identity.example-org.test", "sub": format!("sub-{email}"),
                    "aud": "conductor", "iat": 0, "nbf": 0, "exp": i64::MAX,
                    "jti": "j", "act": {"sub": format!("sub-{email}")}, "scope": "",
                    "principal_kind": kind, "tenant_id": "t", "email": email, "groups": [],
                })
            };
            let body = match bearer.as_str() {
                "owner" if audience => Some(authority("owner@example-org.test", "human")),
                "reader" if audience => Some(authority("reader@example-org.test", "human")),
                "robot" if audience => Some(authority("robot@example-org.test", "service")),
                _ => None,
            };
            let answer = match body {
                Some(body) => {
                    let body = body.to_string();
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: \
                         no-store\r\nPragma: no-cache\r\nContent-Length: {}\r\nConnection: \
                         close\r\n\r\n{body}",
                        body.len()
                    )
                }
                None => {
                    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_owned()
                }
            };
            let _ = stream.write_all(answer.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}/")
}

/// A request over TLS to `port`, trusting `certificate` for `localhost`.
fn over_tls(port: u16, certificate: &[u8], request: &[u8]) -> Answer {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            certificate.to_vec(),
        ))
        .expect("trust the case's certificate");
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    let server = rustls::pki_types::ServerName::try_from("localhost").expect("a server name");
    let connection =
        rustls::ClientConnection::new(Arc::new(config), server).expect("a TLS connection");
    let tcp = TcpStream::connect(("127.0.0.1", port)).expect("connect to the TLS listener");
    tcp.set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    let mut tls = rustls::StreamOwned::new(connection, tcp);
    tls.write_all(request).expect("send over TLS");
    let mut bytes = Vec::new();
    match tls.read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => {}
        Err(error) => panic!("read over TLS: {error}"),
    }
    parse(&bytes)
}

#[test]
fn the_dashboard_is_served_on_loopback_and_on_the_tls_listener_with_bearers_at_once() {
    let case = Case::new("tls");
    case.serve_instance();
    let identity = Identity::new(&identity(), "conductor").expect("an Identity origin");
    let api = case.api(
        vec![
            operator(
                "owner@example-org.test",
                &[
                    OperatorGrant::ChangeSettings,
                    OperatorGrant::AnswerDecisions,
                ],
            ),
            operator("reader@example-org.test", &[]),
        ],
        Some(identity),
    );
    let generated =
        rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("a certificate");
    fs::write(case.dir.join("cert.pem"), generated.cert.pem()).expect("write the certificate");
    fs::write(case.dir.join("key.pem"), generated.key_pair.serialize_pem()).expect("write the key");
    let tls = api::tls_config(&case.dir.join("cert.pem"), &case.dir.join("key.pem"))
        .expect("the TLS listener's config");
    let certificate = generated.cert.der().to_vec();

    let loopback = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let loopback_port = loopback.local_addr().expect("address").port();
    let secure = TcpListener::bind(("127.0.0.1", 0)).expect("bind the TLS listener");
    let tls_port = secure.local_addr().expect("address").port();
    let (one, two) = (Arc::clone(&api), Arc::clone(&api));
    std::thread::spawn(move || api::serve(&loopback, &one));
    std::thread::spawn(move || api::serve_tls(&secure, &two, &tls));

    // The daemon's own page, on loopback.
    let host = format!("127.0.0.1:{loopback_port}");
    let page = over_tcp(loopback_port, &raw("GET", "/", &[("Host", &host)], None));
    assert_eq!(page.status, 200, "{}", page.body);

    // The same app, deployed elsewhere: its origin is another, and it reads the listener's URL
    // with a bearer.
    let elsewhere = "https://dashboard.example-org.test";
    let tls_host = format!("localhost:{tls_port}");
    let preflight = over_tls(
        tls_port,
        &certificate,
        &raw(
            "OPTIONS",
            "/v1/instances/example-org/board",
            &[
                ("Host", &tls_host),
                ("Origin", elsewhere),
                ("Access-Control-Request-Method", "GET"),
                ("Access-Control-Request-Headers", "authorization"),
            ],
            None,
        ),
    );
    assert_eq!(preflight.status, 204, "{}", preflight.head);
    assert!(
        preflight
            .header("access-control-allow-headers")
            .is_some_and(|allowed| allowed.to_lowercase().contains("authorization")),
        "{}",
        preflight.head
    );
    let anonymous = over_tls(
        tls_port,
        &certificate,
        &raw("GET", "/v1/instances", &[("Host", &tls_host)], None),
    );
    assert_eq!(anonymous.status, 401, "{}", anonymous.body);
    let listed = over_tls(
        tls_port,
        &certificate,
        &raw(
            "GET",
            "/v1/instances",
            &[
                ("Host", &tls_host),
                ("Origin", elsewhere),
                ("Authorization", "Bearer reader"),
            ],
            None,
        ),
    );
    assert_eq!(listed.status, 200, "{}", listed.body);
    assert_eq!(listed.json()[0]["name"], "example-org");
    assert!(listed.header("access-control-allow-origin").is_some());
    let read = over_tls(
        tls_port,
        &certificate,
        &raw(
            "GET",
            "/v1/instances/example-org/settings",
            &[("Host", &tls_host), ("Authorization", "Bearer reader")],
            None,
        ),
    );
    assert_eq!(read.status, 200, "{}", read.body);
    assert_eq!(read.json()["instance"], "example-org");
    let tls_page = over_tls(
        tls_port,
        &certificate,
        &raw("GET", "/", &[("Host", &tls_host)], None),
    );
    assert_eq!(tls_page.status, 200, "{}", tls_page.body);
    assert!(
        !tls_page.body.contains("conductor-token"),
        "the TLS listener's page carries no token: bearers only"
    );

    let write = |bearer: &str| {
        over_tls(
            tls_port,
            &certificate,
            &raw(
                "PUT",
                "/v1/instances/example-org/settings",
                &[
                    ("Host", &tls_host),
                    ("Origin", elsewhere),
                    ("Authorization", &format!("Bearer {bearer}")),
                ],
                Some(&disk_low(false)),
            ),
        )
    };
    let reader = write("reader");
    assert_eq!(reader.status, 403, "{}", reader.body);
    assert_eq!(reader.json()["error"], "conductor.operator.GrantMissing");
    assert_eq!(reader.json()["operator"], "reader@example-org.test");
    let robot = write("robot");
    assert_eq!(robot.status, 403, "{}", robot.body);
    assert_eq!(robot.json()["error"], "conductor.operator.ActorNotHuman");
    assert!(case.decisions().is_empty());
    let owner = write("owner");
    assert_eq!(owner.status, 200, "{}", owner.body);
    let decisions = case.decisions();
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    assert_eq!(
        decisions[0]["actor"], "human:owner@example-org.test",
        "{}",
        decisions[0]
    );

    // The built app reads the API's base URL from the page, so the same dist/ runs elsewhere.
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("dashboard/dist/assets");
    let names_base = fs::read_dir(&assets)
        .expect("read the built assets")
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "js"))
        .any(|entry| {
            fs::read_to_string(entry.path()).is_ok_and(|text| text.contains("conductor-api"))
        });
    assert!(
        names_base,
        "the built app reads <meta name=\"conductor-api\">"
    );
}

/// The daemon's API of `case` on a loopback listener of its own, with `operators`, the loopback
/// operator `owner` and the case's config file for `PUT /v1/config`; answers its port.
fn global_api(case: &Case, operators: Vec<DaemonOperator>) -> u16 {
    let store = format!("sqlite:{}", case.dir.join("shared.sqlite").display());
    let api = Arc::new(
        Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None).with_config(
            case.file(),
            case.home(),
            store,
        ),
    );
    api.publish(
        vec![json!({"name": "example-org", "state": "Running"})],
        std::collections::BTreeMap::new(),
        operators,
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || api::serve(&listener, &api));
    port
}

/// A loopback write of `body` to `target` with `method`, from the listener's own page with the
/// daemon's token.
fn own_write(port: u16, method: &str, target: &str, body: &Value) -> Answer {
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://127.0.0.1:{port}");
    over_tcp(
        port,
        &raw(
            method,
            target,
            &[
                ("Host", &host),
                ("Origin", &origin),
                ("X-Conductor-Token", DAEMON_TOKEN),
            ],
            Some(body),
        ),
    )
}

/// `PUT /v1/config` is the global API's write of the config (`story:config-globals`): an
/// operator with ChangeSettings changes the file and the answer names the revision and its
/// operator; one without is refused by name and nothing is written; a refusal of the change itself
/// is a `4xx` naming it on one line.
#[test]
fn put_v1_config_changes_the_file_as_an_operator_with_the_grant() {
    let case = Case::new("put-config");
    let change = json!({
        "level": "global",
        "path": "cadence.watch",
        "value": "5m",
        "reason": "faster watch",
    });

    let reader = global_api(
        &case,
        vec![operator("owner", &[OperatorGrant::AnswerDecisions])],
    );
    let refused = own_write(reader, "PUT", "/v1/config", &change);
    assert_eq!(refused.status, 403, "{}", refused.body);
    assert_eq!(refused.json()["error"], "conductor.operator.GrantMissing");
    assert_eq!(refused.json()["grant"], "ChangeSettings");
    assert!(
        !fs::read_to_string(case.file())
            .expect("read")
            .contains("watch")
    );

    let port = global_api(
        &case,
        vec![operator("owner", &[OperatorGrant::ChangeSettings])],
    );
    let unsent = over_tcp(port, &raw("PUT", "/v1/config", &[], Some(&change)));
    assert_eq!(unsent.status, 403, "{}", unsent.body);

    let changed = own_write(port, "PUT", "/v1/config", &change);
    assert_eq!(changed.status, 200, "{}", changed.body);
    let body = changed.json();
    assert_eq!(body["path"], "cadence.watch", "{body}");
    assert_eq!(body["actor"], "human:owner", "{body}");
    assert!(
        body["revision_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "{body}"
    );
    let text = fs::read_to_string(case.file()).expect("read the config file");
    assert!(text.contains("watch: 5m"), "{text}");

    let mismatch = own_write(
        port,
        "PUT",
        "/v1/config",
        &json!({"level": "instance", "path": "cadence.watch", "value": "5m", "reason": "x"}),
    );
    assert_eq!(mismatch.status, 422, "{}", mismatch.body);
    assert!(
        mismatch.json()["error"]
            .as_str()
            .is_some_and(|error| error.starts_with("LevelScopeMismatch")),
        "{}",
        mismatch.body
    );

    let get = over_tcp(port, &raw("GET", "/v1/config", &[], None));
    assert_eq!(get.status, 405, "{}", get.body);
}

/// story:instance-rules-text: `PUT /v1/config` is no route that writes an instance's
/// `standing_rules`; `POST /v1/instances` and `PUT /v1/instances/{name}` are. A change of the key,
/// set or removed, at the instance level or under `globals`, answers `422` naming its path and
/// those routes, and no part of the offered value; nothing is written or recorded. Another key of
/// the instance still goes through.
#[test]
fn put_v1_config_refuses_the_standing_rules_and_takes_another_instance_key() {
    let case = Case::new("put-config-rules");
    let port = global_api(
        &case,
        vec![operator("owner", &[OperatorGrant::ChangeSettings])],
    );
    let text = fs::read_to_string(case.file()).expect("read");
    let offered = "Zq9 marker rule offered through the config route";
    for (what, change) in [
        (
            "set at the instance",
            json!({"level": "instance", "instance": "example-org", "path": "standing_rules",
                   "value": offered, "reason": "a second writer"}),
        ),
        (
            "removed at the instance",
            json!({"level": "instance", "instance": "example-org", "path": "standing_rules",
                   "reason": "a second writer"}),
        ),
        (
            "set under globals",
            json!({"level": "global", "path": "standing_rules", "value": offered,
                   "reason": "a second writer"}),
        ),
    ] {
        let refused = own_write(port, "PUT", "/v1/config", &change);
        assert_eq!(refused.status, 422, "{what}: {}", refused.body);
        for words in [
            "standing_rules",
            "POST /v1/instances",
            "PUT /v1/instances/{name}",
        ] {
            assert!(
                refused.body.contains(words),
                "{what}: the refusal names {words:?}: {}",
                refused.body
            );
        }
        assert!(!refused.body.contains("Zq9"), "{what}: {}", refused.body);
        assert_eq!(
            fs::read_to_string(case.file()).expect("read"),
            text,
            "{what}"
        );
    }
    let url = format!("sqlite:{}", case.dir.join("shared.sqlite").display());
    if let Ok(store) = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT) {
        let revisions = conductor_model::behaviour::ConfigRevisionStorage::list(&store);
        assert_eq!(revisions.len(), 0, "nothing is recorded");
    }

    let changed = own_write(
        port,
        "PUT",
        "/v1/config",
        &json!({"level": "instance", "instance": "example-org",
                "path": "thresholds.silent_session", "value": "45m", "reason": "another key"}),
    );
    assert_eq!(changed.status, 200, "{}", changed.body);
}

/// `POST /v1/instances/{name}/upgrade` (`story:instance-upgrade`) is a write of the global API
/// like any other: the operator needs UpgradeInstances, and is refused by name without it before
/// anything reaches the loop.
#[test]
fn an_upgrade_needs_an_operator_holding_upgrade_instances() {
    let case = Case::new("upgrade-grant");
    let port = global_api(
        &case,
        vec![operator("owner", &[OperatorGrant::ChangeSettings])],
    );
    let refused = own_write(
        port,
        "POST",
        "/v1/instances/example-org/upgrade",
        &json!({"tag": "v9.9.9"}),
    );
    assert_eq!(refused.status, 403, "{}", refused.body);
    assert_eq!(refused.json()["error"], "conductor.operator.GrantMissing");
    assert_eq!(refused.json()["operator"], "owner");
    assert_eq!(refused.json()["grant"], "UpgradeInstances");

    let unsent = over_tcp(
        port,
        &raw(
            "POST",
            "/v1/instances/example-org/upgrade",
            &[],
            Some(&json!({"tag": "v9.9.9"})),
        ),
    );
    assert_eq!(unsent.status, 403, "{}", unsent.body);
}

/// The variable a runtime alone in a pod is asked for a TCP listener in, by name: the pod
/// supervisor sets it.
const LISTEN: &str = "CONDUCTOR_API_LISTEN";

/// The variable naming the mounted config file a runtime copies over its `--config` file, by name.
const CONFIG_SOURCE: &str = "CONDUCTOR_CONFIG_SOURCE";

/// `conductor instance serve` of a case with the loop off, killed when dropped, with what it
/// printed.
struct Runtime {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Runtime {
    /// Starts the runtime of `case`'s instance over the config file `config`, with `token` as its
    /// token when one is given and `env` added.
    fn start(case: &Case, config: &Path, token: Option<&str>, env: &[(&str, &str)]) -> Self {
        let mut command = common::conductor(case.home());
        command
            .current_dir(case.state())
            .arg("--config")
            .arg(config)
            .args(["instance", "serve"])
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
            .env_remove(conductor_cli::instance::TOKEN_VARIABLE)
            .env_remove(LISTEN)
            .env_remove(CONFIG_SOURCE)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(token) = token {
            command.env(conductor_cli::instance::TOKEN_VARIABLE, token);
        }
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("instance serve starts");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            std::thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        Self { child, printed }
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }

    /// Waits up to 30 s for what it printed to hold `holds`, while it runs.
    fn until(&mut self, what: &str, holds: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let printed = self.printed();
            if holds(&printed) {
                return;
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                panic!("instance serve exited {status} before {what}: {printed}");
            }
            assert!(Instant::now() < deadline, "never held: {what}: {printed}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// What its serving line names, once it printed it.
    fn served(&mut self) -> String {
        self.until("its serving line", |printed| {
            printed.contains("serving /v1/ on ")
        });
        let printed = self.printed();
        printed
            .lines()
            .find_map(|line| line.split_once("serving /v1/ on "))
            .map(|(_, at)| at.trim().to_owned())
            .unwrap_or_default()
    }

    /// The TCP address it serves `/v1/` on, from its serving line.
    fn address(&mut self) -> SocketAddr {
        let served = self.served();
        served.parse().unwrap_or_else(|_| {
            panic!(
                "the runtime serves /v1/ on {served}, no TCP address: {}",
                self.printed()
            )
        })
    }

    /// How it ended, within 30 s.
    fn ended(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "instance serve still runs: {}",
                self.printed()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn over_address(address: SocketAddr, request: &[u8]) -> Answer {
    let mut stream = TcpStream::connect(address).expect("connect to the runtime");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    stream.write_all(request).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    parse(&bytes)
}

/// A runtime alone in a pod (`CONDUCTOR_API_LISTEN`) serves its API on TCP and takes every
/// request with its token, reads too: without it, or with another, the answer is `401` and no
/// route answers (a write too, which on the socket its route refuses `403`). `GET /healthz`, the
/// probe, answers without one; `/v1/health`, the health view's data, is a route like the others.
/// It binds no socket.
#[test]
fn instance_serve_on_tcp_takes_every_request_with_its_token_and_its_probe_without() {
    let case = Case::new("tcp");
    let mut runtime = Runtime::start(
        &case,
        &case.file(),
        Some(INSTANCE_TOKEN),
        &[(LISTEN, "127.0.0.1:0")],
    );
    let address = runtime.address();
    let read = |target: &str, headers: &[(&str, &str)]| {
        over_address(address, &raw("GET", target, headers, None))
    };

    let bare = read("/v1/settings", &[]);
    assert_eq!(bare.status, 401, "a read without the token: {}", bare.body);
    let other = read("/v1/settings", &[("X-Conductor-Token", DAEMON_TOKEN)]);
    assert_eq!(
        other.status, 401,
        "a read with another token: {}",
        other.body
    );
    let tokened = read("/v1/settings", &[("X-Conductor-Token", INSTANCE_TOKEN)]);
    assert_eq!(tokened.status, 200, "{}", tokened.body);
    assert_eq!(tokened.json()["instance"], "example-org");
    let unsent = over_address(
        address,
        &raw("PUT", "/v1/settings", &[], Some(&disk_low(true))),
    );
    assert_eq!(unsent.status, 401, "{}", unsent.body);
    let view = read("/v1/health", &[]);
    assert_eq!(view.status, 401, "{}", view.body);

    let probe = read("/healthz", &[]);
    assert_eq!(probe.status, 200, "{}", probe.body);
    assert_eq!(probe.json(), json!({"state": "serving"}));
    assert!(
        !case.socket().exists(),
        "a runtime on TCP binds no socket: {}",
        runtime.printed()
    );
}

/// A runtime asked for a TCP listener without a token refuses to start: one it minted itself
/// would refuse every request, the daemon's reads too.
#[test]
fn instance_serve_on_tcp_without_a_token_refuses_to_start() {
    let case = Case::new("tcp-untokened");
    let mut runtime = Runtime::start(&case, &case.file(), None, &[(LISTEN, "127.0.0.1:0")]);
    let status = runtime.ended();
    let printed = runtime.printed();
    assert_eq!(status.code(), Some(1), "{printed}");
    assert!(
        printed.contains(LISTEN) && printed.contains(conductor_cli::instance::TOKEN_VARIABLE),
        "the refusal names both variables: {printed}"
    );
}

/// A runtime handed a config source (`CONDUCTOR_CONFIG_SOURCE`, a mounted file it may not write)
/// copies it over its `--config` file at every start before it reads it, creating the file's
/// directory; the copy is its own to write, and a source changed between starts is what the next
/// start reads.
#[test]
fn instance_serve_copies_its_config_source_over_its_config_file_at_every_start() {
    let case = Case::new("source");
    let mounted = case.dir.join("m");
    fs::create_dir_all(&mounted).expect("create the mount");
    let source = mounted.join("conductor.yaml");
    let file = case.dir.join("v").join("conductor.yaml");
    let mount = |low: &str| {
        let text = case.text().replace(
            "    state: ",
            &format!("    thresholds: {{disk_low: {low}}}\n    state: "),
        );
        if source.exists() {
            fs::set_permissions(&source, fs::Permissions::from_mode(0o644))
                .expect("make the source writable");
        }
        fs::write(&source, text).expect("write the source");
        fs::set_permissions(&source, fs::Permissions::from_mode(0o444))
            .expect("make the source read-only");
    };
    let shown = |runtime: &mut Runtime| -> String {
        let served = runtime.served();
        let settings = over_socket(Path::new(&served), &raw("GET", "/v1/settings", &[], None));
        assert_eq!(settings.status, 200, "{}", settings.body);
        let field = settings.json()["fields"]
            .as_array()
            .expect("the settings' fields")
            .iter()
            .find(|field| field["path"] == "thresholds.disk_low")
            .cloned()
            .unwrap_or_else(|| panic!("no thresholds.disk_low: {}", settings.body));
        field["value"].to_string()
    };
    let source_text = source.to_str().expect("a path in UTF-8").to_owned();

    mount("111G");
    let mut runtime = Runtime::start(
        &case,
        &file,
        Some(INSTANCE_TOKEN),
        &[(CONFIG_SOURCE, &source_text)],
    );
    let first = shown(&mut runtime);
    assert!(first.contains("111G"), "the source's value: {first}");
    assert_eq!(
        fs::read(&file).expect("the config file was written"),
        fs::read(&source).expect("the source")
    );
    let mode = fs::metadata(&file).expect("the file").permissions().mode();
    assert_ne!(
        mode & 0o200,
        0,
        "the copy is its owner's to write: {mode:o}"
    );
    drop(runtime);

    mount("222G");
    let mut runtime = Runtime::start(
        &case,
        &file,
        Some(INSTANCE_TOKEN),
        &[(CONFIG_SOURCE, &source_text)],
    );
    let second = shown(&mut runtime);
    assert!(
        second.contains("222G"),
        "the changed source's value: {second}"
    );
    assert_eq!(
        fs::read(&file).expect("the config file"),
        fs::read(&source).expect("the source")
    );
}

/// The daemon's API over the one instance `example-org` behind `route`, on a loopback port of
/// its own; answers the port.
fn daemon_over(route: Route) -> u16 {
    let api = Arc::new(Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None));
    api.publish(
        vec![json!({"name": "example-org", "state": "Running"})],
        [("example-org".to_owned(), route)].into_iter().collect(),
        Vec::new(),
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || api::serve(&listener, &api));
    port
}

/// The daemon's link to a runtime alone in a pod: a route to its TCP address
/// ([`Route::tcp`], what the pod supervisor builds) carries the instance's token on every
/// request, reads included. A proxied read, a federated read and a proxied write answer through
/// it; a route holding another token reads the runtime's `401`.
#[test]
fn the_daemon_reads_and_writes_through_a_tcp_route_with_the_token_on_every_request() {
    let case = Case::new("tcp-proxy");
    let mut runtime = Runtime::start(
        &case,
        &case.file(),
        Some(INSTANCE_TOKEN),
        &[(LISTEN, "127.0.0.1:0")],
    );
    let address = runtime.address();
    let port = daemon_over(Route::tcp(address, INSTANCE_TOKEN.to_owned()));
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://127.0.0.1:{port}");
    let get =
        |port: u16, target: &str| over_tcp(port, &raw("GET", target, &[("Host", &host)], None));

    let proxied = get(port, "/v1/instances/example-org/settings");
    assert_eq!(proxied.status, 200, "{}", proxied.body);
    assert_eq!(proxied.json()["instance"], "example-org");

    let until = Instant::now() + Duration::from_secs(30);
    let row = loop {
        let federated = get(port, "/v1/unanswered");
        assert_eq!(federated.status, 200, "{}", federated.body);
        let row = federated.json()["rows"][0].clone();
        if row["state"] != "unreachable" || Instant::now() > until {
            break row;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(row["instance"], "example-org", "{row}");
    assert_ne!(
        row["state"], "unreachable",
        "a federated read over TCP: {row}"
    );

    let written = over_tcp(
        port,
        &raw(
            "PUT",
            "/v1/instances/example-org/settings",
            &[
                ("Host", &host),
                ("Origin", &origin),
                ("X-Conductor-Token", DAEMON_TOKEN),
            ],
            Some(&disk_low(false)),
        ),
    );
    assert_eq!(written.status, 200, "{}", written.body);

    let other = daemon_over(Route::tcp(address, DAEMON_TOKEN.to_owned()));
    let refused = over_tcp(
        other,
        &raw(
            "GET",
            "/v1/instances/example-org/settings",
            &[("Host", &format!("127.0.0.1:{other}"))],
            None,
        ),
    );
    assert_eq!(refused.status, 401, "{}", refused.body);
}
