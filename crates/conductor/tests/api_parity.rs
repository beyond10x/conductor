//! Parity of the generated HTTP servers with the hand-written API: how far the servers `ess`
//! generates from the specification and its binding (`crates/conductor/http.yaml`,
//! `conductor_model::server`) answer what conductord and the instance runtime answer today.
//!
//! One case serves, in this process, both surfaces over stores seeded alike under this test
//! target's temporary directory: the hand-written daemon API on loopback
//! ([`conductor_cli::daemon::api`]), proxying to a hand-written instance runtime on its socket
//! ([`conductor_cli::dashboard`]); and the generated conductord and instance servers over the
//! store ([`conductor_cli::spec_api`]). Each probe sends both the same request, reads before
//! writes, and compares the status, the shape of the body (its JSON types and member names, never
//! its values) and the change it made to the daemon's store (its rows per entity and the pool's
//! token states).
//!
//! The parity table is printed and compared, line for line, with the committed baseline
//! `tests/fixtures/api_parity.md`: a known difference does not fail the case, a new one does, and
//! so does a difference that went away. `CONDUCTOR_API_PARITY_BLESS=1` writes the table as the new
//! baseline instead.

mod active;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use conductor_cli::daemon::api::{Api, Route};
use conductor_cli::dashboard::{self, Sources};
use conductor_cli::spec_api::{self, Surface};
use conductor_model::behaviour::{
    BoxStorage, ConfigRevisionStorage, MigrationStorage, OperatorStorage, PoolTokenStorage,
    ReconciliationStorage, SecretStorage,
};
use conductor_model::config::InstanceName;
use conductor_model::operator::OperatorName;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::server::{conductor_instance, conductord};
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenState,
};
use serde_json::{Value, json};

/// The daemon's token: its page carries it, and the generated surface admits a write with it.
const DAEMON_TOKEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// The token the daemon minted for the instance.
const INSTANCE_TOKEN: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// The instance of the case.
const INSTANCE: &str = "example-org";

/// The variable that writes the table as the new baseline.
const BLESS: &str = "CONDUCTOR_API_PARITY_BLESS";

/// The case's directory, short so `<state>/api.sock` fits a socket address; removed after a
/// passing run, with the config repository a `PUT /v1/config` commits to.
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
    fn new() -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ap");
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

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    /// The store the hand-written daemon writes (`daemon.store`).
    fn hand_store(&self) -> String {
        format!("sqlite:{}", self.dir.join("hand.sqlite").display())
    }

    /// The store the generated servers write, seeded as the hand-written one is.
    fn spec_store(&self) -> String {
        format!("sqlite:{}", self.dir.join("spec.sqlite").display())
    }

    fn text(&self) -> String {
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: {INSTANCE}\n\
             \x20   placement: local\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: {INSTANCE}}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = self.hand_store(),
            root = self.home().join(INSTANCE),
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
        sources.checkouts = home.join(INSTANCE);
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources.config_file = Some(self.file());
        sources.instance_name = Some(INSTANCE.to_owned());
        sources
    }

    /// The hand-written surfaces: the instance runtime on its socket, and the daemon's loopback
    /// listener proxying to it. Answers the listener's port.
    fn serve_hand(&self) -> u16 {
        let socket = self.state().join("api.sock");
        let listener = UnixListener::bind(&socket).expect("bind the instance's socket");
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || {
            dashboard::serve_unix(listener, sources, Some(state), INSTANCE_TOKEN.to_owned());
        });
        let api = Arc::new(
            Api::new(DAEMON_TOKEN.to_owned(), "operator-one".to_owned(), None)
                .with_config(self.file(), self.home(), self.hand_store())
                .with_timeout(Duration::from_secs(2)),
        );
        api.publish(
            vec![json!({"name": INSTANCE, "state": "Running"})],
            [(
                INSTANCE.to_owned(),
                Route {
                    socket,
                    token: INSTANCE_TOKEN.to_owned(),
                    remote: None,
                    tcp: None,
                },
            )]
            .into_iter()
            .collect(),
            Vec::new(),
        );
        // The daemon publishes the host roles with the instances: the config lists none.
        api.publish_host(Vec::new(), None);
        // The daemon's store answers: `GET /readyz` reads ready. The answer is dated ahead, so it
        // does not age past the readiness bound however long the case runs.
        api.publish_store_at(true, Instant::now() + Duration::from_secs(3600));
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the daemon's listener");
        let port = listener.local_addr().expect("an address").port();
        std::thread::spawn(move || conductor_cli::daemon::api::serve(&listener, &api));
        port
    }

    /// One generated surface over the generated servers' store. Answers its port.
    fn serve_spec(&self, surface: Surface) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind a generated surface");
        let port = listener.local_addr().expect("an address").port();
        let url = self.spec_store();
        std::thread::spawn(move || {
            let store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
                .expect("open the generated servers' store");
            spec_api::serve(&listener, surface, store, DAEMON_TOKEN);
        });
        port
    }
}

/// The one token both stores start with: Available, in the instance's pool.
fn seed(url: &str) {
    let mut store = conductor_cli::store::open_url(url, conductor_cli::daemon::TENANT)
        .expect("open a store to seed");
    PoolTokenStorage::put(
        &mut store,
        PoolTokenSnapshot {
            state: PoolTokenState::Available,
            data: PoolTokenData {
                token_id: TokenId(Uuid("00000000-0000-4000-8000-000000000001".to_owned())),
                instance: InstanceName(INSTANCE.to_owned()),
                owner: OperatorName("operator-one".to_owned()),
                seat: Seat("operator-one@example-org.example".to_owned()),
                secret: SecretName("example-org-claude-operator-one".to_owned()),
                token_state: TokenState::Ok,
                limited_until: None,
                agent: None,
                contributed_at: Timestamp("2026-11-08T09:00:00Z".to_owned()),
                actor: conductor_cli::actor::operator("operator-one"),
            },
        },
    );
    store.check().expect("seed the store");
}

/// What the daemon's store holds: the rows of each entity the daemon's routes write, and the
/// states of the pool's tokens.
fn state(url: &str) -> BTreeMap<&'static str, String> {
    let store = conductor_cli::store::open_url(url, conductor_cli::daemon::TENANT)
        .expect("open a store to read");
    let mut tokens: Vec<String> = store
        .pool_tokens()
        .iter()
        .map(|token| format!("{:?}", token.state))
        .collect();
    tokens.sort();
    BTreeMap::from([
        (
            "ConfigRevision",
            ConfigRevisionStorage::list(&store).len().to_string(),
        ),
        ("Operator", OperatorStorage::list(&store).len().to_string()),
        ("Secret", SecretStorage::list(&store).len().to_string()),
        (
            "Reconciliation",
            ReconciliationStorage::list(&store).len().to_string(),
        ),
        (
            "Migration",
            MigrationStorage::list(&store).len().to_string(),
        ),
        ("Box", BoxStorage::list(&store).len().to_string()),
        ("PoolToken", tokens.join(",")),
    ])
}

/// What a probe changed in a store: each entity whose value moved, `before→after`.
fn change(
    before: &BTreeMap<&'static str, String>,
    after: &BTreeMap<&'static str, String>,
) -> String {
    let moved: Vec<String> = after
        .iter()
        .filter(|(entity, value)| before.get(*entity) != Some(value))
        .map(|(entity, value)| {
            format!(
                "{entity} {}→{value}",
                before.get(entity).map_or("", String::as_str)
            )
        })
        .collect();
    if moved.is_empty() {
        "-".to_owned()
    } else {
        moved.join("; ")
    }
}

/// One HTTP exchange on `port`: the status and the body, as a loopback page of the daemon sends
/// it (its host, its origin and the daemon's token).
fn exchange(port: u16, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    let body = body.unwrap_or("");
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n\
         X-Conductor-Token: {DAEMON_TOKEN}\r\nContent-Type: application/json\r\n\
         Accept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("send");
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    let text = String::from_utf8_lossy(&answer).into_owned();
    let (head, rest) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let rest = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(rest)
    } else {
        rest.to_owned()
    };
    (status, rest)
}

fn dechunk(mut rest: &str) -> String {
    let mut joined = String::new();
    while let Some((size, after)) = rest.split_once("\r\n") {
        let Ok(size) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if size == 0 || after.len() < size {
            break;
        }
        joined.push_str(&after[..size]);
        rest = after.get(size + 2..).unwrap_or_default();
    }
    joined
}

/// The shape of a body: its JSON types and member names, never its values; `text` for a body
/// that is no JSON, `empty` for none.
fn shape(body: &str) -> String {
    match serde_json::from_str::<Value>(body.trim()) {
        Ok(value) => shape_of(&value),
        Err(_) if body.trim().is_empty() => "empty".to_owned(),
        Err(_) => "text".to_owned(),
    }
}

fn shape_of(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Number(_) => "number".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Array(items) => items
            .first()
            .map_or_else(|| "[]".to_owned(), |item| format!("[{}]", shape_of(item))),
        Value::Object(members) => {
            let members: Vec<String> = members
                .iter()
                .map(|(name, value)| format!("{name}:{}", shape_of(value)))
                .collect();
            format!("{{{}}}", members.join(","))
        }
    }
}

/// The top-level members of a body, or its kind: what the table shows of a shape.
fn outline(body: &str) -> String {
    match serde_json::from_str::<Value>(body.trim()) {
        Ok(Value::Object(members)) => {
            let names: Vec<&str> = members.keys().map(String::as_str).collect();
            format!("{{{}}}", names.join(","))
        }
        Ok(Value::Array(items)) => format!("[{}]", items.len()),
        Ok(other) => shape_of(&other),
        Err(_) if body.trim().is_empty() => "empty".to_owned(),
        Err(_) => "text".to_owned(),
    }
}

/// Whether the binding names a route, and which operation.
enum Binding {
    /// The route the binding gives this operation.
    Bound(&'static str),
    /// A hand-written route no operation of the specification is bound to, and why.
    Unbound(&'static str),
}

/// One request both surfaces are sent.
struct Probe {
    /// The route as the hand-written surface serves it.
    route: &'static str,
    binding: Binding,
    /// Which request: `client`, what the dashboard or the CLI sends today; `spec`, the
    /// specification's input from its scenarios.
    case: &'static str,
    method: &'static str,
    /// The path on the daemon's listener.
    hand: String,
    /// The generated surface and the path on it.
    spec: (Surface, String),
    body: Option<String>,
}

fn probes() -> Vec<Probe> {
    let operator = json!({"kind": "Human", "operator": "operator-one"});
    let instance = |route: &str| format!("/v1/instances/{INSTANCE}/{route}");
    let read = |route, binding, path: String| Probe {
        route,
        binding,
        case: "read",
        method: "GET",
        hand: path.clone(),
        spec: (Surface::Conductord, path),
        body: None,
    };
    let write = |route, binding, case, method, path: String, body: Value| Probe {
        route,
        binding,
        case,
        method,
        hand: path.clone(),
        spec: (Surface::Conductord, path),
        body: Some(body.to_string()),
    };
    let agent = |route, binding, case, verb: &str, body: Value| Probe {
        route,
        binding,
        case,
        method: "POST",
        hand: instance(&format!("agents/example-org-planner/{verb}")),
        spec: (
            Surface::Instance,
            format!("/v1/agents/example-org-planner/{verb}"),
        ),
        body: Some(body.to_string()),
    };
    let reads = || {
        vec![
            read(
                "GET /v1/instances",
                Binding::Bound("conductor.instance.Instances"),
                "/v1/instances".to_owned(),
            ),
            read(
                "GET /v1/secrets",
                Binding::Bound("conductor.secret.Secrets"),
                "/v1/secrets".to_owned(),
            ),
            read(
                "GET /v1/instances/{name}/tokens",
                Binding::Bound("conductor.token.Tokens"),
                instance("tokens"),
            ),
            read(
                "GET /v1/instances/{name}/settings",
                Binding::Bound("conductor.config.EffectiveSettings"),
                instance("settings"),
            ),
            read(
                "GET /readyz",
                Binding::Bound("conductor.store.Readiness"),
                "/readyz".to_owned(),
            ),
            read(
                "GET /v1/version",
                Binding::Bound("conductor.store.Version"),
                "/v1/version".to_owned(),
            ),
        ]
    };
    let mut all = reads();
    all.extend([
        write(
            "PUT /v1/config",
            Binding::Bound("conductor.config.PutConfig"),
            "client",
            "PUT",
            "/v1/config".to_owned(),
            json!({"level": "global", "path": "cadence.watch", "value": "5m", "reason": "faster"}),
        ),
        write(
            "PUT /v1/config",
            Binding::Bound("conductor.config.PutConfig"),
            "spec",
            "PUT",
            "/v1/config".to_owned(),
            json!({
                "actor": operator, "level": "Instance", "instance": INSTANCE,
                "path": "thresholds.silent_session", "value": "45m",
                "reason": "silence alarms fire on long gates", "changed_at": "2026-11-08T09:00:00Z",
            }),
        ),
        write(
            "PUT /v1/config",
            Binding::Bound("conductor.config.PutConfig"),
            "spec level-scope-mismatch",
            "PUT",
            "/v1/config".to_owned(),
            json!({
                "actor": operator, "level": "Global", "instance": INSTANCE,
                "path": "cadence.watch", "value": "5m",
                "reason": "a global key names no instance", "changed_at": "2026-11-08T09:05:00Z",
            }),
        ),
        write(
            "POST /v1/instances/{name}/upgrade",
            Binding::Bound("conductor.instance.UpgradeInstance"),
            "client",
            "POST",
            instance("upgrade"),
            json!({"tag": "v1.1.0"}),
        ),
        write(
            "POST /v1/instances/{name}/upgrade",
            Binding::Bound("conductor.instance.UpgradeInstance"),
            "spec",
            "POST",
            instance("upgrade"),
            json!({"actor": operator, "binary_version": "1.1.0", "at": "2026-11-08T12:00:00Z"}),
        ),
        // The instance API of a consumer control plane: declared, and not served yet.
        read(
            "GET /v1/instances/{name}",
            Binding::Bound("conductor.instance.InstanceByName"),
            format!("/v1/instances/{INSTANCE}"),
        ),
        write(
            "POST /v1/instances",
            Binding::Bound("conductor.config.AddInstance"),
            "spec",
            "POST",
            "/v1/instances".to_owned(),
            json!({
                "actor": operator, "name": "example-org-two",
                "mapping": "{sources: [{github: example-org}], checkouts: {root: /srv/example-org-two, trees: /srv/trees}}",
                "reason": "a new tenant", "changed_at": "2026-11-12T09:00:00Z",
            }),
        ),
        write(
            "PUT /v1/instances/{name}",
            Binding::Bound("conductor.config.PutInstance"),
            "spec",
            "PUT",
            format!("/v1/instances/{INSTANCE}"),
            json!({
                "actor": operator,
                "mapping": "{sources: [{github: example-org}], checkouts: {root: /srv/example-org, trees: /srv/trees}}",
                "reason": "read the forge", "changed_at": "2026-11-12T11:00:00Z",
            }),
        ),
        write(
            "DELETE /v1/instances/{name}",
            Binding::Bound("conductor.config.RemoveInstance"),
            "spec",
            "DELETE",
            "/v1/instances/example-org-two".to_owned(),
            json!({"actor": operator, "reason": "the tenant left", "changed_at": "2026-11-12T13:00:00Z"}),
        ),
    ]);
    for verb in ["start", "stop", "restart"] {
        let (route, operation) = match verb {
            "start" => (
                "POST /v1/instances/{name}/start",
                "conductor.instance.StartInstance",
            ),
            "stop" => (
                "POST /v1/instances/{name}/stop",
                "conductor.instance.StopInstance",
            ),
            _ => (
                "POST /v1/instances/{name}/restart",
                "conductor.instance.RestartInstance",
            ),
        };
        let spec = if verb == "stop" {
            json!({"actor": operator, "at": "2026-11-08T10:00:00Z"})
        } else {
            json!({
                "actor": operator, "placement": "local", "roles": ["Director"],
                "sources": [{"kind": "GitHub", "value": {"owner": INSTANCE}}],
                "box_id": "6f1c1d2e-0000-4000-8000-000000000001", "at": "2026-11-08T09:00:00Z",
            })
        };
        all.push(write(
            route,
            Binding::Bound(operation),
            "client",
            "POST",
            instance(verb),
            json!({}),
        ));
        all.push(write(
            route,
            Binding::Bound(operation),
            "spec",
            "POST",
            instance(verb),
            spec,
        ));
    }
    all.extend([
        agent(
            "POST /v1/agents/{name}/deliver",
            Binding::Bound("conductor.agent.DeliverToAgent"),
            "client",
            "deliver",
            json!({"text": "[DISPATCH example-repo story-1] plan the story"}),
        ),
        agent(
            "POST /v1/agents/{name}/deliver",
            Binding::Bound("conductor.agent.DeliverToAgent"),
            "spec",
            "deliver",
            json!({"actor": operator, "first_line": "[DISPATCH example-repo story-1] plan the story"}),
        ),
        agent(
            "POST /v1/agents/{name}/restart",
            Binding::Bound("conductor.agent.RestartAgent"),
            "client",
            "restart",
            json!({}),
        ),
        agent(
            "POST /v1/agents/{name}/restart",
            Binding::Bound("conductor.agent.RestartAgent"),
            "spec",
            "restart",
            json!({"actor": operator, "started_at": "2026-11-08T12:00:05Z"}),
        ),
    ]);
    // The hand-written routes no operation is bound to: the same request reaches the generated
    // conductord, which serves no such path.
    let federated = "a union of every instance's answer; no specification view";
    all.extend([
        read(
            "GET /",
            Binding::Unbound("the dashboard app's files"),
            "/".to_owned(),
        ),
        read(
            "GET /v1/sessions",
            Binding::Unbound(federated),
            "/v1/sessions".to_owned(),
        ),
        read(
            "GET /v1/decisions",
            Binding::Unbound(federated),
            "/v1/decisions".to_owned(),
        ),
        read(
            "GET /v1/unanswered",
            Binding::Unbound(federated),
            "/v1/unanswered".to_owned(),
        ),
        read(
            "GET /v1/kpis",
            Binding::Unbound(federated),
            "/v1/kpis".to_owned(),
        ),
        write(
            "POST /v1/ask",
            Binding::Unbound("a question delivered to the supervisor; no command"),
            "client",
            "POST",
            "/v1/ask".to_owned(),
            json!({"text": "is the loop healthy?"}),
        ),
        write(
            "POST /v1/roles/{role}/start",
            Binding::Unbound("a host role asked for now; no command"),
            "client",
            "POST",
            "/v1/roles/supervisor/start".to_owned(),
            json!({}),
        ),
        write(
            "POST /v1/instances/{name}/join",
            Binding::Unbound(
                "JoinInstance's instance is a member of `key`, which no path segment carries",
            ),
            "client",
            "POST",
            instance("join"),
            json!({}),
        ),
        write(
            "POST /v1/instances/{name}/leave",
            Binding::Unbound(
                "LeaveInstance's instance is a member of `key`, which no path segment carries",
            ),
            "client",
            "POST",
            instance("leave"),
            json!({}),
        ),
        read(
            "GET /v1/instances/{name}/<route>",
            Binding::Unbound("proxied to the instance runtime; no operation"),
            instance("decisions"),
        ),
        write(
            "POST /v1/instances/{name}/answers",
            Binding::Unbound("proxied to the instance runtime; a batch of decisions"),
            "client",
            "POST",
            instance("answers"),
            json!({}),
        ),
        read(
            "GET /healthz",
            Binding::Unbound("the liveness probe reads no entity; a view has a source"),
            "/healthz".to_owned(),
        ),
        read(
            "GET /metrics",
            Binding::Unbound("Prometheus text; a view answers JSON rows"),
            "/metrics".to_owned(),
        ),
    ]);
    all.extend(reads().into_iter().map(|probe| Probe {
        case: "read after the writes",
        ..probe
    }));
    all
}

/// One row of the parity table.
struct Row {
    route: String,
    binding: String,
    case: String,
    hand: String,
    spec: String,
    verdict: String,
    detail: String,
}

fn run(probe: &Probe, hand_port: u16, spec_ports: &BTreeMap<&str, u16>, case: &Case) -> Row {
    let (surface, spec_path) = &probe.spec;
    let spec_port = spec_ports[match surface {
        Surface::Conductord => "conductord",
        Surface::Instance => "instance",
    }];
    let before = (state(&case.hand_store()), state(&case.spec_store()));
    let body = probe.body.as_deref();
    let (hand_status, hand_body) = exchange(hand_port, probe.method, &probe.hand, body);
    let (spec_status, spec_body) = exchange(spec_port, probe.method, spec_path, body);
    let after = (state(&case.hand_store()), state(&case.spec_store()));
    let hand_change = change(&before.0, &after.0);
    let spec_change = change(&before.1, &after.1);
    let (binding, verdict) = match probe.binding {
        Binding::Bound(operation) => {
            let mut differs = Vec::new();
            if hand_status != spec_status {
                differs.push("status");
            }
            if shape(&hand_body) != shape(&spec_body) {
                differs.push("body");
            }
            if hand_change != spec_change {
                differs.push("store");
            }
            let verdict = if differs.is_empty() {
                "match".to_owned()
            } else {
                format!("differs ({})", differs.join(", "))
            };
            (operation.to_owned(), verdict)
        }
        Binding::Unbound(why) => (format!("unbound: {why}"), "unbound".to_owned()),
    };
    Row {
        route: probe.route.to_owned(),
        binding,
        case: probe.case.to_owned(),
        hand: format!("{hand_status} {}", outline(&hand_body)),
        spec: format!("{spec_status} {}", outline(&spec_body)),
        verdict,
        detail: format!(
            "{} {} [{}]\n  hand-written {hand_status} store {hand_change}: {}\n  generated    {spec_status} store {spec_change}: {}",
            probe.method,
            probe.hand,
            probe.case,
            hand_body.trim(),
            spec_body.trim()
        ),
    }
}

/// The routes the generated surfaces serve on a path no binding names: every operation of a
/// network component with no hand-written route. The binding names its paths under `/v1/` and
/// the readiness probe's `/readyz`.
fn spec_only() -> Vec<Row> {
    conductord::ROUTES
        .iter()
        .map(|route| ("conductord", route))
        .chain(
            conductor_instance::ROUTES
                .iter()
                .map(|route| ("conductor-instance", route)),
        )
        .filter(|(_, (_, path))| {
            !path.starts_with("/v1/")
                && *path != "/readyz"
                && *path != "/docs"
                && *path != "/openapi.json"
        })
        .map(|(component, (method, path))| Row {
            route: format!("{method} {path}"),
            binding: format!("derived ({component}); no hand-written route"),
            case: "-".to_owned(),
            hand: "-".to_owned(),
            spec: "-".to_owned(),
            verdict: "spec-only".to_owned(),
            detail: String::new(),
        })
        .collect()
}

fn table(rows: &[Row]) -> String {
    let count = |verdict: &str| {
        rows.iter()
            .filter(|row| row.verdict.starts_with(verdict))
            .count()
    };
    let mut text = format!(
        "# Parity of the generated HTTP servers with the hand-written API\n\n\
         Written by `tests/api_parity.rs` (`{BLESS}=1`); compared line for line.\n\n\
         Probes: {} match, {} differ, {} unbound; {} routes spec-only.\n\n\
         | route | binding | probe | hand-written | generated | verdict |\n\
         |---|---|---|---|---|---|\n",
        count("match"),
        count("differs"),
        count("unbound"),
        count("spec-only"),
    );
    for row in rows {
        text.push_str(&format!(
            "| `{}` | {} | {} | `{}` | `{}` | {} |\n",
            row.route,
            row.binding,
            row.case,
            row.hand.replace('|', "\\|"),
            row.spec.replace('|', "\\|"),
            row.verdict
        ));
    }
    text
}

#[test]
fn the_generated_servers_answer_what_the_hand_written_api_answers() {
    let case = Case::new();
    seed(&case.hand_store());
    seed(&case.spec_store());
    let hand_port = case.serve_hand();
    let spec_ports = BTreeMap::from([
        ("conductord", case.serve_spec(Surface::Conductord)),
        ("instance", case.serve_spec(Surface::Instance)),
    ]);
    let mut rows: Vec<Row> = probes()
        .iter()
        .map(|probe| run(probe, hand_port, &spec_ports, &case))
        .collect();
    for row in &rows {
        println!("{}", row.detail);
    }
    rows.extend(spec_only());
    let text = table(&rows);
    println!("\n{text}");
    let baseline = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/api_parity.md");
    if std::env::var_os(BLESS).is_some() {
        fs::write(&baseline, &text).expect("write the baseline");
        return;
    }
    let known = fs::read_to_string(&baseline).unwrap_or_default();
    let new: Vec<&str> = text
        .lines()
        .filter(|line| !known.lines().any(|was| was == *line))
        .collect();
    let gone: Vec<&str> = known
        .lines()
        .filter(|line| !text.lines().any(|now| now == *line))
        .collect();
    assert!(
        new.is_empty() && gone.is_empty(),
        "the parity table moved from {}:\nnew:\n{}\ngone:\n{}\nrun with {BLESS}=1 to accept it",
        baseline.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
