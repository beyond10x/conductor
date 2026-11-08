//! conductord's global API (`story:daemon-reconcile`, `story:instance-api`, design
//! architecture-next § 5.2, § 5.3). It answers on the loopback listener `127.0.0.1:<daemon.port>`
//! ([`serve`]) and, when `daemon.tls` names one, on a TLS listener with Identity bearers
//! ([`serve_tls`]), both at once:
//! - `/` and every file of the dashboard app ([`crate::dashboard::app_file`]): on loopback
//!   `index.html` carries the daemon's token in `<meta name="conductor-token">`, the one token a
//!   browser holds; on the TLS listener it carries none, and the API takes bearers only;
//! - `GET /v1/instances`: the rows the reconciler last published ([`Api::publish`]);
//! - `/v1/instances/{name}/<route>`: the instance's `/v1/<route>`, proxied to its runtime's socket
//!   ([`Route`]) with the instance's token, which no browser holds; only its writing routes
//!   ([`WRITES`]) are admitted as writes, any other method is forwarded bare and the instance
//!   answers it 405. `start`, `stop`, `restart` and `upgrade` are the daemon's own routes and are
//!   not proxied; `POST /v1/instances/{name}/upgrade` hands an upgrade to the loop
//!   ([`super::upgrade::post`]), the others are not served yet;
//! - `PUT /v1/config`: one key of the config changed ([`crate::config::put::put_config`]).
//! - `GET /v1/sessions`, `decisions`, `unanswered`, `kpis` and `events` (SSE): the union of every
//!   instance's answer, each row carrying `instance`, an instance that does not answer within
//!   `daemon.timeout` a row `state: unreachable` ([`federate`], `story:federated-views`);
//!   `POST /v1/ask`: refused with `conductor.instance.SupervisorNotRunning` until a supervisor
//!   runs; `GET /v1/secrets`: the secrets' names, sources and instances, never a value
//!   ([`crate::secret::view`]);
//! - an instance's `agents/{name}/deliver` and `restart` are proxied as writes, `attach` bare.
//!
//! A local client that is no browser (`conductor instance upgrade`) writes with the token the
//! daemon keeps in [`TOKEN_FILE`] of its directory, mode 0600, naming the listener's origin.
//!
//! Admission. On loopback every request names this listener's host (or none), so a page served
//! from elsewhere cannot read through DNS rebinding; reads are open, and a write comes from this
//! listener's own page ([`crate::dashboard::same_origin`]) with the daemon's token. On the TLS
//! listener every request but the app's files and a CORS preflight carries `Authorization:
//! Bearer`, resolved by the Identity service `daemon.tls.identity` ([`Identity`]); any origin may
//! read with a bearer (`Access-Control-Allow-Origin: *`, no credentials), so the same app deployed
//! elsewhere reads the listener's URL.
//!
//! Every write carries an operator (`conductor.operator`, design § 5.3): on loopback the operator
//! the Unix user names, on the TLS listener the bearer's principal, named as `daemon.operators`
//! names it (its email or subject). A principal that is not a person is refused
//! `conductor.operator.ActorNotHuman`; an operator without the grant the write needs
//! ([`grant_for`]) `conductor.operator.GrantMissing`, each `403` with a JSON body naming the
//! error. A loopback operator holds every grant while `daemon.operators` is empty: one operator is
//! implied by the Unix user on a single host. The write is forwarded with the operator in
//! `X-Conductor-Operator`, and the instance records it as that operator's.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{DaemonOperator, OperatorGrant};
use conductor_model::secret::Source;
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use serde::Deserialize;
use serde_json::{Value, json};

use super::federate;
use crate::dashboard;

/// The route of the instances list.
pub const INSTANCES: &str = "/v1/instances";

/// The routes under `/v1/instances/{name}/` that are the daemon's own and never proxied.
const OWN: [&str; 4] = ["start", "stop", "restart", "upgrade"];

/// How long a client may take to send its request or take its answer.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// How long an instance may take to answer a proxied request: a cold answer waits on its first
/// computation.
const PROXY_TIMEOUT: Duration = Duration::from_secs(60);

/// The longest body of a write, 64 KiB: the instance API's own bound, and the daemon's on an
/// upgrade and a config change.
const MAX_FORWARD: usize = dashboard::MAX_BODY;

/// The audience the daemon resolves bearers for.
pub const AUDIENCE: &str = "conductor";

/// One instance runtime the daemon proxies to: its socket and the token it minted for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// `<state>/api.sock` ([`crate::instance::socket`]).
    pub socket: PathBuf,
    /// The token the instance takes a write with.
    pub token: String,
}

/// What the reconciler publishes after each tick.
#[derive(Debug, Default)]
struct Published {
    rows: Vec<Value>,
    routes: BTreeMap<String, Route>,
    operators: Vec<DaemonOperator>,
}

/// The config file `PUT /v1/config` changes and the store it records the revision in.
struct ConfigTarget {
    file: PathBuf,
    home: PathBuf,
    store: String,
}

/// What every connection of both listeners shares.
pub struct Api {
    published: Mutex<Published>,
    /// The daemon's token: its loopback page carries it, a loopback write presents it.
    token: String,
    /// The operator the Unix user names: who a loopback write is done for.
    local: String,
    identity: Option<Identity>,
    config: Option<ConfigTarget>,
    /// How long an instance has to answer one federated read.
    timeout: Duration,
    /// When each instance last answered a federated read, RFC 3339.
    answered: Mutex<BTreeMap<String, String>>,
    /// The secrets source a name only the config writes is listed with.
    secrets: Source,
}

impl Api {
    /// The API with the daemon's `token`, the loopback operator `local` and, for the TLS
    /// listener, the Identity service that resolves bearers.
    #[must_use]
    pub fn new(token: String, local: String, identity: Option<Identity>) -> Self {
        Self {
            published: Mutex::new(Published::default()),
            token,
            local,
            identity,
            config: None,
            timeout: Duration::from_secs(crate::config::FEDERATED_TIMEOUT),
            answered: Mutex::new(BTreeMap::new()),
            secrets: Source::Keychain,
        }
    }

    /// The API with `PUT /v1/config` changing the config `file` (`~` read as `home`) and
    /// recording each revision in the store at the URL `store` (`daemon.store`); without it the
    /// route answers `503`.
    #[must_use]
    pub fn with_config(mut self, file: PathBuf, home: PathBuf, store: String) -> Self {
        self.config = Some(ConfigTarget { file, home, store });
        self
    }

    /// The API with `timeout`, how long an instance has to answer one federated read
    /// (`daemon.timeout`, `story:federated-views`).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The API with `source`, the secrets source `GET /v1/secrets` names for a name only the
    /// config writes (`daemon.secrets.source`).
    #[must_use]
    pub const fn with_secrets(mut self, source: Source) -> Self {
        self.secrets = source;
        self
    }

    /// Publishes the instances' `rows`, the `routes` to their runtimes and the deployment's
    /// `operators`.
    pub fn publish(
        &self,
        rows: Vec<Value>,
        routes: BTreeMap<String, Route>,
        operators: Vec<DaemonOperator>,
    ) {
        let mut published = self
            .published
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *published = Published {
            rows,
            routes,
            operators,
        };
    }

    fn rows(&self) -> Vec<Value> {
        self.published
            .lock()
            .map(|published| published.rows.clone())
            .unwrap_or_default()
    }

    fn route(&self, name: &str) -> Option<Route> {
        self.published
            .lock()
            .ok()
            .and_then(|published| published.routes.get(name).cloned())
    }

    /// Every published instance as a federated route asks it: its route, and when it was last
    /// seen, the later of its last answer and the reconciler's last sight of it.
    fn members(&self) -> Vec<federate::Member> {
        let answered = self
            .answered
            .lock()
            .map(|answered| answered.clone())
            .unwrap_or_default();
        let Ok(published) = self.published.lock() else {
            return Vec::new();
        };
        published
            .rows
            .iter()
            .filter_map(|row| {
                let name = row["name"].as_str()?.to_owned();
                let seen = row["last_seen"].as_str().map(str::to_owned);
                let last_seen = match (seen, answered.get(&name).cloned()) {
                    (Some(seen), Some(answer)) => Some(seen.max(answer)),
                    (seen, answer) => seen.or(answer),
                };
                Some(federate::Member {
                    route: published.routes.get(&name).cloned(),
                    name,
                    last_seen,
                })
            })
            .collect()
    }

    /// Notes that the instance `name` answered a federated read now.
    fn seen(&self, name: &str) {
        if let (Ok(now), Ok(mut answered)) = (crate::records::now(), self.answered.lock()) {
            answered.insert(name.to_owned(), now.0);
        }
    }

    fn operators(&self) -> Vec<DaemonOperator> {
        self.published
            .lock()
            .map(|published| published.operators.clone())
            .unwrap_or_default()
    }
}

/// The grant a write on the route `route` needs: answers and the Plan page's moves are decisions,
/// an upgrade upgrades, and everything else, `PUT /v1/config` included, changes settings.
#[must_use]
pub fn grant_for(route: &str) -> OperatorGrant {
    match route {
        "answers" | "plan" => OperatorGrant::AnswerDecisions,
        "upgrade" => OperatorGrant::UpgradeInstances,
        _ => OperatorGrant::ChangeSettings,
    }
}

/// The route of the config: `PUT` changes one key of it ([`crate::config::put::put_config`]).
pub const CONFIG: &str = "/v1/config";

/// The file in the daemon's directory that holds its token, mode 0600, for a local client that is
/// no browser (`conductor instance upgrade`).
pub const TOKEN_FILE: &str = "api.token";

/// Keeps the daemon's `token` in [`TOKEN_FILE`] of its directory `dir`, the owner's only.
///
/// # Errors
///
/// The directory cannot be created or the file written.
pub fn keep_token(dir: &Path, token: &str) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(TOKEN_FILE);
    let next = dir.join(format!("{TOKEN_FILE}.next"));
    let _ = fs::remove_file(&next);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&next)
        .with_context(|| format!("write {}", next.display()))?;
    file.write_all(token.as_bytes())
        .with_context(|| format!("write {}", next.display()))?;
    fs::rename(&next, &path).with_context(|| format!("write {}", path.display()))
}

/// The token the daemon keeps in its directory `dir`, if it is there.
#[must_use]
pub fn kept_token(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join(TOKEN_FILE))
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

/// The grant's name as the specification spells it.
const fn grant_name(grant: OperatorGrant) -> &'static str {
    match grant {
        OperatorGrant::AnswerDecisions => "AnswerDecisions",
        OperatorGrant::ChangeSettings => "ChangeSettings",
        OperatorGrant::StartInstances => "StartInstances",
        OperatorGrant::StopInstances => "StopInstances",
        OperatorGrant::UpgradeInstances => "UpgradeInstances",
        OperatorGrant::Purge => "Purge",
        OperatorGrant::Join => "Join",
    }
}

/// Which listener a request came in on.
#[derive(Debug, Clone, Copy)]
enum Listener {
    /// `127.0.0.1:<port>`.
    Loopback(u16),
    /// The TLS listener.
    Tls,
}

/// Serves the loopback `listener` until it fails, each connection on a thread of its own.
pub fn serve(listener: &TcpListener, api: &Arc<Api>) {
    let port = listener.local_addr().map_or(0, |address| address.port());
    for stream in listener.incoming().flatten() {
        let api = Arc::clone(api);
        std::thread::spawn(move || {
            let mut stream = stream;
            if timeouts(&stream).is_err() {
                return;
            }
            // A client that goes away mid-request costs only its own answer.
            let _ = connection(&mut stream, Listener::Loopback(port), &api);
            let _ = stream.shutdown(Shutdown::Write);
            let _ = io::copy(&mut (&stream).take(MAX_FORWARD as u64), &mut io::sink());
        });
    }
}

/// Serves the TLS `listener` with `config` until it fails, each connection on a thread of its own.
pub fn serve_tls(listener: &TcpListener, api: &Arc<Api>, config: &Arc<rustls::ServerConfig>) {
    for stream in listener.incoming().flatten() {
        let (api, config) = (Arc::clone(api), Arc::clone(config));
        std::thread::spawn(move || {
            if timeouts(&stream).is_err() {
                return;
            }
            let Ok(server) = rustls::ServerConnection::new(config) else {
                return;
            };
            let mut tls = rustls::StreamOwned::new(server, stream);
            if connection(&mut tls, Listener::Tls, &api).is_ok() {
                tls.conn.send_close_notify();
                let _ = tls.flush();
            }
        });
    }
}

fn timeouts(stream: &TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))
}

/// The TLS listener's config from the PEM files `cert` (the chain) and `key`.
///
/// # Errors
///
/// A file does not read or holds no certificate or key, or rustls refuses them.
pub fn tls_config(cert: &Path, key: &Path) -> Result<Arc<rustls::ServerConfig>> {
    let chain = CertificateDer::pem_file_iter(cert)
        .with_context(|| format!("read {}", cert.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("read the certificates of {}", cert.display()))?;
    if chain.is_empty() {
        bail!("{} holds no certificate", cert.display());
    }
    let key = PrivateKeyDer::from_pem_file(key)
        .with_context(|| format!("read the private key of {}", key.display()))?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .context("the TLS protocol versions")?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .context("the TLS listener's certificate and key")?;
    Ok(Arc::new(config))
}

/// One request as read: its method, target, headers and the bytes after its head.
struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    early: Vec<u8>,
}

impl Request {
    fn header(&self, wanted: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
            .map(|(_, value)| value.as_str())
    }

    fn path(&self) -> &str {
        self.target
            .split_once('?')
            .map_or(self.target.as_str(), |(path, _)| path)
    }

    fn query(&self) -> Option<&str> {
        self.target.split_once('?').map(|(_, query)| query)
    }
}

fn parse(head: &str, early: Vec<u8>) -> Option<Request> {
    let mut lines = head.split("\r\n");
    let mut words = lines.next()?.split(' ');
    let (method, target, version) = (words.next()?, words.next()?, words.next()?);
    if !version.starts_with("HTTP/1.") {
        return None;
    }
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    Some(Request {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        early,
    })
}

fn connection<S: Read + Write>(stream: &mut S, listener: Listener, api: &Api) -> io::Result<()> {
    let (head, early) = dashboard::read_head(stream)?;
    let reply = match head.and_then(|head| parse(&head, early)) {
        Some(request) => answer(&request, stream, listener, api),
        None => reply(
            400,
            "Bad Request",
            &json!({"error": "bad request"}),
            listener,
        ),
    };
    stream.write_all(&reply)?;
    stream.flush()
}

/// Who a request is from, as its listener admitted it.
enum Who {
    /// The loopback listener's page or a local client.
    Local,
    /// A bearer the Identity service resolved: its operator name and whether it is a person.
    Bearer { name: String, human: bool },
}

fn answer<S: Read + Write>(
    request: &Request,
    stream: &mut S,
    listener: Listener,
    api: &Api,
) -> Vec<u8> {
    let path = request.path().to_owned();
    if matches!(listener, Listener::Tls) && request.method == "OPTIONS" {
        return preflight();
    }
    if let Listener::Loopback(port) = listener
        && !own_host(request.header("host"), port)
    {
        return reply(
            403,
            "Forbidden",
            &json!({"error": "this listener answers only its own host"}),
            listener,
        );
    }
    if request.method == "GET" && !path.starts_with("/v1/") {
        return app(&path, listener, api);
    }
    if !path.starts_with("/v1/") {
        if dashboard::app_file(&path).is_none() {
            return reply(
                404,
                "Not Found",
                &json!({"error": format!("no route {path}")}),
                listener,
            );
        }
        return response(
            405,
            "Method Not Allowed",
            "application/json",
            b"{\"error\":\"the app's files answer GET only\"}\n",
            "Allow: GET\r\n",
            listener,
        );
    }
    let who = match listener {
        Listener::Loopback(_) => Who::Local,
        Listener::Tls => match bearer(request, api) {
            Ok(who) => who,
            Err(why) => {
                return reply(401, "Unauthorized", &json!({"error": why}), listener);
            }
        },
    };
    if path == CONFIG {
        if request.method != "PUT" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{CONFIG} answers PUT only")}),
                listener,
            );
        }
        let operator = match admit_write(request, listener, &who, "config", api) {
            Ok(operator) => operator,
            Err(refusal) => return refusal,
        };
        let body = match read_body(request, stream) {
            Ok(body) => body,
            Err(refusal) => return refusal(listener),
        };
        let (code, reason, answer) = put_config(api, &body, &operator);
        return reply(code, reason, &answer, listener);
    }
    if let Some(answer) = federated(request, stream, listener, &who, api) {
        return answer;
    }
    if path == INSTANCES {
        if request.method != "GET" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{INSTANCES} answers GET only")}),
                listener,
            );
        }
        return reply(200, "OK", &Value::Array(api.rows()), listener);
    }
    let Some((name, route)) = path
        .strip_prefix("/v1/instances/")
        .and_then(|rest| rest.split_once('/'))
    else {
        return reply(
            404,
            "Not Found",
            &json!({"error": format!("no route {path}")}),
            listener,
        );
    };
    if route == "upgrade" {
        if request.method != "POST" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{path} answers POST only")}),
                listener,
            );
        }
        let operator = match admit_write(request, listener, &who, route, api) {
            Ok(operator) => operator,
            Err(refusal) => return refusal,
        };
        let body = match read_body(request, stream) {
            Ok(body) => body,
            Err(refusal) => return refusal(listener),
        };
        let (status, answer) =
            super::upgrade::post(&path, &body, crate::actor::operator(&operator));
        let (code, reason) = status_parts(status);
        return reply(code, reason, &answer, listener);
    }
    if OWN.contains(&route) {
        return reply(
            404,
            "Not Found",
            &json!({"error": format!("{path}: the daemon serves no {route} yet")}),
            listener,
        );
    }
    let Some(target) = api.route(name) else {
        return reply(
            404,
            "Not Found",
            &json!({"error": format!("no running instance {name}")}),
            listener,
        );
    };
    // Only an instance's writing routes are admitted and carry its token; any other method is
    // forwarded bare, and the instance answers it 405.
    let write = writes(route, &request.method);
    let operator = if write {
        match admit_write(request, listener, &who, route, api) {
            Ok(operator) => Some(operator),
            Err(refusal) => return refusal,
        }
    } else {
        None
    };
    let body = if write {
        match read_body(request, stream) {
            Ok(body) => body,
            Err(refusal) => return refusal(listener),
        }
    } else {
        Vec::new()
    };
    match proxy(&target, request, route, &body, operator.as_deref()) {
        Ok(bytes) => with_cors(bytes, listener),
        Err(error) => reply(
            502,
            "Bad Gateway",
            &json!({
                "error": format!("instance {name} does not answer: {error:#}"),
                "state": "Unreachable",
            }),
            listener,
        ),
    }
}

/// Whether `host` names the loopback listener on `port`, or is absent (a client that is no
/// browser).
fn own_host(host: Option<&str>, port: u16) -> bool {
    host.is_none_or(|host| {
        [
            format!("127.0.0.1:{port}"),
            format!("localhost:{port}"),
            "127.0.0.1".to_owned(),
            "localhost".to_owned(),
        ]
        .iter()
        .any(|own| own.eq_ignore_ascii_case(host))
    })
}

/// The app's file at `path`, `index.html` with the daemon's token on loopback.
fn app(path: &str, listener: Listener, api: &Api) -> Vec<u8> {
    let Some(file) = dashboard::app_file(path) else {
        return reply(
            404,
            "Not Found",
            &json!({"error": format!("no route {path}")}),
            listener,
        );
    };
    let body = match listener {
        Listener::Loopback(_) if file.path() == Path::new("index.html") => {
            dashboard::page_with(file.contents(), &api.token)
        }
        _ => file.contents().to_vec(),
    };
    response(
        200,
        "OK",
        dashboard::content_type(file),
        &body,
        "",
        listener,
    )
}

/// The bearer of `request`, resolved by the Identity service.
fn bearer(request: &Request, api: &Api) -> Result<Who, String> {
    let Some(given) = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
    else {
        return Err("this listener takes an Identity bearer: Authorization: Bearer".to_owned());
    };
    let Some(identity) = &api.identity else {
        return Err("this daemon names no Identity service (daemon.tls.identity)".to_owned());
    };
    let authority = identity
        .resolve(given)
        .map_err(|error| format!("the Identity service refused the bearer: {error:#}"))?;
    let operators = api.operators();
    let name = operators
        .iter()
        .map(|operator| operator.name.0.clone())
        .find(|name| {
            authority.email.as_deref() == Some(name.as_str()) || authority.subject == *name
        })
        .or(authority.email)
        .unwrap_or(authority.subject);
    Ok(Who::Bearer {
        name,
        human: authority.principal_kind == "human",
    })
}

/// Admits a write of `who` on the instance route `route`: the loopback listener's own page with
/// the daemon's token; a person; an operator with the grant. Answers the operator's name.
fn admit_write(
    request: &Request,
    listener: Listener,
    who: &Who,
    route: &str,
    api: &Api,
) -> Result<String, Vec<u8>> {
    if let Listener::Loopback(port) = listener {
        let header = |name: &str| request.header(name);
        let token = request.header(dashboard::TOKEN_HEADER);
        if !dashboard::same_origin(header, port)
            || !token.is_some_and(|given| dashboard::same_token(&api.token, given))
        {
            return Err(reply(
                403,
                "Forbidden",
                &json!({
                    "error": "a write is taken only from this listener's own origin, with the daemon's token \
                              (a page of it, or a local client that keeps it); reload the page",
                }),
                listener,
            ));
        }
    }
    let name = match who {
        Who::Local => api.local.clone(),
        Who::Bearer { name, human: false } => {
            return Err(reply(
                403,
                "Forbidden",
                &json!({
                    "error": "conductor.operator.ActorNotHuman",
                    "actor": {"kind": "Agent", "operator": null, "agent": null},
                    "principal": name,
                }),
                listener,
            ));
        }
        Who::Bearer { name, human: true } => name.clone(),
    };
    let grant = grant_for(route);
    let operators = api.operators();
    let implied = matches!(who, Who::Local) && operators.is_empty();
    let holds = operators
        .iter()
        .any(|operator| operator.name.0 == name && operator.grants.contains(&grant));
    if implied || holds {
        Ok(name)
    } else {
        Err(reply(
            403,
            "Forbidden",
            &json!({
                "error": "conductor.operator.GrantMissing",
                "operator": name,
                "grant": grant_name(grant),
            }),
            listener,
        ))
    }
}

/// The body of `PUT /v1/config`: the change as `conductor config put` takes it; the actor is the
/// operator admitted and the time is now.
#[derive(Deserialize)]
struct ConfigChange {
    level: String,
    instance: Option<String>,
    role: Option<String>,
    path: String,
    value: Option<String>,
    reason: String,
}

/// `PUT /v1/config` with `body` as `operator` (`story:config-globals`): the change through
/// [`crate::config::put::put_config`], recorded in the daemon's store; answers the revision with
/// the `ConfigChanged` it publishes, a refusal `422` (`403` for `ActorNotHuman`) with its one-line
/// display, or `503` when the daemon was given no config file.
fn put_config(api: &Api, body: &[u8], operator: &str) -> (u16, &'static str, Value) {
    use conductor_model::config::{InstanceName, PutConfig, PutConfigOutcome, RoleName};
    let Some(target) = &api.config else {
        return (
            503,
            "Service Unavailable",
            json!({"error": "this daemon was given no config file to change"}),
        );
    };
    let change = match serde_json::from_slice::<ConfigChange>(body) {
        Ok(change) => change,
        Err(error) => {
            return (
                400,
                "Bad Request",
                json!({"error": format!(
                    "the body is not a change: {error}; write {{\"level\", \"path\", \"value\", \
                     \"reason\"}} with \"instance\" and \"role\" as the level needs"
                )}),
            );
        }
    };
    let Some(level) = crate::config::level_word(&change.level) else {
        return (
            400,
            "Bad Request",
            json!({"error": format!(
                "level {:?} is not one of global, instance, role",
                change.level
            )}),
        );
    };
    let failed = |error: anyhow::Error| {
        (
            500,
            "Internal Server Error",
            json!({"error": format!("{error:#}")}),
        )
    };
    let changed_at = match crate::records::now() {
        Ok(now) => now,
        Err(error) => return failed(error),
    };
    let input = PutConfig {
        actor: crate::actor::operator(operator),
        level,
        instance: change.instance.map(InstanceName),
        role: change.role.map(RoleName),
        path: change.path,
        value: change.value,
        reason: change.reason,
        changed_at,
    };
    let opened = crate::store::open_url(&target.store, super::TENANT)
        .map_err(|error| anyhow!("daemon.store: {error:#}"));
    let mut store = match opened {
        Ok(store) => store,
        Err(error) => return failed(error),
    };
    match crate::config::put::put_config(&target.file, &target.home, &mut store, &input) {
        Ok(Ok(data)) => {
            let changed = crate::config::put::changed(&data);
            println!(
                "conductord: config {} changed at {} by {}: {}",
                data.path,
                crate::store::level_name(data.level).to_ascii_lowercase(),
                crate::actor::written(&data.actor),
                data.revision_id.0.0
            );
            (
                200,
                "OK",
                json!({
                    "revision_id": data.revision_id.0.0.to_string(),
                    "actor": crate::actor::written(&changed.actor),
                    "level": crate::store::level_name(changed.level),
                    "instance": changed.instance_name.map(|name| name.0),
                    "role": changed.role.map(|role| role.0),
                    "path": changed.path,
                    "value": changed.value,
                    "reason": changed.reason,
                    "commit": data.commit,
                }),
            )
        }
        Ok(Err(refusal)) => {
            let status = match refusal.outcome {
                PutConfigOutcome::ActorNotHuman { .. } => (403, "Forbidden"),
                _ => (422, "Unprocessable Content"),
            };
            (status.0, status.1, json!({"error": refusal.to_string()}))
        }
        Err(error) => failed(error),
    }
}

/// The route of the secrets' names (`story:secrets-source`): `GET` lists them
/// ([`crate::secret::view`]), never a value.
pub const SECRETS: &str = "/v1/secrets";

/// The answer of the global API's federated routes, `ask` and `secrets`
/// (`story:federated-views`, [`federate`]), or none for another path. The events stream is
/// written to `stream` here and answers nothing more.
fn federated<S: Read + Write>(
    request: &Request,
    stream: &mut S,
    listener: Listener,
    who: &Who,
    api: &Api,
) -> Option<Vec<u8>> {
    let path = request.path();
    let only = |method: &str| {
        reply(
            405,
            "Method Not Allowed",
            &json!({"error": format!("{path} answers {method} only")}),
            listener,
        )
    };
    if path == SECRETS {
        if request.method != "GET" {
            return Some(only("GET"));
        }
        let (code, reason, answer) = secrets(api);
        return Some(reply(code, reason, &answer, listener));
    }
    if path == federate::ASK {
        if request.method != "POST" {
            return Some(only("POST"));
        }
        if let Err(refusal) = admit_write(request, listener, who, "ask", api) {
            return Some(refusal);
        }
        let body = match read_body(request, stream) {
            Ok(body) => body,
            Err(refusal) => return Some(refusal(listener)),
        };
        let (code, reason, answer) = ask(api, &body);
        return Some(reply(code, reason, &answer, listener));
    }
    let route = path
        .strip_prefix("/v1/")
        .filter(|route| federate::ROUTES.contains(route))?;
    if request.method != "GET" {
        return Some(only("GET"));
    }
    let seen = |name: &str| api.seen(name);
    if route == "events" {
        let cors = if matches!(listener, Listener::Tls) {
            CORS
        } else {
            ""
        };
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\nConnection: close\r\n{cors}\r\n"
        );
        federate::events(
            stream,
            head.as_bytes(),
            &|| api.members(),
            request.query(),
            api.timeout,
            &seen,
        );
        return Some(Vec::new());
    }
    let answer = federate::view(&api.members(), route, request.query(), api.timeout, &seen);
    Some(reply(200, "OK", &answer, listener))
}

/// `GET /v1/secrets`: the rows of [`crate::secret::view`] over the daemon's store and its config
/// file; `503` when the daemon was given no config file.
fn secrets(api: &Api) -> (u16, &'static str, Value) {
    let Some(target) = &api.config else {
        return (
            503,
            "Service Unavailable",
            json!({"error": "this daemon was given no config file to read"}),
        );
    };
    let failed = |error: anyhow::Error| {
        (
            500,
            "Internal Server Error",
            json!({"error": format!("{error:#}")}),
        )
    };
    let text = match fs::read_to_string(&target.file)
        .with_context(|| format!("read {}", target.file.display()))
    {
        Ok(text) => text,
        Err(error) => return failed(error),
    };
    let store = match crate::store::open_url(&target.store, super::TENANT)
        .map_err(|error| anyhow!("daemon.store: {error:#}"))
    {
        Ok(store) => store,
        Err(error) => return failed(error),
    };
    let rows = crate::secret::view(&store, &text, api.secrets);
    (200, "OK", serde_json::to_value(rows).unwrap_or_default())
}

/// The body of `POST /v1/ask`: the question, and the instance whose controller it is for.
#[derive(Deserialize)]
struct Question {
    text: String,
    to: Option<String>,
}

/// `POST /v1/ask` with `body` (decision F4): the question is for the supervisor, or with `to` for
/// that instance's controller through it; no supervisor runs until the daemon starts the host
/// roles, so a question is refused with `conductor.instance.SupervisorNotRunning` (`503`), and one
/// for an instance the daemon does not list with `conductor.instance.UnknownInstance` (`404`).
fn ask(api: &Api, body: &[u8]) -> (u16, &'static str, Value) {
    use conductor_model::config::InstanceName;
    use conductor_model::instance::{SupervisorNotRunning, UnknownInstance};
    let question = match serde_json::from_slice::<Question>(body) {
        Ok(question) if !question.text.trim().is_empty() => question,
        Ok(_) => {
            return (
                400,
                "Bad Request",
                json!({"error": "the question is empty: write {\"text\": \"...\"}"}),
            );
        }
        Err(error) => {
            return (
                400,
                "Bad Request",
                json!({"error": format!(
                    "the body is not a question: {error}; write {{\"text\", \"to\"}}, \"to\" an instance or absent"
                )}),
            );
        }
    };
    if let Some(to) = &question.to
        && !api.members().iter().any(|member| member.name == *to)
    {
        let refusal = UnknownInstance {
            name: InstanceName(to.clone()),
        };
        return (
            404,
            "Not Found",
            json!({"error": "conductor.instance.UnknownInstance", "name": refusal.name.0}),
        );
    }
    let refusal = SupervisorNotRunning {
        instance: question.to.map(InstanceName),
    };
    (
        503,
        "Service Unavailable",
        json!({
            "error": "conductor.instance.SupervisorNotRunning",
            "instance": refusal.instance.map(|name| name.0),
            "detail": "no supervisor runs to take the question; the daemon starts the host roles \
                       in a later release, and nothing was delivered",
        }),
    )
}

/// The code and reason of a status line such as `202 Accepted`.
fn status_parts(status: &'static str) -> (u16, &'static str) {
    status
        .split_once(' ')
        .and_then(|(code, reason)| Some((code.parse().ok()?, reason)))
        .unwrap_or((500, "Internal Server Error"))
}

/// A refusal answered on the listener it is given.
type Refusal = fn(Listener) -> Vec<u8>;

/// The body of a write: what was read with the head, then the rest up to its `Content-Length`.
fn read_body<S: Read>(request: &Request, stream: &mut S) -> Result<Vec<u8>, Refusal> {
    if request.header("transfer-encoding").is_some() {
        return Err(|listener| {
            reply(
                411,
                "Length Required",
                &json!({"error": "send the body with a Content-Length, not chunked"}),
                listener,
            )
        });
    }
    let Some(length) = request
        .header("content-length")
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return Err(|listener| {
            reply(
                411,
                "Length Required",
                &json!({"error": "send the body with a Content-Length"}),
                listener,
            )
        });
    };
    if length > MAX_FORWARD {
        return Err(|listener| {
            reply(
                413,
                "Content Too Large",
                &json!({"error": format!("a body is at most {MAX_FORWARD} bytes")}),
                listener,
            )
        });
    }
    let mut body = request.early[..request.early.len().min(length)].to_vec();
    if body.len() < length {
        let mut rest = vec![0_u8; length - body.len()];
        if stream.read_exact(&mut rest).is_err() {
            return Err(|listener| {
                reply(
                    400,
                    "Bad Request",
                    &json!({"error": "the body ended before its Content-Length"}),
                    listener,
                )
            });
        }
        body.extend_from_slice(&rest);
    }
    Ok(body)
}

/// The instance routes that write, each with its method: the answers, the settings and the Plan
/// page's moves.
const WRITES: [(&str, &str); 3] = [("answers", "POST"), ("settings", "PUT"), ("plan", "POST")];

/// Whether `method` on the instance route `route` writes: one of [`WRITES`], or `POST` on an
/// agent's `deliver` or `restart` (`agents/{name}/deliver`, `story:agent-kinds`).
fn writes(route: &str, method: &str) -> bool {
    if WRITES.contains(&(route, method)) {
        return true;
    }
    method == "POST"
        && route
            .strip_prefix("agents/")
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(name, verb)| !name.is_empty() && matches!(verb, "deliver" | "restart"))
}

/// The headers a proxied request keeps.
const FORWARDED: [&str; 3] = ["if-none-match", "content-type", "accept"];

/// Sends `request` to the instance at `target` as `/v1/<route>`, a write with the instance's token
/// and `operator`, and answers what the instance answered, byte for byte.
fn proxy(
    target: &Route,
    request: &Request,
    route: &str,
    body: &[u8],
    operator: Option<&str>,
) -> Result<Vec<u8>> {
    let mut stream = UnixStream::connect(&target.socket)
        .with_context(|| format!("connect to {}", target.socket.display()))?;
    stream.set_read_timeout(Some(PROXY_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let query = request
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let mut head = format!(
        "{} /v1/{route}{query} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n",
        request.method
    );
    for name in FORWARDED {
        if let Some(value) = request.header(name) {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
    }
    if let Some(operator) = operator {
        head.push_str(&format!(
            "{}: {}\r\n{}: {operator}\r\nContent-Length: {}\r\n",
            dashboard::TOKEN_HEADER,
            target.token,
            dashboard::OPERATOR_HEADER,
            body.len()
        ));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)?;
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer)?;
    if answer.is_empty() {
        bail!("it closed the connection without an answer");
    }
    Ok(answer)
}

/// The CORS headers a TLS answer carries: any origin may read with a bearer.
const CORS: &str = "Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Expose-Headers: ETag, X-Conductor-Age, Retry-After\r\n";

/// `bytes`, an instance's answer, with the CORS headers on the TLS listener.
fn with_cors(bytes: Vec<u8>, listener: Listener) -> Vec<u8> {
    if !matches!(listener, Listener::Tls) {
        return bytes;
    }
    match bytes.windows(2).position(|pair| pair == b"\r\n") {
        Some(end) => {
            let mut out = bytes[..end + 2].to_vec();
            out.extend_from_slice(CORS.as_bytes());
            out.extend_from_slice(&bytes[end + 2..]);
            out
        }
        None => bytes,
    }
}

/// The answer to a CORS preflight on the TLS listener.
fn preflight() -> Vec<u8> {
    format!(
        "HTTP/1.1 204 No Content\r\n{CORS}Access-Control-Allow-Methods: GET, POST, PUT\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type, If-None-Match\r\n\
         Access-Control-Max-Age: 600\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .into_bytes()
}

fn reply(status: u16, reason: &str, body: &Value, listener: Listener) -> Vec<u8> {
    let mut text = body.to_string();
    text.push('\n');
    response(
        status,
        reason,
        "application/json",
        text.as_bytes(),
        "",
        listener,
    )
}

fn response(
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    extra: &str,
    listener: Listener,
) -> Vec<u8> {
    let cors = if matches!(listener, Listener::Tls) {
        CORS
    } else {
        ""
    };
    let mut answer = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: {}\r\nConnection: close\r\n{cors}{extra}\r\n",
        body.len(),
        dashboard::CONTENT_SECURITY_POLICY,
    )
    .into_bytes();
    answer.extend_from_slice(body);
    answer
}

/// What the Identity service answers for a bearer it resolves (`GET /v1/access-authority`); the
/// fields the daemon reads.
#[derive(Debug, Deserialize)]
struct Authority {
    #[serde(rename = "sub")]
    subject: String,
    #[serde(rename = "aud")]
    audience: String,
    principal_kind: String,
    #[serde(default)]
    email: Option<String>,
}

/// The Identity service that resolves the TLS listener's bearers (`daemon.tls.identity`): an
/// `https://` origin, or an `http://` one on `127.0.0.1`, `localhost` or a cluster's
/// `*.svc.cluster.local`, as Identity's own client allows.
#[derive(Debug, Clone)]
pub struct Identity {
    host: String,
    port: u16,
    https: bool,
    audience: String,
}

impl Identity {
    /// The Identity service at `origin`, resolving bearers for `audience`.
    ///
    /// # Errors
    ///
    /// The origin is no URL of the kinds above, or carries a path.
    pub fn new(origin: &str, audience: &str) -> Result<Self> {
        let (https, rest) = if let Some(rest) = origin.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = origin.strip_prefix("http://") {
            (false, rest)
        } else {
            bail!("{origin} is no http:// or https:// origin");
        };
        let authority = rest.trim_end_matches('/');
        if authority.is_empty() || authority.contains('/') || authority.contains('@') {
            bail!("{origin} is no origin: give the scheme, the host and the port only");
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (
                host.to_owned(),
                port.parse::<u16>()
                    .map_err(|_| anyhow!("{origin}: the port {port} is no port"))?,
            ),
            None => (authority.to_owned(), if https { 443 } else { 80 }),
        };
        let internal =
            host == "127.0.0.1" || host == "localhost" || host.ends_with(".svc.cluster.local");
        if !https && !internal {
            bail!(
                "{origin}: an Identity service off this host or cluster is reached over https://"
            );
        }
        Ok(Self {
            host,
            port,
            https,
            audience: audience.to_owned(),
        })
    }

    /// The authority of `bearer`, as the service resolves it for the daemon's audience.
    fn resolve(&self, bearer: &str) -> Result<Authority> {
        if bearer.contains(['\r', '\n']) {
            bail!("the bearer holds a line break");
        }
        let request = format!(
            "GET /v1/access-authority HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {bearer}\r\n\
             x-b10x-audience: {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.host, self.audience
        );
        let tcp = TcpStream::connect((self.host.as_str(), self.port))
            .with_context(|| format!("connect to {}:{}", self.host, self.port))?;
        tcp.set_read_timeout(Some(IO_TIMEOUT))?;
        tcp.set_write_timeout(Some(IO_TIMEOUT))?;
        let bytes = if self.https {
            let config = client_config()?;
            let name = ServerName::try_from(self.host.clone())
                .map_err(|_| anyhow!("{} is no server name", self.host))?;
            let connection = rustls::ClientConnection::new(config, name)?;
            let mut tls = rustls::StreamOwned::new(connection, tcp);
            exchange(&mut tls, &request)?
        } else {
            let mut tcp = tcp;
            exchange(&mut tcp, &request)?
        };
        let (status, body) = split(&bytes)?;
        if status != 200 {
            bail!("it answered {status}");
        }
        let authority: Authority =
            serde_json::from_slice(&body).context("its answer is no access authority")?;
        if authority.audience != self.audience {
            bail!(
                "the bearer is for {}, not {}",
                authority.audience,
                self.audience
            );
        }
        Ok(authority)
    }
}

fn exchange<S: Read + Write>(stream: &mut S, request: &str) -> Result<Vec<u8>> {
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => Ok(bytes),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && !bytes.is_empty() => {
            Ok(bytes)
        }
        Err(error) => Err(error.into()),
    }
}

/// The status and the body of an HTTP answer, a chunked body joined.
pub(crate) fn split(bytes: &[u8]) -> Result<(u16, Vec<u8>)> {
    let end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("its answer has no head"))?;
    let head = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| anyhow!("its answer has no status"))?;
    let mut body = bytes[end + 4..].to_vec();
    if head.contains("transfer-encoding: chunked") {
        let mut joined = Vec::new();
        let mut rest = body.as_slice();
        while let Some(line) = rest.windows(2).position(|pair| pair == b"\r\n") {
            let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..line]).trim(), 16)
                .context("a chunk's size")?;
            rest = &rest[line + 2..];
            if size == 0 || rest.len() < size {
                break;
            }
            joined.extend_from_slice(&rest[..size]);
            rest = rest.get(size + 2..).unwrap_or_default();
        }
        body = joined;
    }
    Ok((status, body))
}

/// The roots an `https://` Identity service is checked against: the system's bundle, the file
/// `SSL_CERT_FILE` names, else the first of the usual places.
fn client_config() -> Result<Arc<rustls::ClientConfig>> {
    let file = std::env::var_os("SSL_CERT_FILE")
        .map(PathBuf::from)
        .into_iter()
        .chain(
            [
                "/etc/ssl/certs/ca-certificates.crt",
                "/etc/pki/tls/certs/ca-bundle.crt",
                "/etc/ssl/cert.pem",
            ]
            .map(PathBuf::from),
        )
        .find(|path| fs::metadata(path).is_ok())
        .ok_or_else(|| anyhow!("no system certificate bundle is found; set SSL_CERT_FILE"))?;
    let mut roots = rustls::RootCertStore::empty();
    for certificate in CertificateDer::pem_file_iter(&file)
        .with_context(|| format!("read {}", file.display()))?
        .flatten()
    {
        let _ = roots.add(certificate);
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(Arc::new(config))
}
