//! `story:federated-views` (design architecture-next § 5.2): the global API answers `sessions`,
//! `decisions`, `unanswered` and `kpis` as the union of every instance's rows, each row carrying
//! `instance`, and streams `events` as SSE with the same column; an instance that does not answer
//! within `daemon.timeout` is one row with `state: unreachable` and the time it was last seen, and
//! the view is still `200`. `ask` is refused by name until a supervisor runs (decision F4).
//! `conductor status` with no instance prints the federated view (F5). The same unit wires
//! `GET /v1/secrets` and the instance API's `agents/{name}/*` routes, proxied by the daemon.
//!
//! The instances are fakes in this process: a Unix socket each that answers canned JSON, and one
//! that accepts and never answers. Everything is under this test target's temporary directory,
//! kept short so a socket's path fits; nothing is read from the environment.

mod active;
mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use conductor_cli::agent::{Capability, Context, Handle, Kind, Refused, Report, Runtime, Slot};
use conductor_cli::daemon::api::{self, Api, Route};
use conductor_cli::dashboard::{self, Sources};
use conductor_model::agent::{AgentKindName, AgentName};
use conductor_model::config::{InstanceName, Limits, PlacementName};
use serde_json::{Value, json};

/// The daemon's own token, the one its page carries.
const DAEMON_TOKEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// The token of an instance in these cases.
const INSTANCE_TOKEN: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// When the daemon last saw the silent instance and the stopped one.
const GAMMA_SEEN: &str = "2026-11-08T09:00:00Z";
const DELTA_SEEN: &str = "2026-11-08T08:00:00Z";

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
            .join("fv")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the case's directory");
        Self { dir }
    }

    fn socket(&self, instance: &str) -> PathBuf {
        self.dir.join(format!("{instance}.sock"))
    }

    fn home(&self) -> PathBuf {
        self.dir.join("h")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    /// A config file with the instances `names` and the daemon section `daemon`.
    fn config(&self, daemon: &str, names: &[&str]) -> PathBuf {
        let mut text = format!("version: conductor.config/2\ndaemon: {daemon}\ninstances:\n");
        for name in names {
            let at = self.dir.join(name);
            text.push_str(&format!(
                "  - name: {name}\n    placement: local\n    session_prefix: {name}\n    \
                 sources: [{{github: example-org}}]\n    \
                 checkouts: {{root: {root:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
                 state: {state:?}\n    cache: {cache:?}\n",
                root = at.join("root"),
                trees = at.join("trees"),
                records = at.join("records"),
                state = at.join("state"),
                cache = at.join("cache"),
            ));
        }
        fs::create_dir_all(self.home()).expect("create the case's home");
        fs::write(self.file(), text).expect("write the config file");
        self.file()
    }

    fn store(&self) -> String {
        format!("sqlite:{}", self.dir.join("shared.sqlite").display())
    }

    /// The fake instances `alpha` and `beta` answering, `gamma` silent and `delta` stopped,
    /// published to a daemon API with `timeout`; answers its loopback port.
    fn federation(&self, timeout: Duration) -> u16 {
        answering(&self.socket("alpha"), "alpha");
        answering(&self.socket("beta"), "beta");
        silent(&self.socket("gamma"));
        let api = Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None).with_timeout(timeout);
        let route = |name: &str| {
            (
                name.to_owned(),
                Route {
                    socket: self.socket(name),
                    token: INSTANCE_TOKEN.to_owned(),
                    remote: None,
                },
            )
        };
        api.publish(
            vec![
                json!({"name": "alpha", "state": "Running", "last_seen": "2026-11-08T10:00:00Z"}),
                json!({"name": "beta", "state": "Running", "last_seen": "2026-11-08T10:00:00Z"}),
                json!({"name": "gamma", "state": "Running", "last_seen": GAMMA_SEEN}),
                json!({"name": "delta", "state": "Unreachable", "last_seen": DELTA_SEEN}),
            ],
            [route("alpha"), route("beta"), route("gamma")]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
            Vec::new(),
        );
        listen(Arc::new(api))
    }
}

/// Serves `api` on a loopback listener of its own; answers its port.
fn listen(api: Arc<Api>) -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || api::serve(&listener, &api));
    port
}

/// What the fake instance `name` answers on `path`.
fn canned(name: &str, path: &str) -> Value {
    match path {
        "/v1/sessions" => json!({"rows": [{"name": format!("{name}-one")}], "errors": []}),
        "/v1/decisions" => {
            json!({"open": [{"request_id": format!("{name}-r1")}], "history": [], "errors": []})
        }
        "/v1/unanswered" => json!({"count": 1, "oldest": null}),
        "/v1/kpis" => json!([{"id": "k1", "value": 1}, {"id": "k2", "value": 2}]),
        "/v1/events" => json!({"rows": [{
            "at": "2026-11-08T10:00:00Z", "kind": "merge", "text": format!("{name} #1"),
        }]}),
        _ => json!({"error": "not found"}),
    }
}

/// A fake instance on `socket` answering [`canned`] rows.
fn answering(socket: &Path, name: &'static str) {
    let listener = UnixListener::bind(socket).expect("bind a fake instance");
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut stream = stream;
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    head.push(byte[0]);
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                let target = head.split(' ').nth(1).unwrap_or_default();
                let path = target.split('?').next().unwrap_or_default();
                let body = canned(name, path).to_string();
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
            });
        }
    });
}

/// A fake instance on `socket` that accepts and never answers.
fn silent(socket: &Path) {
    let listener = UnixListener::bind(socket).expect("bind a silent instance");
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming().flatten() {
            held.push(stream);
        }
    });
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

fn get(port: u16, target: &str) -> Answer {
    over_tcp(port, &raw("GET", target, &[], None))
}

/// A loopback write from the listener's own page with the daemon's token.
fn own_write(port: u16, target: &str, body: &Value) -> Answer {
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://127.0.0.1:{port}");
    over_tcp(
        port,
        &raw(
            "POST",
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

fn rows_of<'a>(answer: &'a Value, instance: &str) -> Vec<&'a Value> {
    answer["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("no rows: {answer}"))
        .iter()
        .filter(|row| row["instance"] == instance)
        .collect()
}

/// The unreachable rows of the silent and the stopped instance: one each, with the time each was
/// last seen.
fn assert_unreachable(answer: &Value) {
    for (name, seen) in [("gamma", GAMMA_SEEN), ("delta", DELTA_SEEN)] {
        let rows = rows_of(answer, name);
        assert_eq!(rows.len(), 1, "one row for {name}: {answer}");
        assert_eq!(rows[0]["state"], "unreachable", "{answer}");
        assert_eq!(rows[0]["last_seen"], seen, "{answer}");
    }
}

#[test]
fn the_views_are_the_union_of_every_instance_with_an_unreachable_one_as_a_row() {
    let case = Case::new("union");
    let port = case.federation(Duration::from_secs(1));

    let started = Instant::now();
    let sessions = get(port, "/v1/sessions?repository=example");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the instances are asked at once, each within daemon.timeout: {:?}",
        started.elapsed()
    );
    assert_eq!(sessions.status, 200, "{}", sessions.body);
    let sessions = sessions.json();
    assert_eq!(rows_of(&sessions, "alpha")[0]["name"], "alpha-one");
    assert_eq!(rows_of(&sessions, "beta")[0]["name"], "beta-one");
    assert_unreachable(&sessions);

    let decisions = get(port, "/v1/decisions");
    assert_eq!(decisions.status, 200, "{}", decisions.body);
    let decisions = decisions.json();
    assert_eq!(rows_of(&decisions, "alpha")[0]["request_id"], "alpha-r1");
    assert_eq!(rows_of(&decisions, "beta")[0]["request_id"], "beta-r1");
    assert_unreachable(&decisions);

    let unanswered = get(port, "/v1/unanswered");
    assert_eq!(unanswered.status, 200, "{}", unanswered.body);
    let unanswered = unanswered.json();
    assert_eq!(rows_of(&unanswered, "alpha")[0]["count"], 1);
    assert_eq!(rows_of(&unanswered, "beta")[0]["count"], 1);
    assert_unreachable(&unanswered);

    let kpis = get(port, "/v1/kpis");
    assert_eq!(kpis.status, 200, "{}", kpis.body);
    let kpis = kpis.json();
    assert_eq!(rows_of(&kpis, "alpha").len(), 2, "{kpis}");
    assert_eq!(rows_of(&kpis, "beta").len(), 2, "{kpis}");
    assert_unreachable(&kpis);
}

#[test]
fn events_stream_every_instances_events_as_sse_with_the_instance_column() {
    let case = Case::new("events");
    let port = case.federation(Duration::from_secs(1));
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the daemon");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .expect("a read timeout");
    stream
        .write_all(&raw("GET", "/v1/events", &[], None))
        .expect("send");
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read the head");
        if line == "\r\n" || line.is_empty() {
            break;
        }
        head.push_str(&line);
    }
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "{head}"
    );
    let mut seen: BTreeMap<String, Value> = BTreeMap::new();
    let until = Instant::now() + Duration::from_secs(20);
    while seen.len() < 4 && Instant::now() < until {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("read an event") == 0 {
            break;
        }
        if let Some(data) = line.trim_end().strip_prefix("data: ") {
            let row: Value = serde_json::from_str(data).expect("an event's data is JSON");
            let name = row["instance"].as_str().expect("instance").to_owned();
            seen.insert(name, row);
        }
    }
    assert_eq!(seen["alpha"]["text"], "alpha #1", "{seen:?}");
    assert_eq!(seen["beta"]["text"], "beta #1", "{seen:?}");
    assert_eq!(seen["gamma"]["state"], "unreachable", "{seen:?}");
    assert_eq!(seen["delta"]["state"], "unreachable", "{seen:?}");
    assert_eq!(seen["delta"]["last_seen"], DELTA_SEEN, "{seen:?}");
}

#[test]
fn ask_is_refused_by_name_until_a_supervisor_runs() {
    let case = Case::new("ask");
    let port = case.federation(Duration::from_secs(1));
    let refused = own_write(
        port,
        "/v1/ask",
        &json!({"text": "which repository is red?"}),
    );
    assert_eq!(refused.status, 503, "{}", refused.body);
    let body = refused.json();
    assert_eq!(
        body["error"], "conductor.instance.SupervisorNotRunning",
        "{body}"
    );
    assert_eq!(body["instance"], Value::Null, "{body}");
    let to = own_write(
        port,
        "/v1/ask",
        &json!({"text": "is the build green?", "to": "alpha"}),
    );
    assert_eq!(to.status, 503, "{}", to.body);
    assert_eq!(to.json()["instance"], "alpha", "{}", to.body);
    let unknown = own_write(port, "/v1/ask", &json!({"text": "hello", "to": "omega"}));
    assert_eq!(unknown.status, 404, "{}", unknown.body);
    assert_eq!(
        unknown.json()["error"],
        "conductor.instance.UnknownInstance"
    );
    let foreign = over_tcp(
        port,
        &raw("POST", "/v1/ask", &[], Some(&json!({"text": "hello"}))),
    );
    assert_eq!(
        foreign.status, 403,
        "a write needs the daemon's token: {}",
        foreign.body
    );
}

#[test]
fn daemon_timeout_is_5s_unless_the_file_writes_one() {
    let case = Case::new("timeout-key");
    let shown = |daemon: &str| {
        case.config(daemon, &["alpha"]);
        let text = fs::read_to_string(case.file()).expect("read the config file");
        conductor_cli::config::parse(&text, &case.home())
            .map(|config| conductor_cli::config::seconds(&config.daemon.timeout))
    };
    let store = case.store();
    assert_eq!(shown(&format!("{{store: {store:?}}}")), Ok(Some(5)));
    assert_eq!(
        shown(&format!("{{store: {store:?}, timeout: 2s}}")),
        Ok(Some(2))
    );
    let problems = shown(&format!("{{store: {store:?}, timeout: soon}}")).expect_err("refused");
    assert!(
        problems
            .iter()
            .any(|problem| problem.to_string().contains("daemon.timeout")),
        "{problems:?}"
    );
}

#[test]
fn conductor_status_with_no_instance_prints_the_federated_view() {
    let case = Case::new("status");
    let port = case.federation(Duration::from_secs(1));
    let file = case.config(
        &format!("{{store: {:?}, port: {port}, timeout: 1s}}", case.store()),
        &["alpha", "beta", "gamma", "delta"],
    );
    let output = common::conductor(case.home())
        .current_dir(&case.dir)
        .arg("--config")
        .arg(&file)
        .args(["status", "--format", "json"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let view: Value = serde_json::from_str(&stdout).expect("the view is JSON");
    let rows = view["instances"].as_array().expect("instances");
    let row = |name: &str| {
        rows.iter()
            .find(|row| row["instance"] == name)
            .unwrap_or_else(|| panic!("no row for {name}: {view}"))
    };
    assert_eq!(row("alpha")["unanswered"], 1, "{view}");
    assert_eq!(row("beta")["unanswered"], 1, "{view}");
    assert_eq!(row("gamma")["state"], "unreachable", "{view}");
    assert_eq!(row("delta")["state"], "unreachable", "{view}");
    assert_eq!(row("delta")["last_seen"], DELTA_SEEN, "{view}");

    let text = common::conductor(case.home())
        .current_dir(&case.dir)
        .arg("--config")
        .arg(&file)
        .arg("status")
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.lines()
            .any(|line| line.starts_with("delta") && line.contains("unreachable")),
        "{text}"
    );
}

#[test]
fn get_v1_secrets_lists_names_sources_and_instances_never_values() {
    let case = Case::new("secrets");
    let file = case.config(
        &format!("{{store: {:?}}}", case.store()),
        &["alpha", "beta"],
    );
    let text = fs::read_to_string(&file)
        .expect("read the config file")
        .replace(
            "  - name: alpha\n",
            "  - name: alpha\n    forge: {token: \"secret:example-forge-token\"}\n",
        );
    fs::write(&file, &text).expect("write the config file");
    let api = Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None).with_config(
        file,
        case.home(),
        case.store(),
    );
    let port = listen(Arc::new(api));
    let answer = get(port, "/v1/secrets");
    assert_eq!(answer.status, 200, "{}", answer.body);
    let rows = answer.json();
    let rows = rows.as_array().expect("rows");
    let row = rows
        .iter()
        .find(|row| row["name"] == "example-forge-token")
        .unwrap_or_else(|| panic!("no row: {rows:?}"));
    assert_eq!(row["source"], "keychain");
    assert_eq!(row["instances"], json!(["alpha"]));
    assert!(row.get("value").is_none(), "{row}");
}

// ---------------------------------------------------------------------------------------------
// The agent routes of the instance API, proxied by the daemon
// ---------------------------------------------------------------------------------------------

/// An interactive kind that records what it is given.
#[derive(Default)]
struct Recording {
    delivered: Arc<Mutex<Vec<String>>>,
    starts: Arc<Mutex<u32>>,
}

impl Kind for Recording {
    fn name(&self) -> AgentKindName {
        AgentKindName("example-interactive".to_owned())
    }

    fn binary(&self) -> String {
        "true".to_owned()
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Interactive]
    }

    fn start(&self, _context: &Context<'_>, slot: &Slot) -> Result<Handle, Refused> {
        *self.starts.lock().expect("starts") += 1;
        Ok(Handle {
            name: slot.name.clone(),
            kind: self.name(),
            id: None,
        })
    }

    fn deliver(&self, _context: &Context<'_>, _handle: &Handle, text: &str) -> Result<(), Refused> {
        self.delivered
            .lock()
            .expect("delivered")
            .push(text.to_owned());
        Ok(())
    }

    fn reports(&self, _context: &Context<'_>, _handle: &Handle) -> Result<Vec<Report>, Refused> {
        Ok(Vec::new())
    }

    fn stop(&self, _context: &Context<'_>, _handle: &Handle) -> Result<(), Refused> {
        Ok(())
    }
}

/// The instance API of `case` served in this process on its socket, with the agent `ex-alpha` of
/// the kind [`Recording`] running.
fn serve_instance_with_agent(case: &Case) -> (Arc<Mutex<Vec<String>>>, Arc<Mutex<u32>>) {
    let home = case.home();
    let records = home.join("r");
    fs::create_dir_all(records.join("dispatches")).expect("create dispatches/");
    fs::create_dir_all(records.join("decisions")).expect("create decisions/");
    fs::create_dir_all(home.join("watch")).expect("create watch/");
    fs::write(home.join("agents.json"), "[]").expect("write the session list");
    let mut sources = Sources::new(home.clone(), records);
    let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
    sources.agents = cat(&home.join("agents.json"));
    sources.sweep = cat(Path::new("/dev/null"));
    sources.gc = cat(Path::new("/dev/null"));
    sources.watch = home.join("watch");
    let kind = Recording::default();
    let (delivered, starts) = (Arc::clone(&kind.delivered), Arc::clone(&kind.starts));
    let mut runtime = Runtime::new(Box::new(|_: &AgentName, _: &str| Ok(())));
    runtime.register(Box::new(kind));
    let providers = conductor_cli::confine::Host;
    let context = Context {
        config: None,
        state: None,
        providers: &providers,
    };
    runtime
        .start(
            &context,
            Slot {
                name: AgentName("ex-alpha".to_owned()),
                instance: InstanceName("example-org".to_owned()),
                role: "controller".to_owned(),
                kind: AgentKindName("example-interactive".to_owned()),
                placement: PlacementName("local".to_owned()),
                profile: None,
                limits: Limits {
                    memory_max: None,
                    cpu_weight: None,
                    tasks_max: None,
                },
            },
        )
        .expect("the agent starts");
    let listener = UnixListener::bind(case.socket("example-org")).expect("bind the socket");
    let state = case.dir.join("s");
    std::thread::spawn(move || {
        dashboard::serve_unix_with_agents(
            listener,
            sources,
            Some(state),
            INSTANCE_TOKEN.to_owned(),
            runtime,
        );
    });
    (delivered, starts)
}

#[test]
fn the_agent_routes_deliver_and_restart_through_the_daemon_and_attach_is_refused_by_name() {
    let case = Case::new("agents");
    let (delivered, starts) = serve_instance_with_agent(&case);
    let api = Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None);
    api.publish(
        vec![json!({"name": "example-org", "state": "Running"})],
        [(
            "example-org".to_owned(),
            Route {
                socket: case.socket("example-org"),
                token: INSTANCE_TOKEN.to_owned(),
                remote: None,
            },
        )]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    let port = listen(Arc::new(api));
    let base = "/v1/instances/example-org/agents";

    let delivered_answer = own_write(
        port,
        &format!("{base}/ex-alpha/deliver"),
        &json!({"text": "hello"}),
    );
    assert_eq!(delivered_answer.status, 200, "{}", delivered_answer.body);
    assert_eq!(
        *delivered.lock().expect("delivered"),
        vec!["hello".to_owned()]
    );

    let restarted = own_write(port, &format!("{base}/ex-alpha/restart"), &json!({}));
    assert_eq!(restarted.status, 200, "{}", restarted.body);
    assert_eq!(restarted.json()["generation"], 2, "{}", restarted.body);
    assert_eq!(*starts.lock().expect("starts"), 2);

    let unknown = own_write(
        port,
        &format!("{base}/ex-omega/deliver"),
        &json!({"text": "hi"}),
    );
    assert_eq!(unknown.status, 404, "{}", unknown.body);
    assert_eq!(
        unknown.json()["error"],
        "conductor.agent.AgentNotFound",
        "{}",
        unknown.body
    );

    let attach = get(port, &format!("{base}/ex-alpha/attach"));
    assert_eq!(attach.status, 501, "{}", attach.body);
    assert!(
        attach.json()["error"]
            .as_str()
            .is_some_and(|error| error.contains("conductor agent attach")),
        "{}",
        attach.body
    );

    // A write without the daemon's token is refused before it reaches the instance.
    let foreign = over_tcp(
        port,
        &raw(
            "POST",
            &format!("{base}/ex-alpha/deliver"),
            &[],
            Some(&json!({"text": "sneaky"})),
        ),
    );
    assert_eq!(foreign.status, 403, "{}", foreign.body);
    assert_eq!(delivered.lock().expect("delivered").len(), 1);
}
