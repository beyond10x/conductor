//! The instance API of a consumer control plane on conductord's global API: `POST /v1/instances`
//! adds one whole instance to the config file, `GET`, `PUT` and `DELETE /v1/instances/{name}`
//! read, replace and remove one (`conductor.config.AddInstance`, `PutInstance`,
//! `RemoveInstance`, `conductor.instance.InstanceByName`), each change validated as the whole
//! file, written, and recorded as a `ConfigRevision` of its actor. The daemon's API runs in this
//! process: on its TLS listener with a fake Identity service (as `tests/daemon_remote.rs` has
//! one), whose bearers are a person (`owner`) and an agent (`agent`).

mod active;
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Output;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use conductor_cli::daemon::api::{self, Api, Identity};
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::restart;
use conductor_cli::daemon::units::Units;
use conductor_cli::systemd::Service;
use conductor_model::behaviour::{ConfigRevisionStorage, ReconciliationStorage};
use conductor_model::config::{DaemonOperator, Level, OperatorGrant, OperatorRole, SecretsSource};
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use conductor_model::operator::{ActorKind, OperatorName};
use conductor_model::secret::{SecretName, Source};
use serde_json::{Value, json};

/// The control token of every case that names one: the value of the secret `control` in the
/// case's `files` secrets source.
const CONTROL: &str = "control-token-0123456789abcdef0123456789abcdef";

/// A case's directory under this target's temporary directory, removed after a passing run.
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
            .join("instance-routes")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self { dir };
        fs::write(
            case.file(),
            format!(
                "version: conductor.config/2\ndaemon: {{store: {:?}}}\ninstances:\n{}",
                case.store(),
                case.instance_yaml("alpha")
            ),
        )
        .expect("write the config file");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn store(&self) -> String {
        format!("sqlite:{}", self.dir.join("daemon.sqlite").display())
    }

    /// One instance of the case as the config file writes it, its directories in the case.
    fn instance_yaml(&self, name: &str) -> String {
        let root = self.dir.join(name);
        format!(
            "  - name: {name}\n    placement: local\n    session_prefix: {name}\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {:?}, trees: {:?}}}\n    records: {:?}\n    state: {:?}\n    \
             cache: {:?}\n",
            root.join("checkouts"),
            root.join("trees"),
            root.join("records"),
            root.join("state"),
            root.join("cache"),
        )
    }

    /// One instance mapping as a consumer sends it: the config file's mapping as a JSON object.
    fn mapping(&self, name: &str, prefix: &str) -> Value {
        let root = self.dir.join(name);
        let at = |sub: &str| root.join(sub).display().to_string();
        json!({
            "name": name,
            "placement": "local",
            "session_prefix": prefix,
            "sources": [{"github": "example-org"}],
            "checkouts": {"root": at("checkouts"), "trees": at("trees")},
            "records": at("records"),
            "state": at("state"),
            "cache": at("cache"),
        })
    }

    fn text(&self) -> String {
        fs::read_to_string(self.file()).expect("read the config file")
    }

    /// `conductor config validate` on the file as it lies.
    fn validate(&self) -> Output {
        common::conductor(self.home())
            .env(conductor_cli::config::CONFIG_VARIABLE, self.file())
            .args(["config", "validate"])
            .output()
            .expect("run config validate")
    }

    fn assert_valid(&self) {
        let output = self.validate();
        assert_eq!(
            output.status.code(),
            Some(0),
            "config validate: {}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            self.text()
        );
    }

    /// The case's `files` secrets source, its secret `control` holding `value` (none: absent).
    fn secrets(&self, value: Option<&str>) -> SecretsSource {
        let dir = self.dir.join("secrets");
        fs::create_dir_all(&dir).expect("create the secrets directory");
        match value {
            Some(value) => fs::write(dir.join("control"), value).expect("write the secret"),
            None => {
                let _ = fs::remove_file(dir.join("control"));
            }
        }
        SecretsSource {
            source: Source::Files,
            namespace: None,
            url: None,
            dir: Some(dir.display().to_string()),
        }
    }

    /// The config revisions the daemon's store holds: actor, level, instance, path, value.
    fn revisions(&self) -> Vec<conductor_model::config::ConfigRevisionData> {
        let store = conductor_cli::store::open_url(&self.store(), conductor_cli::daemon::TENANT)
            .expect("open the daemon's store");
        let mut rows: Vec<_> = ConfigRevisionStorage::list(&store)
            .into_iter()
            .map(|row| row.data)
            .collect();
        rows.sort_by(|left, right| left.changed_at.0.cmp(&right.changed_at.0));
        rows
    }
}

/// The fake Identity service: `access-<who>` resolves to `<who>@example-org.test`, a person for
/// `owner` and `reader`, an agent for `agent`; any other bearer is refused.
fn fake_identity() -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the fake Identity");
    let origin = format!(
        "http://127.0.0.1:{}",
        listener.local_addr().expect("address").port()
    );
    let at = origin.clone();
    thread::spawn(move || {
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
            let head = String::from_utf8_lossy(&head).into_owned();
            let bearer = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                        .map(|(_, value)| value.trim().to_owned())
                })
                .and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned))
                .unwrap_or_default();
            let who = bearer
                .strip_prefix("access-")
                .filter(|who| ["owner", "reader", "agent"].contains(who));
            let reply = match who.filter(|_| head.starts_with("GET /v1/access-authority ")) {
                Some(who) => {
                    let kind = if who == "agent" { "agent" } else { "human" };
                    let body = json!({
                        "iss": at, "sub": format!("sub-{who}"), "aud": "conductor",
                        "iat": 0, "nbf": 0, "exp": i64::MAX, "jti": "j",
                        "act": {"sub": format!("sub-{who}")}, "scope": "",
                        "principal_kind": kind, "tenant_id": "t",
                        "email": format!("{who}@example-org.test"), "groups": [],
                    })
                    .to_string();
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                         Connection: close\r\n\r\n{body}",
                        body.len()
                    )
                }
                None => {
                    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_owned()
                }
            };
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    origin
}

fn operator(name: &str, grants: &[OperatorGrant]) -> DaemonOperator {
    DaemonOperator {
        name: OperatorName(name.to_owned()),
        role: OperatorRole("member".to_owned()),
        grants: grants.to_vec(),
    }
}

/// The daemon's API of `case`, its operators `owner` (ChangeSettings) and `reader` (none), the
/// instance `alpha` published Running.
fn api(case: &Case, identity: Option<&str>) -> Api {
    let identity = identity.map(|origin| Identity::new(origin, api::AUDIENCE).expect("an origin"));
    let api = Api::new(
        "4444444444444444444444444444444444444444444444444444444444444444".to_owned(),
        "operator".to_owned(),
        identity,
    )
    .with_config(case.file(), case.home(), case.store());
    api.publish(
        vec![json!({"name": "alpha", "state": "Running", "placement": "local", "roles": []})],
        BTreeMap::new(),
        vec![
            operator("owner@example-org.test", &[OperatorGrant::ChangeSettings]),
            operator("reader@example-org.test", &[]),
        ],
    );
    api.publish_host(Vec::new(), None);
    api
}

/// The TLS listener of `api`: its port and certificate.
fn serve_tls(case: &Case, api: Api) -> (u16, Vec<u8>) {
    let generated =
        rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("a certificate");
    fs::write(case.dir.join("cert.pem"), generated.cert.pem()).expect("write the cert");
    fs::write(case.dir.join("key.pem"), generated.key_pair.serialize_pem()).expect("write the key");
    let tls = api::tls_config(&case.dir.join("cert.pem"), &case.dir.join("key.pem"))
        .expect("the TLS listener's config");
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the TLS listener");
    let port = listener.local_addr().expect("address").port();
    let api = Arc::new(api);
    thread::spawn(move || api::serve_tls(&listener, &api, &tls));
    (port, generated.cert.der().to_vec())
}

/// One answer: its status and its body, JSON where it is.
#[derive(Debug)]
struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON body ({error}): {self:?}"))
    }
}

fn request_text(
    method: &str,
    target: &str,
    host: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> String {
    let mut text = format!("{method} {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
    for (name, value) in headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body) = body {
        let body = body.to_string();
        text.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ));
    } else {
        text.push_str("\r\n");
    }
    text
}

fn parse_answer(bytes: &[u8]) -> Answer {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    Answer {
        status,
        body: body.to_owned(),
    }
}

/// One request to the TLS listener at `port` that presents `certificate`.
fn tls(
    (port, certificate): &(u16, Vec<u8>),
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> Answer {
    let request = request_text(method, target, &format!("localhost:{port}"), headers, body);
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(certificate.clone()))
        .expect("trust the certificate");
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from("localhost").expect("a server name");
    let connection = rustls::ClientConnection::new(Arc::new(config), name).expect("a connection");
    let tcp = TcpStream::connect(("127.0.0.1", *port)).expect("connect");
    tcp.set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    let mut stream = rustls::StreamOwned::new(connection, tcp);
    stream.write_all(request.as_bytes()).expect("send");
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
        Err(error) => panic!("read over TLS: {error}"),
    }
    parse_answer(&bytes)
}

/// Serves `api` on a plain listener bound to `address`: `127.0.0.1` is the loopback listener,
/// `0.0.0.0` one that takes bearers, as `daemon.address: 0.0.0.0` in a pod has it. Its port.
fn serve_plain(api: Api, address: &str) -> u16 {
    let listener = TcpListener::bind((address, 0)).expect("bind the listener");
    let port = listener.local_addr().expect("address").port();
    let api = Arc::new(api);
    thread::spawn(move || api::serve(&listener, &api));
    port
}

/// One request to the plain listener on `port`, through `127.0.0.1`: the answer's bytes.
fn plain_bytes(
    port: u16,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> Vec<u8> {
    let request = request_text(method, target, &format!("127.0.0.1:{port}"), headers, body);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream.write_all(request.as_bytes()).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    bytes
}

fn plain(
    port: u16,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> Answer {
    parse_answer(&plain_bytes(port, method, target, headers, body))
}

/// The bearer header of `token`.
fn bearer(token: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {token}"))
}

const OWNER: (&str, &str) = ("Authorization", "Bearer access-owner");
const AGENT: (&str, &str) = ("Authorization", "Bearer access-agent");

/// A person's bearer creates, reads, replaces and removes one whole instance; every change is the
/// whole file validated, written and recorded as theirs; an agent's bearer is refused by name, a
/// second create of one name is a conflict, an unknown name is 404, and an invalid mapping or one
/// naming another instance than the path is 422 naming its YAML path, the file unchanged.
#[test]
fn a_person_creates_reads_replaces_and_removes_one_instance() {
    let case = Case::new("person");
    let identity = fake_identity();
    let listener = serve_tls(&case, api(&case, Some(&identity)));
    let send = |method, target: &str, headers: &[(&str, &str)], body: Option<&Value>| {
        tls(&listener, method, target, headers, body)
    };

    // Create.
    let body = json!({"mapping": case.mapping("beta", "beta"), "reason": "a new tenant"});
    let created = send("POST", "/v1/instances", &[OWNER], Some(&body));
    assert_eq!(created.status, 201, "{created:?}");
    let answer = created.json();
    assert_eq!(answer["name"], "beta", "{answer}");
    assert!(
        answer["revision"].as_str().is_some_and(|id| !id.is_empty()),
        "{answer}"
    );
    assert!(case.text().contains("name: beta"), "{}", case.text());
    case.assert_valid();

    // Read one, and a name the file does not hold.
    let read = send("GET", "/v1/instances/beta", &[OWNER], None);
    assert_eq!(read.status, 200, "{read:?}");
    assert_eq!(read.json()["name"], "beta", "{read:?}");
    let read = send("GET", "/v1/instances/alpha", &[OWNER], None);
    assert_eq!(read.status, 200, "{read:?}");
    assert_eq!(read.json()["state"], "Running", "{read:?}");
    let unknown = send("GET", "/v1/instances/nobody", &[OWNER], None);
    assert_eq!(unknown.status, 404, "{unknown:?}");

    // A second create of the same name.
    let again = send("POST", "/v1/instances", &[OWNER], Some(&body));
    assert_eq!(again.status, 409, "{again:?}");
    assert_eq!(
        again.json(),
        json!({"error": "conductor.config.InstanceExists", "name": "beta"})
    );

    // An agent's bearer: refused by name, nothing written.
    let before = case.text();
    let gamma = json!({"mapping": case.mapping("gamma", "gamma"), "reason": "an agent's try"});
    let refused = send("POST", "/v1/instances", &[AGENT], Some(&gamma));
    assert_eq!(refused.status, 403, "{refused:?}");
    assert_eq!(
        refused.json()["error"],
        "conductor.operator.ActorNotHuman",
        "{refused:?}"
    );
    assert_eq!(case.text(), before);

    // Replace.
    let replaced = send(
        "PUT",
        "/v1/instances/beta",
        &[OWNER],
        Some(&json!({"mapping": case.mapping("beta", "beta-two"), "reason": "a new prefix"})),
    );
    assert_eq!(replaced.status, 200, "{replaced:?}");
    assert_eq!(replaced.json()["name"], "beta", "{replaced:?}");
    assert!(
        case.text().contains("session_prefix: beta-two"),
        "{}",
        case.text()
    );
    case.assert_valid();

    // A mapping naming another instance than the path, and one that does not validate.
    let before = case.text();
    let other = send(
        "PUT",
        "/v1/instances/beta",
        &[OWNER],
        Some(&json!({"mapping": case.mapping("gamma", "gamma"), "reason": "a rename"})),
    );
    assert_eq!(other.status, 422, "{other:?}");
    assert_eq!(other.json()["path"], "name", "{other:?}");
    let mut invalid = case.mapping("beta", "beta");
    invalid["colour"] = json!("blue");
    let invalid = send(
        "PUT",
        "/v1/instances/beta",
        &[OWNER],
        Some(&json!({"mapping": invalid, "reason": "an unknown key"})),
    );
    assert_eq!(invalid.status, 422, "{invalid:?}");
    let answer = invalid.json();
    assert_eq!(
        answer["error"], "conductor.config.ConfigInvalid",
        "{answer}"
    );
    let path = answer["path"].as_str().unwrap_or_default();
    assert!(
        path.starts_with("instances[") && path.ends_with("colour"),
        "{answer}"
    );
    let detail = answer["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("colour") && !detail.contains('\n'),
        "{answer}"
    );
    assert_eq!(case.text(), before, "an invalid mapping changed the file");

    // Unknown names.
    let unknown = send(
        "PUT",
        "/v1/instances/nobody",
        &[OWNER],
        Some(&json!({"mapping": case.mapping("nobody", "nobody"), "reason": "none"})),
    );
    assert_eq!(unknown.status, 404, "{unknown:?}");
    assert_eq!(
        unknown.json(),
        json!({"error": "conductor.instance.UnknownInstance", "name": "nobody"})
    );
    let unknown = send("DELETE", "/v1/instances/nobody", &[OWNER], None);
    assert_eq!(unknown.status, 404, "{unknown:?}");

    // Remove, with no body and with a reason.
    let removed = send(
        "DELETE",
        "/v1/instances/beta",
        &[OWNER],
        Some(&json!({"reason": "the tenant left"})),
    );
    assert_eq!(removed.status, 200, "{removed:?}");
    assert!(!case.text().contains("name: beta"), "{}", case.text());
    case.assert_valid();
    let gone = send("GET", "/v1/instances/beta", &[OWNER], None);
    assert_eq!(gone.status, 404, "{gone:?}");

    // Each change is one revision of the instance level at the path `.`, by the person.
    let revisions = case.revisions();
    assert_eq!(revisions.len(), 3, "{revisions:#?}");
    for revision in &revisions {
        assert_eq!(revision.actor.kind, ActorKind::Human, "{revision:?}");
        assert_eq!(
            conductor_cli::actor::written(&revision.actor),
            "human:owner@example-org.test"
        );
        assert_eq!(revision.level, Level::Instance, "{revision:?}");
        assert_eq!(revision.path, ".", "{revision:?}");
        assert_eq!(
            revision.instance.as_ref().map(|name| name.0.as_str()),
            Some("beta")
        );
    }
    assert!(
        revisions[0]
            .value
            .as_deref()
            .is_some_and(|value| value.contains("session_prefix"))
    );
    assert_eq!(revisions[1].reason, "a new prefix");
    assert_eq!(revisions[2].value, None);
    assert_eq!(revisions[2].reason, "the tenant left");
}

/// A bearer without the grant changes no instance: refused with the grant's name.
#[test]
fn a_person_without_the_grant_is_refused_by_its_name() {
    let case = Case::new("reader");
    let identity = fake_identity();
    let listener = serve_tls(&case, api(&case, Some(&identity)));
    let before = case.text();
    let body = json!({"mapping": case.mapping("beta", "beta"), "reason": "a new tenant"});
    let refused = tls(
        &listener,
        "POST",
        "/v1/instances",
        &[("Authorization", "Bearer access-reader")],
        Some(&body),
    );
    assert_eq!(refused.status, 403, "{refused:?}");
    assert_eq!(refused.json()["grant"], "ChangeSettings", "{refused:?}");
    assert_eq!(case.text(), before);
    assert!(case.revisions().is_empty());
}

/// The daemon's API of `case` with the control token `control` of its secrets source holding
/// `value`, and no Identity service unless `identity` names one.
fn controlled(case: &Case, identity: Option<&str>, value: Option<&str>) -> Api {
    api(case, identity)
        .with_control_token(case.secrets(value), Some(SecretName("control".to_owned())))
}

/// Decision D2 without `daemon.tls`: the control token is the machine principal
/// `control-plane` on the loopback listener and on one bound to every address, and creates,
/// reads, replaces and removes an instance there, each revision recorded as its own; the file as
/// it lies after each change validates.
#[test]
fn the_control_token_changes_instances_on_either_listener_without_tls() {
    let case = Case::new("control");
    let api = controlled(&case, None, Some(CONTROL));
    let api = Arc::new(api);
    let loopback = {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
        let port = listener.local_addr().expect("address").port();
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve(&listener, &served));
        port
    };
    let everywhere = {
        let listener = TcpListener::bind(("0.0.0.0", 0)).expect("bind every address");
        let port = listener.local_addr().expect("address").port();
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve(&listener, &served));
        port
    };
    let token = bearer(CONTROL);
    let token = [(token.0, token.1.as_str())];

    for (port, name) in [(loopback, "beta"), (everywhere, "gamma")] {
        let created = plain(
            port,
            "POST",
            "/v1/instances",
            &token,
            Some(&json!({"mapping": case.mapping(name, name), "reason": "a new tenant"})),
        );
        assert_eq!(created.status, 201, "{port}: {created:?}");
        assert_eq!(created.json()["actor"], "control-plane", "{created:?}");
        case.assert_valid();
        let read = plain(port, "GET", &format!("/v1/instances/{name}"), &token, None);
        assert_eq!(read.status, 200, "{read:?}");
        assert_eq!(read.json()["name"], name, "{read:?}");
        let listed = plain(port, "GET", "/v1/instances", &token, None);
        assert_eq!(listed.status, 200, "{listed:?}");
        let replaced = plain(
            port,
            "PUT",
            &format!("/v1/instances/{name}"),
            &token,
            Some(&json!({"mapping": case.mapping(name, &format!("{name}-two")), "reason": "r"})),
        );
        assert_eq!(replaced.status, 200, "{replaced:?}");
        case.assert_valid();
        let removed = plain(
            port,
            "DELETE",
            &format!("/v1/instances/{name}"),
            &token,
            None,
        );
        assert_eq!(removed.status, 200, "{removed:?}");
        case.assert_valid();
        assert!(
            !case.text().contains(&format!("name: {name}")),
            "{}",
            case.text()
        );
    }
    let revisions = case.revisions();
    assert_eq!(revisions.len(), 6, "{revisions:#?}");
    for revision in &revisions {
        assert_eq!(revision.actor.kind, ActorKind::ControlPlane, "{revision:?}");
        assert_eq!(
            conductor_cli::actor::written(&revision.actor),
            "control-plane"
        );
        assert_eq!(revision.level, Level::Instance);
        assert_eq!(revision.path, ".");
    }
}

/// Decision D2: every write but the instance routes, and the secrets route, refuse the control
/// plane as they refuse an agent; nothing is written.
#[test]
fn the_control_token_is_refused_on_every_other_write_and_the_secrets() {
    let case = Case::new("control-refused");
    let api = controlled(&case, None, Some(CONTROL));
    // A route to alpha's runtime, so a proxied write meets its admission: nothing listens there.
    api.publish(
        vec![json!({"name": "alpha", "state": "Running"})],
        [(
            "alpha".to_owned(),
            api::Route {
                socket: case.dir.join("none.sock"),
                token: "1111".to_owned(),
                remote: None,
                tcp: None,
            },
        )]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    let port = serve_plain(api, "127.0.0.1");
    let token = bearer(CONTROL);
    let token = [(token.0, token.1.as_str())];
    let before = case.text();
    let refused = [
        (
            "PUT",
            "/v1/config",
            json!({"level": "global", "path": "cadence.watch", "value": "5m", "reason": "r"}),
        ),
        ("POST", "/v1/instances/alpha/join", json!({"token": "x"})),
        ("POST", "/v1/instances/alpha/leave", json!({})),
        (
            "POST",
            "/v1/instances/alpha/upgrade",
            json!({"tag": "v1.1.0"}),
        ),
        ("POST", "/v1/ask", json!({"text": "a question"})),
        ("POST", "/v1/roles/supervisor/start", json!({})),
        (
            "POST",
            "/v1/instances/alpha/answers",
            json!({"answers": []}),
        ),
        ("PUT", "/v1/instances/alpha/settings", json!({})),
        ("GET", "/v1/secrets", Value::Null),
    ];
    for (method, target, body) in refused {
        let body = (!body.is_null()).then_some(&body);
        let answer = plain(port, method, target, &token, body);
        assert_eq!(answer.status, 403, "{method} {target}: {answer:?}");
        let answer = answer.json();
        assert_eq!(
            answer["error"], "conductor.operator.ActorNotHuman",
            "{answer}"
        );
        assert_eq!(answer["actor"]["kind"], "ControlPlane", "{answer}");
    }
    assert_eq!(case.text(), before);
    assert!(case.revisions().is_empty());

    // The reads a person's bearer reads, it reads.
    for target in [
        "/v1/instances",
        "/v1/instances/alpha",
        "/v1/instances/alpha/tokens",
    ] {
        let answer = plain(port, "GET", target, &token, None);
        assert_eq!(answer.status, 200, "{target}: {answer:?}");
    }
}

/// A bearer that is not the control token is `401`, the same answer whatever it is, naming no
/// kind of bearer; on the bearer listener with an Identity service a person's bearer is still the
/// person.
#[test]
fn a_wrong_bearer_is_401_with_no_hint() {
    let case = Case::new("wrong-bearer");
    let everywhere = serve_plain(controlled(&case, None, Some(CONTROL)), "0.0.0.0");
    let loopback = serve_plain(controlled(&case, None, Some(CONTROL)), "127.0.0.1");
    let near = format!("{}x", &CONTROL[..CONTROL.len() - 1]);
    for port in [everywhere, loopback] {
        let answers: Vec<Answer> = ["nothing-like-it", near.as_str(), "access-owner"]
            .into_iter()
            .map(|wrong| {
                let header = bearer(wrong);
                plain(port, "GET", "/v1/instances", &[(header.0, &header.1)], None)
            })
            .collect();
        for answer in &answers {
            assert_eq!(answer.status, 401, "{answer:?}");
            assert_eq!(answer.body, answers[0].body, "the answers differ");
            let lower = answer.body.to_lowercase();
            for hint in ["control", "identity service", "token", "secret"] {
                assert!(!lower.contains(hint), "{hint}: {answer:?}");
            }
        }
    }

    let identity = fake_identity();
    let listener = serve_tls(&case, controlled(&case, Some(&identity), Some(CONTROL)));
    let wrong = tls(
        &listener,
        "GET",
        "/v1/instances",
        &[("Authorization", "Bearer access-nobody")],
        None,
    );
    assert_eq!(wrong.status, 401, "{wrong:?}");
    assert!(!wrong.body.to_lowercase().contains("control"), "{wrong:?}");
    let person = tls(&listener, "GET", "/v1/instances", &[OWNER], None);
    assert_eq!(person.status, 200, "{person:?}");
    let header = bearer(CONTROL);
    let machine = tls(
        &listener,
        "GET",
        "/v1/instances",
        &[(header.0, &header.1)],
        None,
    );
    assert_eq!(machine.status, 200, "{machine:?}");
}

/// An empty or a missing secret value admits nobody, an empty bearer included.
#[test]
fn an_empty_or_missing_control_secret_admits_nobody() {
    let case = Case::new("empty-secret");
    for value in [Some(""), Some("\n"), None] {
        let port = serve_plain(controlled(&case, None, value), "0.0.0.0");
        for given in ["", " ", CONTROL] {
            let answer = plain(
                port,
                "POST",
                "/v1/instances",
                &[("Authorization", &format!("Bearer {given}"))],
                Some(&json!({"mapping": case.mapping("beta", "beta"), "reason": "r"})),
            );
            assert_eq!(answer.status, 401, "{value:?} {given:?}: {answer:?}");
        }
    }
    assert!(!case.text().contains("name: beta"));
    assert!(case.revisions().is_empty());
}

/// The key absent: a bearer on loopback is not read, the answer the same byte for byte, and a
/// write still needs the loopback page's origin and the daemon's token.
#[test]
fn without_the_key_a_bearer_changes_nothing() {
    let case = Case::new("no-key");
    case.secrets(Some(CONTROL));
    let port = serve_plain(api(&case, None), "127.0.0.1");
    let header = bearer(CONTROL);
    let with = plain_bytes(port, "GET", "/v1/instances", &[(header.0, &header.1)], None);
    let without = plain_bytes(port, "GET", "/v1/instances", &[], None);
    assert_eq!(
        String::from_utf8_lossy(&with),
        String::from_utf8_lossy(&without)
    );
    let refused = plain(
        port,
        "POST",
        "/v1/instances",
        &[(header.0, &header.1)],
        Some(&json!({"mapping": case.mapping("beta", "beta"), "reason": "r"})),
    );
    assert_eq!(refused.status, 403, "{refused:?}");
    assert!(!case.text().contains("name: beta"));
}

/// The service manager of the restart case: what runs, and every call made.
#[derive(Debug, Default)]
struct Fake {
    running: std::collections::BTreeSet<String>,
    calls: Vec<String>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.calls.push(format!("start {}", service.unit));
        self.running.insert(service.unit.clone());
        Ok(())
    }
    fn stop(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("stop {unit}"));
        self.running.remove(unit);
        Ok(())
    }
    fn active(&mut self, unit: &str) -> Result<bool> {
        Ok(self.running.contains(unit))
    }
    fn enable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
    fn disable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
}

/// `POST /v1/instances/{name}/restart` with the control token: the loop stops and starts the
/// runtime and records one `Restart` for `OperatorCommand` by `control-plane`; an unknown name is
/// `404`, an instance with no runtime started `409`.
#[test]
fn a_restart_is_recorded_as_the_control_planes() {
    let case = Case::new("restart");
    let open = |dir: &str| {
        Daemon::open(
            case.file(),
            case.home(),
            case.dir.join(dir),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Fake::default(),
        )
        .expect("open the daemon")
    };
    let mut daemon = open("daemon");
    daemon
        .pass(ReconcileReason::Cadence, std::time::Instant::now())
        .expect("the first pass");
    let requests = restart::listen();
    let port = serve_plain(controlled(&case, None, Some(CONTROL)), "0.0.0.0");
    let header = bearer(CONTROL);

    let ask = |name: &str, daemon: &mut Daemon<Fake>| {
        let (target, header) = (format!("/v1/instances/{name}/restart"), header.clone());
        let client =
            thread::spawn(move || plain(port, "POST", &target, &[(header.0, &header.1)], None));
        let handed = requests
            .recv_timeout(Duration::from_secs(10))
            .expect("the restart reaches the loop");
        daemon.answer_restart(handed);
        client.join().expect("the client")
    };
    let restarted = ask("alpha", &mut daemon);
    assert_eq!(restarted.status, 202, "{restarted:?}");
    assert_eq!(restarted.json()["actor"], "control-plane", "{restarted:?}");
    let unknown = ask("nobody", &mut daemon);
    assert_eq!(unknown.status, 404, "{unknown:?}");
    assert_eq!(
        unknown.json()["error"],
        "conductor.instance.UnknownInstance"
    );

    let unit = conductor_cli::daemon::reconcile::unit_name("alpha");
    assert_eq!(
        daemon.units.calls,
        [
            format!("start {unit}"),
            format!("stop {unit}"),
            format!("start {unit}")
        ]
    );
    let restarts: Vec<_> = ReconciliationStorage::list(daemon.store())
        .into_iter()
        .filter(|row| row.data.action == ReconcileAction::Restart)
        .collect();
    assert_eq!(restarts.len(), 1, "{restarts:?}");
    let restart = &restarts[0].data;
    assert_eq!(restart.reason, ReconcileReason::OperatorCommand);
    assert_eq!(restart.result, ReconcileResult::Succeeded);
    assert_eq!(restart.actor.kind, ActorKind::ControlPlane);

    let (status, body) =
        open("never-started").restart("alpha", &conductor_cli::actor::control_plane());
    assert_eq!(status, "409 Conflict", "{body}");
    assert_eq!(body["error"], "conductor.instance.InstanceStateConflict");
}

/// A daemon that runs its instances as pods refuses an instance's token-pool join and leave, in
/// one line that says why.
#[test]
fn join_and_leave_are_refused_for_pods() {
    let case = Case::new("pods");
    let token = "4444444444444444444444444444444444444444444444444444444444444444";
    // No operator listed: the loopback operator holds every grant, `Join` too.
    let api = api(&case, None).with_pods(true);
    api.publish(
        vec![json!({"name": "alpha", "state": "Running"})],
        BTreeMap::new(),
        Vec::new(),
    );
    let port = serve_plain(api, "127.0.0.1");
    for route in ["join", "leave"] {
        let origin = format!("http://127.0.0.1:{port}");
        let answer = plain(
            port,
            "POST",
            &format!("/v1/instances/alpha/{route}"),
            &[("Origin", &origin), ("X-Conductor-Token", token)],
            Some(&json!({})),
        );
        assert_eq!(answer.status, 409, "{route}: {answer:?}");
        let error = answer.json()["error"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        assert!(error.contains("pod") && !error.contains('\n'), "{error}");
    }
}

/// `daemon.control_token` names a secret, `secret:<name>`, and `config show` writes it; a value
/// is refused by its YAML path and not echoed.
#[test]
fn the_control_token_is_a_secret_name_in_the_daemon_section() {
    let case = Case::new("config-key");
    let text = case.text();
    fs::write(
        case.file(),
        text.replacen(
            "daemon: {",
            "daemon: {control_token: \"secret:control\", ",
            1,
        ),
    )
    .expect("write the config file");
    case.assert_valid();
    let shown = common::conductor(case.home())
        .env(conductor_cli::config::CONFIG_VARIABLE, case.file())
        .args(["config", "show", "--format", "json"])
        .output()
        .expect("run config show");
    let shown: Value = serde_json::from_slice(&shown.stdout).expect("config show's JSON");
    let written = shown
        .pointer("/daemon/control_token")
        .or_else(|| shown.pointer("/config/daemon/control_token"));
    assert_eq!(written, Some(&json!("secret:control")), "{shown}");

    fs::write(
        case.file(),
        text.replacen("daemon: {", "daemon: {control_token: plain-value-here, ", 1),
    )
    .expect("write the config file");
    let output = case.validate();
    assert_eq!(output.status.code(), Some(1));
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(said.contains("daemon.control_token"), "{said}");
    assert!(!said.contains("plain-value-here"), "{said}");
}

/// The daemon's own secrets are no instance's (`conductor_cli::secret::daemons_own`): a mapping
/// that names the control token's secret under `model_token`, or as a remote placement's
/// `bearer`, is 422 `ConfigInvalid` at that path, its reason saying "a secret of the daemon's
/// own" and not which key of the daemon names it, the file unchanged; a secret the daemon does
/// not own is still taken; and a file written by hand that names it does not validate.
#[test]
fn a_mapping_naming_a_secret_of_the_daemons_own_is_refused_by_its_path() {
    let case = Case::new("own-secret");
    let text = case.text();
    fs::write(
        case.file(),
        text.replacen(
            "daemon: {",
            "daemon: {control_token: \"secret:control\", ",
            1,
        ),
    )
    .expect("write the config file");
    case.assert_valid();
    let api = Arc::new(controlled(&case, None, Some(CONTROL)));
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || api::serve(&listener, &api));
    let token = bearer(CONTROL);
    let token = [(token.0, token.1.as_str())];
    let before = case.text();

    let mut by_model = case.mapping("beta", "beta");
    by_model["model_token"] = json!("secret:control");
    let mut by_bearer = case.mapping("beta", "beta");
    by_bearer["placement"] = json!({
        "daemon": "https://substrate.example.com",
        "adapter": "oci",
        "bearer": "secret:control",
        "trust_roots": "/etc/substrate/roots.pem",
    });
    let mut replaced = case.mapping("alpha", "alpha");
    replaced["model_token"] = json!("secret:control");
    for (method, target, mapping, path) in [
        (
            "POST",
            "/v1/instances",
            by_model,
            "instances[1].model_token",
        ),
        (
            "POST",
            "/v1/instances",
            by_bearer,
            "instances[1].placement.bearer",
        ),
        (
            "PUT",
            "/v1/instances/alpha",
            replaced,
            "instances[0].model_token",
        ),
    ] {
        let refused = plain(
            port,
            method,
            target,
            &token,
            Some(&json!({"mapping": mapping, "reason": "a tenant"})),
        );
        assert_eq!(refused.status, 422, "{path}: {refused:?}");
        let answer = refused.json();
        assert_eq!(
            answer["error"], "conductor.config.ConfigInvalid",
            "{answer}"
        );
        assert_eq!(answer["path"], path, "{answer}");
        let detail = answer["detail"].as_str().unwrap_or_default();
        assert!(
            detail.contains(&format!("{path}: names a secret of the daemon's own"))
                && !detail.contains("control_token")
                && !detail.contains('\n'),
            "{answer}"
        );
        assert_eq!(case.text(), before, "{path}: the file changed");
    }

    let mut not_owned = case.mapping("beta", "beta");
    not_owned["model_token"] = json!("secret:beta-model");
    let created = plain(
        port,
        "POST",
        "/v1/instances",
        &token,
        Some(&json!({"mapping": not_owned, "reason": "a tenant"})),
    );
    assert_eq!(created.status, 201, "{created:?}");
    case.assert_valid();

    fs::write(
        case.file(),
        format!("{before}    model_token: secret:control\n"),
    )
    .expect("write the config file");
    let output = case.validate();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(
        said.contains("instances[0].model_token: names a secret of the daemon's own"),
        "{said}"
    );
}

/// `story:source-include-list`: a mapping whose sources carry `include` is created and replaced,
/// and `GET` echoes the list on the source that names it and on no other; `include` beside
/// `exclude`, on a `local` source, empty, or a `github` entry of two steps is 422 at its YAML
/// path, the file unchanged.
#[test]
fn the_instance_routes_take_and_echo_an_include_list() {
    let case = Case::new("include");
    let api = Arc::new(controlled(&case, None, Some(CONTROL)));
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || api::serve(&listener, &api));
    let token = bearer(CONTROL);
    let token = [(token.0, token.1.as_str())];

    let mut mapping = case.mapping("beta", "beta");
    mapping["sources"] = json!([
        {"github": "example-org", "include": ["one"]},
        {"gitlab": "example-group", "host": "gitlab.example.com", "include": ["alpha", "sub/gamma"]},
    ]);
    let created = plain(
        port,
        "POST",
        "/v1/instances",
        &token,
        Some(&json!({"mapping": mapping, "reason": "a tenant of three repositories"})),
    );
    assert_eq!(created.status, 201, "{created:?}");
    case.assert_valid();
    let read = plain(port, "GET", "/v1/instances/beta", &token, None);
    assert_eq!(read.status, 200, "{read:?}");
    assert_eq!(
        read.json()["sources"],
        json!([
            {"kind": "GitHub", "value": {"owner": "example-org", "include": ["one"]}},
            {"kind": "GitLab", "value": {"group": "example-group", "host": "gitlab.example.com",
                "exclude": [], "include": ["alpha", "sub/gamma"]}},
        ]),
        "{read:?}"
    );

    let mut replaced = case.mapping("beta", "beta");
    replaced["sources"] = json!([{"github": "example-org", "include": ["two"]}]);
    let put = plain(
        port,
        "PUT",
        "/v1/instances/beta",
        &token,
        Some(&json!({"mapping": replaced, "reason": "another repository"})),
    );
    assert_eq!(put.status, 200, "{put:?}");
    case.assert_valid();
    let read = plain(port, "GET", "/v1/instances/beta", &token, None);
    assert_eq!(
        read.json()["sources"],
        json!([{"kind": "GitHub", "value": {"owner": "example-org", "include": ["two"]}}]),
        "{read:?}"
    );
    let put = plain(
        port,
        "PUT",
        "/v1/instances/beta",
        &token,
        Some(&json!({"mapping": case.mapping("beta", "beta"), "reason": "every repository"})),
    );
    assert_eq!(put.status, 200, "{put:?}");
    let read = plain(port, "GET", "/v1/instances/beta", &token, None);
    assert_eq!(
        read.json()["sources"],
        json!([{"kind": "GitHub", "value": {"owner": "example-org"}}]),
        "without include the source is echoed as before: {read:?}"
    );

    let before = case.text();
    for (sources, path) in [
        (
            json!([{"gitlab": "example-group", "include": ["a"], "exclude": ["b"]}]),
            "instances[1].sources[0].include",
        ),
        (
            json!([{"local": "/srv/example-org", "include": ["a"]}]),
            "instances[1].sources[0].include",
        ),
        (
            json!([{"gitlab": "example-group", "include": []}]),
            "instances[1].sources[0].include",
        ),
        (
            json!([{"github": "example-org", "include": ["one/two"]}]),
            "instances[1].sources[0].include[0]",
        ),
    ] {
        let mut mapping = case.mapping("beta", "beta");
        mapping["sources"] = sources;
        let refused = plain(
            port,
            "PUT",
            "/v1/instances/beta",
            &token,
            Some(&json!({"mapping": mapping, "reason": "a refused list"})),
        );
        assert_eq!(refused.status, 422, "{path}: {refused:?}");
        let answer = refused.json();
        assert_eq!(
            answer["error"], "conductor.config.ConfigInvalid",
            "{answer}"
        );
        assert_eq!(answer["path"], path, "{answer}");
        assert_eq!(case.text(), before, "{path}: the file changed");
    }
}
