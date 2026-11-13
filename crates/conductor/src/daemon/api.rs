//! conductord's global API (`story:daemon-reconcile`, `story:instance-api`, design
//! architecture-next § 5.2, § 5.3). It answers on the loopback listener `127.0.0.1:<daemon.port>`
//! ([`serve`]) and, when `daemon.tls` names one, on a TLS listener with Identity bearers
//! ([`serve_tls`]), both at once:
//! - `/` and every file of the dashboard app ([`crate::dashboard::app_file`]): on loopback
//!   `index.html` carries the daemon's token in `<meta name="conductor-token">`, the one token a
//!   browser holds; on the TLS listener it carries none, and the API takes bearers only;
//! - `GET /v1/instances`: the rows the reconciler last published ([`Api::publish`]);
//! - the instance API of a consumer control plane (design A6): `POST /v1/instances` adds one
//!   whole instance to the config file, `GET`, `PUT` and `DELETE /v1/instances/{name}` read,
//!   replace and remove one ([`crate::config::instance`]), and `POST
//!   /v1/instances/{name}/restart` restarts its runtime ([`super::restart`]); a write of these is
//!   `ChangeSettings` (`restart`: `StartInstances`);
//! - `/v1/instances/{name}/<route>`: the instance's `/v1/<route>`, proxied to its runtime's socket,
//!   or to the TCP address of a runtime alone in a pod with the token on every request
//!   ([`Route`]), with the instance's token, which no browser holds; only its writing routes
//!   ([`WRITES`]) are admitted as writes, any other method is forwarded bare and the instance
//!   answers it 405. `start`, `stop`, `restart` and `upgrade` are the daemon's own routes and are
//!   not proxied; `POST /v1/instances/{name}/upgrade` hands an upgrade to the loop
//!   ([`super::upgrade::post`]), `restart` a restart ([`super::restart::post`]), `start` and
//!   `stop` are not served yet;
//! - `PUT /v1/config`: one key of the config changed ([`crate::config::put::put_config`]).
//! - `GET /v1/sessions`, `decisions`, `unanswered`, `kpis` and `events` (SSE): the union of every
//!   instance's answer, each row carrying `instance`, an instance that does not answer within
//!   `daemon.timeout` a row `state: unreachable` ([`federate`], `story:federated-views`);
//!   `POST /v1/ask`: delivered to the supervisor's session, refused with
//!   `conductor.instance.SupervisorNotRunning` while none runs; `POST /v1/roles/{role}/start`:
//!   a host role `globals.roles` lists asked for now ([`super::host_roles`]); `GET /v1/secrets`: the secrets' names, sources and instances, never a value
//!   ([`crate::secret::view`]);
//! - an instance's `agents/{name}/deliver` and `restart` are proxied as writes, `attach` bare.
//! - the work call of a consumer control plane (`story:control-plane-work-handler`,
//!   [`crate::dashboard::work`]): `POST /v1/instances/{name}/work` is proxied as a write with a
//!   body of up to 1 MiB, `GET /v1/instances/{name}/work/{reference}` as a read; a reference that
//!   is no reference (`conductor.dispatch.InvalidReference`) is `422` here, before its path
//!   reaches the runtime. For an instance that is not running both answer as every proxied route
//!   does: `404` naming no running instance, `503` before the reconciler published.
//! - `GET /healthz`, `/readyz` and `/metrics`: the probes and the metrics, answered without a
//!   bearer on both listeners, ahead of the app ([`super::metrics`],
//!   `story:daemon-probes-and-metrics`). While the reconciler has published nothing, an answer
//!   read from its publication (`GET /v1/instances`, the federated reads, a proxied route,
//!   `POST /v1/ask`, `POST /v1/roles/{role}/start`) is `503` with the readiness reason.
//! - `GET /v1/instances/{name}/tokens`, `POST .../join` and `POST .../leave`: the instance's token
//!   pool, the daemon's own routes ([`crate::token`], `story:token-pool`); a join or a leave is a
//!   write of the `Join` grant, refused `409` for a daemon that runs its instances as pods
//!   ([`Api::with_pods`]).
//! - `GET /v1/instances/{name}/secrets`: for each key of the instance's mapping that names a
//!   secret, whether the daemon's secrets source holds it, never a value; the daemon's own route,
//!   read from its config file and its source at each call ([`crate::secret::instance_view`],
//!   `story:instance-secret-state`).
//!
//! The control token (decision D2, [`Api::with_control_token`]): while the file names one
//! (`daemon.control_token: secret:<name>`), a request on either listener whose bearer equals its
//! value is the machine principal `control-plane`. It reads every `GET` under `/v1/` a person's
//! bearer reads but `GET /v1/secrets`, and writes `POST /v1/instances`, `PUT` and `DELETE
//! /v1/instances/{name}`, `POST /v1/instances/{name}/restart` and `POST
//! /v1/instances/{name}/work`, recorded as its own: the work call is forwarded with
//! `X-Conductor-Actor-Kind: control-plane`, which the daemon sets only for it. Every other write,
//! the proxied `answers`, `settings`, `plan` and an agent's `deliver` and `restart` among them, and
//! `GET /v1/secrets` refuse it as an agent is refused (`ActorNotHuman`). Any other bearer
//! is the Identity service's to admit on the bearer listener and nobody's on loopback, and is
//! answered one `401` that says nothing of the kind expected. While the file names none, both
//! listeners answer as they did before it.
//!
//! A local client that is no browser (`conductor instance upgrade`) writes with the token the
//! daemon keeps in [`TOKEN_FILE`] of its directory, mode 0600, naming the listener's origin.
//!
//! Admission. On loopback every request names this listener's host (or none), so a page served
//! from elsewhere cannot read through DNS rebinding; reads are open, and a write comes from this
//! listener's own page ([`crate::dashboard::same_origin`]) with the daemon's token. On the TLS
//! listener every request but the app's files and a CORS preflight carries `Authorization:
//! Bearer` (the probes and the metrics carry none: their bodies hold states, counts and ages
//! only), resolved by the Identity service `daemon.tls.identity` ([`Identity`]); a request without
//! one is answered `401` naming that service (`identity`), where `conductor login` obtains one
//! (`story:daemon-remote`). A browser logs in at `GET /login` instead ([`super::browser`]) and
//! presents the session cookie it is given where it has no bearer; a write with the cookie names
//! this listener's own origin. Any origin may
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
//! `X-Conductor-Operator`, and the instance records it as that operator's; a caller's own
//! `X-Conductor-Operator` or `X-Conductor-Actor-Kind` is never forwarded.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{DaemonOperator, OperatorGrant, SecretsSource};
use conductor_model::operator::Actor;
use conductor_model::secret::{SecretName, Source};
use conductor_model::store::{BinaryVersion, Version};
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use serde::Deserialize;
use serde_json::{Value, json};

use super::federate;
use super::host_roles::Supervisor;
use crate::dashboard;

/// The route of the instances list.
pub const INSTANCES: &str = "/v1/instances";

/// The prefix of the host roles' routes: `POST /v1/roles/{role}/start`
/// (`story:host-roles-by-daemon`).
pub const ROLES: &str = "/v1/roles/";

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
    /// For a runtime in a box on a remote substrate daemon, the relay session its exchanges run
    /// as there instead of the socket (`story:placement-substrate-remote`).
    pub remote: Option<crate::placement::RemoteRoute>,
    /// For a runtime alone in a pod, the address of its TCP listener instead of the socket
    /// ([`crate::instance::LISTEN_VARIABLE`]): every request sent there carries [`Self::token`],
    /// reads too ([`Route::tcp()`]).
    pub tcp: Option<SocketAddr>,
}

impl Route {
    /// The route to a runtime alone in a pod that serves its API on TCP at `address` and takes
    /// every request with `token`, the value its [`crate::instance::TOKEN_VARIABLE`] holds.
    #[must_use]
    pub fn tcp(address: SocketAddr, token: String) -> Self {
        Self {
            socket: PathBuf::new(),
            token,
            remote: None,
            tcp: Some(address),
        }
    }

    /// A connection to the runtime: to its TCP address within `within`, else to its socket.
    ///
    /// # Errors
    ///
    /// It does not connect.
    pub(crate) fn connect(&self, within: Duration) -> Result<Link> {
        match self.tcp {
            Some(address) => TcpStream::connect_timeout(&address, within)
                .map(Link::Tcp)
                .with_context(|| format!("connect to {address}")),
            None => UnixStream::connect(&self.socket)
                .map(Link::Unix)
                .with_context(|| format!("connect to {}", self.socket.display())),
        }
    }

    /// `request`, one whole HTTP request, as it is sent on this route: to a TCP address with the
    /// token in `X-Conductor-Token` after its request line, where its head carries none; as it
    /// is to a socket.
    pub(crate) fn carrying<'a>(&self, request: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        use std::borrow::Cow;
        if self.tcp.is_none() {
            return Cow::Borrowed(request);
        }
        let end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or(request.len());
        let head = String::from_utf8_lossy(&request[..end]);
        let carries = head
            .split("\r\n")
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .any(|(name, _)| name.trim().eq_ignore_ascii_case(dashboard::TOKEN_HEADER));
        let Some(line) = request.windows(2).position(|pair| pair == b"\r\n") else {
            return Cow::Borrowed(request);
        };
        if carries {
            return Cow::Borrowed(request);
        }
        let header = format!("{}: {}\r\n", dashboard::TOKEN_HEADER, self.token);
        let mut out = Vec::with_capacity(request.len() + header.len());
        out.extend_from_slice(&request[..line + 2]);
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(&request[line + 2..]);
        Cow::Owned(out)
    }
}

/// A connection to one runtime ([`Route::connect`]): its socket or its TCP listener.
pub(crate) enum Link {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Link {
    /// Bounds each read by `timeout`.
    pub(crate) fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.set_read_timeout(timeout),
            Self::Tcp(stream) => stream.set_read_timeout(timeout),
        }
    }

    /// Bounds each write by `timeout`.
    pub(crate) fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.set_write_timeout(timeout),
            Self::Tcp(stream) => stream.set_write_timeout(timeout),
        }
    }

    /// Ends the sending side.
    pub(crate) fn end_writes(&self) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.shutdown(Shutdown::Write),
            Self::Tcp(stream) => stream.shutdown(Shutdown::Write),
        }
    }
}

impl Read for Link {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Unix(stream) => stream.read(buf),
            Self::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for Link {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Unix(stream) => stream.write(buf),
            Self::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.flush(),
            Self::Tcp(stream) => stream.flush(),
        }
    }
}

/// What the reconciler publishes after each tick.
#[derive(Debug, Default)]
struct Published {
    rows: Vec<Value>,
    routes: BTreeMap<String, Route>,
    operators: Vec<DaemonOperator>,
    /// Whether the reconciler published at all: before it did, the rows are not empty but
    /// unread (`story:daemon-probes-and-metrics`).
    reconciled: bool,
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
    /// The browsers logged in on the TLS listener (`story:daemon-remote`).
    browsers: super::browser::Sessions,
    config: Option<ConfigTarget>,
    /// How long an instance has to answer one federated read.
    timeout: Duration,
    /// When each instance last answered a federated read, RFC 3339.
    answered: Mutex<BTreeMap<String, String>>,
    /// The secrets source `daemon.secrets` names: `GET /v1/secrets` lists a name only the config
    /// writes with its kind, and `GET /v1/instances/{name}/secrets` reads it at each call.
    secrets: SecretsSource,
    /// The names no instance's `GET /v1/instances/{name}/secrets` lists
    /// ([`crate::secret::withheld`]).
    withheld: BTreeSet<String>,
    /// The host roles `globals.roles` lists, and the supervisor's session while it runs
    /// (`story:host-roles-by-daemon`); none before the reconciler published them.
    hosts: Mutex<Option<(Vec<String>, Option<Supervisor>)>>,
    /// What `POST /v1/ask` delivers a question to the supervisor with.
    deliver: Deliver,
    /// What the probes and the metrics keep between requests
    /// (`story:daemon-probes-and-metrics`).
    probes: super::metrics::Probes,
    /// How long one answer of `/metrics` is kept: [`super::metrics::KEEP`] unless a case sets
    /// another ([`Self::with_metrics_keep`]).
    metrics_keep: Duration,
    /// Whether `daemon serve` is still starting ([`Self::starting`]): a request that is no probe
    /// waits for the reconciler's first publication, as it did while no listener took a request
    /// before it.
    starting: AtomicBool,
    /// The control token (decision D2): the secrets source `daemon.secrets` names and the secret
    /// `daemon.control_token` names in it, read at each request a bearer comes with; none while
    /// the file names no control token ([`Self::with_control_token`]).
    control: Option<(SecretsSource, SecretName)>,
    /// Whether the daemon runs its instances as pods (`daemon.kubernetes` set): an instance's
    /// token-pool join and leave are refused then ([`Self::with_pods`]).
    pods: bool,
}

/// How `POST /v1/ask` delivers a question to the supervisor's session: by default the
/// `claude-code` kind's deliver ([`super::host_roles::deliver`]).
pub type Deliver = Arc<dyn Fn(&Supervisor, &str) -> Result<()> + Send + Sync>;

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
            browsers: super::browser::Sessions::default(),
            config: None,
            timeout: Duration::from_secs(crate::config::FEDERATED_TIMEOUT),
            answered: Mutex::new(BTreeMap::new()),
            secrets: SecretsSource {
                source: Source::Keychain,
                namespace: None,
                url: None,
                dir: None,
            },
            withheld: BTreeSet::new(),
            hosts: Mutex::new(None),
            deliver: Arc::new(super::host_roles::deliver),
            probes: super::metrics::Probes::default(),
            metrics_keep: super::metrics::KEEP,
            starting: AtomicBool::new(false),
            control: None,
            pods: false,
        }
    }

    /// The API with the control token (decision D2): a request whose bearer equals the value
    /// `secrets` holds under `name` (`daemon.control_token`) is the machine principal
    /// `control-plane` ([`crate::actor::control_plane`]), on either listener. The value is read
    /// from the source at each request that comes with a bearer, never kept, and compared in
    /// constant time; an empty or missing value admits nobody. Without `name` the API answers as
    /// it does without this call.
    #[must_use]
    pub fn with_control_token(mut self, secrets: SecretsSource, name: Option<SecretName>) -> Self {
        self.control = name.map(|name| (secrets, name));
        self
    }

    /// The API of a daemon that runs its instances as pods (`daemon.kubernetes` set): a runtime
    /// in a pod keeps its own store and does not read the daemon's token pool, so an instance's
    /// `POST .../join` and `POST .../leave` are refused (design B2).
    #[must_use]
    pub const fn with_pods(mut self, pods: bool) -> Self {
        self.pods = pods;
        self
    }

    /// What the bearer of `request` is to the control token.
    fn control(&self, request: &Request) -> Control {
        let Some((secrets, name)) = &self.control else {
            return Control::Unset;
        };
        let Some(given) = presented(request) else {
            return Control::NoBearer;
        };
        let value = crate::secret::open(secrets)
            .ok()
            .and_then(|source| source.reveal(name).ok().flatten());
        match value {
            Some(value)
                if !value.expose().is_empty() && same_secret(value.expose(), given.as_bytes()) =>
            {
                Control::Admitted
            }
            _ => Control::Refused,
        }
    }

    /// The API of a daemon that serves before its reconciler published
    /// (`story:daemon-probes-and-metrics`). Until [`Self::publish`] or [`Self::started`] a
    /// request that is no probe waits, at most [`super::metrics::STORE_BOUND`], so a client that
    /// connects as soon as the listener is bound still reads what the reconciler first saw. The
    /// deployment's operators are `operators` meanwhile, as the config file writes them: a write
    /// is refused for a missing grant before the store answers as it is after.
    #[must_use]
    pub fn starting(mut self, operators: Vec<DaemonOperator>) -> Self {
        self.published
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .operators = operators;
        self.starting = AtomicBool::new(true);
        self
    }

    /// Ends the start without a publication: the store does not answer, and a request is
    /// answered without waiting, a read of the publication `503` ([`Self::unpublished`]).
    pub fn started(&self) {
        self.starting.store(false, Ordering::Release);
    }

    /// Waits while the daemon starts, at most [`super::metrics::STORE_BOUND`].
    fn await_start(&self) {
        let asked = Instant::now();
        while self.starting.load(Ordering::Acquire) && asked.elapsed() < super::metrics::STORE_BOUND
        {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Publishes whether the store answered the read just asked of it: what `GET /readyz`
    /// answers from (`story:daemon-probes-and-metrics`).
    pub fn publish_store(&self, answers: bool) {
        self.publish_store_at(answers, Instant::now());
    }

    /// [`Self::publish_store`] for a read answered, or not, at `at`.
    pub fn publish_store_at(&self, answers: bool, at: Instant) {
        self.probes.store(answers, at);
    }

    /// Publishes whether the reconciler's last record failed and none was written since: `GET
    /// /readyz` is `503` meanwhile (`story:daemon-probes-and-metrics`).
    pub fn publish_unrecorded(&self, failed: bool) {
        self.probes.unrecorded(failed);
    }

    /// Whether the reconciler published at all ([`Self::publish`]).
    #[must_use]
    pub fn is_reconciled(&self) -> bool {
        self.published
            .lock()
            .is_ok_and(|published| published.reconciled)
    }

    /// The answer to a request whose answer is read from the reconciler's publication while it
    /// has published nothing: `503` with the readiness reason, never an answer read from the
    /// nothing before it (`story:daemon-probes-and-metrics`).
    fn unpublished(&self, listener: Listener) -> Vec<u8> {
        let reason = super::metrics::readiness(self, Instant::now())
            .unwrap_or(super::metrics::NOT_PUBLISHED);
        reply(
            503,
            "Service Unavailable",
            &json!({"error": reason}),
            listener,
        )
    }

    /// Whether the store answered the last read asked of it; false before the first.
    #[must_use]
    pub fn store_answers(&self) -> bool {
        matches!(self.probes.seen(), Some((true, _)))
    }

    /// Whether the store was asked a read at all, answered or not.
    #[must_use]
    pub fn store_asked(&self) -> bool {
        self.probes.seen().is_some()
    }

    pub(super) const fn probes(&self) -> &super::metrics::Probes {
        &self.probes
    }

    /// How long an instance has to answer one federated read.
    pub(super) const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// How long one answer of `/metrics` is kept.
    pub(super) const fn metrics_keep(&self) -> Duration {
        self.metrics_keep
    }

    /// The host directory beside the config file ([`crate::config::host_of`]), where the build
    /// ledger is; none for an API without a config file.
    pub(super) fn host_dir(&self) -> Option<PathBuf> {
        self.config
            .as_ref()
            .map(|config| crate::config::host_of(&config.file))
    }

    /// The instances' rows and the instances as a federated route asks them, once the reconciler
    /// published; none before.
    pub(super) fn reconciled(&self) -> Option<(Vec<Value>, Vec<federate::Member>)> {
        self.is_reconciled().then(|| (self.rows(), self.members()))
    }

    /// The API with `deliver` taking `POST /v1/ask`'s question to the supervisor, in place of the
    /// `claude-code` kind's deliver.
    #[must_use]
    pub fn with_deliver(mut self, deliver: Deliver) -> Self {
        self.deliver = deliver;
        self
    }

    /// Publishes the host roles `globals.roles` lists and the supervisor's session while it runs
    /// (`story:host-roles-by-daemon`).
    pub fn publish_host(&self, listed: Vec<String>, supervisor: Option<Supervisor>) {
        *self.hosts.lock().unwrap_or_else(PoisonError::into_inner) = Some((listed, supervisor));
    }

    /// The host roles `globals.roles` lists; none before [`Self::publish_host`].
    fn listed_host_roles(&self) -> Option<Vec<String>> {
        self.hosts
            .lock()
            .ok()
            .and_then(|hosts| hosts.as_ref().map(|hosts| hosts.0.clone()))
    }

    fn supervisor(&self) -> Option<Supervisor> {
        self.hosts
            .lock()
            .ok()
            .and_then(|hosts| hosts.as_ref().and_then(|hosts| hosts.1.clone()))
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

    /// The API with `keep`, how long one answer of `/metrics` is kept, in place of
    /// [`super::metrics::KEEP`]; no config key sets it, a case does.
    #[must_use]
    pub const fn with_metrics_keep(mut self, keep: Duration) -> Self {
        self.metrics_keep = keep;
        self
    }

    /// The API with the secrets source of `daemon`, the section the daemon started with
    /// (`daemon.secrets`): `GET /v1/secrets` names its kind for a name only the config writes,
    /// and `GET /v1/instances/{name}/secrets` reads it, listing none of the names
    /// [`crate::secret::withheld`] gives for the section.
    #[must_use]
    pub fn with_secrets(mut self, daemon: &conductor_model::config::Daemon) -> Self {
        self.secrets = daemon.secrets.clone();
        self.withheld = crate::secret::withheld(daemon);
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
            reconciled: true,
        };
        drop(published);
        self.started();
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
        // An instance's runtime started again (`POST /v1/instances/{name}/restart`).
        "restart" => OperatorGrant::StartInstances,
        // story:token-pool: joining an instance, and leaving it, is the `Join` grant.
        "join" | "leave" => OperatorGrant::Join,
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
    /// The listener on `daemon.port` bound to a loopback address, `127.0.0.1` unless
    /// `daemon.address` names another: the address it is bound to.
    Loopback(SocketAddr),
    /// A listener reached from elsewhere, which takes bearers: the TLS listener, or the one on
    /// `daemon.port` bound to a `daemon.address` that is not loopback.
    Bearers,
}

/// Serves `listener`, on `daemon.port`, until it fails, each connection on a thread of its own:
/// on a loopback address as the loopback listener, on any other (`daemon.address`, as in a
/// container whose port is published) as the TLS listener is, a `/v1/` request taken with a
/// bearer only and no token in the page (`story:container-start-without-prompt`).
pub fn serve(listener: &TcpListener, api: &Arc<Api>) {
    // A listener whose address cannot be read is taken as reachable from elsewhere: it fails
    // closed.
    let kind = match listener.local_addr() {
        Ok(address) if address.ip().is_loopback() => Listener::Loopback(address),
        _ => Listener::Bearers,
    };
    for stream in listener.incoming().flatten() {
        let api = Arc::clone(api);
        std::thread::spawn(move || {
            let mut stream = stream;
            if timeouts(&stream).is_err() {
                return;
            }
            // A client that goes away mid-request costs only its own answer.
            let _ = connection(&mut stream, kind, &api);
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
            if connection(&mut tls, Listener::Bearers, &api).is_ok() {
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
    /// A bearer the Identity service resolved, or a browser's session cookie bound to one
    /// (`cookie`, `story:daemon-remote`): its operator name and whether it is a person.
    Bearer {
        name: String,
        human: bool,
        cookie: bool,
    },
    /// The machine principal `control-plane`: a bearer that equals the control token (decision
    /// D2), admitted for the reads and the instance routes only ([`CONTROL_PLANE_ROUTES`]).
    ControlPlane,
}

/// The routes besides the reads `control-plane` is admitted for, by the route word
/// [`admit_instance`] is asked for: `POST /v1/instances`, `PUT` and `DELETE /v1/instances/{name}`
/// (`instances`), `POST /v1/instances/{name}/restart` (`restart`) and the work call `POST
/// /v1/instances/{name}/work` (`work`, story:control-plane-work-handler). Every other write,
/// the proxied `answers`, `settings`, `plan` and an agent's `deliver` and `restart` included,
/// refuses it as it refuses an agent, and so does `GET /v1/secrets`.
const CONTROL_PLANE_ROUTES: [&str; 3] = ["instances", "restart", WORK];

/// What a request's bearer is to the control token ([`Api::with_control_token`]).
enum Control {
    /// The file names no control token: the listener admits as it does without one.
    Unset,
    /// The request carries no bearer.
    NoBearer,
    /// The bearer equals the control token.
    Admitted,
    /// The bearer is not the control token, or the token cannot be read or is empty.
    Refused,
}

/// The bearer `request` presents in `Authorization: Bearer`, if any.
fn presented(request: &Request) -> Option<&str> {
    request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// Whether `given` is `expected`, compared in constant time: their SHA-256 digests, which have one
/// length whatever the lengths of the two, compared byte by byte without an early end.
fn same_secret(expected: &[u8], given: &[u8]) -> bool {
    let (expected, given) = (
        ring::digest::digest(&ring::digest::SHA256, expected),
        ring::digest::digest(&ring::digest::SHA256, given),
    );
    expected
        .as_ref()
        .iter()
        .zip(given.as_ref())
        .fold(0_u8, |differs, (a, b)| differs | (a ^ b))
        == 0
}

/// The answer to a bearer neither the control token nor the Identity service admits, while the
/// file names a control token: one `401` whatever the bearer was and whichever kind was asked
/// for, so it tells a caller nothing of what is expected.
const NOT_ADMITTED: &str = "the bearer is not admitted";

fn not_admitted(listener: Listener, api: &Api) -> Vec<u8> {
    let identity = api.identity.as_ref().map(Identity::origin);
    reply(
        401,
        "Unauthorized",
        &json!({"error": NOT_ADMITTED, "identity": identity}),
        listener,
    )
}

/// The refusal of `control-plane` on a route it is not admitted for: as an agent is refused a
/// write, `conductor.operator.ActorNotHuman`.
fn control_plane_refused(listener: Listener) -> Vec<u8> {
    reply(
        403,
        "Forbidden",
        &json!({
            "error": "conductor.operator.ActorNotHuman",
            "actor": {"kind": "ControlPlane", "operator": null, "agent": null},
            "principal": "control-plane",
        }),
        listener,
    )
}

fn answer<S: Read + Write>(
    request: &Request,
    stream: &mut S,
    listener: Listener,
    api: &Api,
) -> Vec<u8> {
    let path = request.path().to_owned();
    if matches!(listener, Listener::Bearers) && request.method == "OPTIONS" {
        return preflight();
    }
    if matches!(listener, Listener::Bearers)
        && request.method == "GET"
        && (path == super::browser::LOGIN || path == super::browser::CALLBACK)
    {
        return browser_login(request, &path, api);
    }
    if let Listener::Loopback(bound) = listener
        && !own_host(request.header("host"), bound)
    {
        return reply(
            403,
            "Forbidden",
            &json!({"error": "this listener answers only its own host"}),
            listener,
        );
    }
    // The probes and the metrics answer without a bearer, ahead of the app
    // (`story:daemon-probes-and-metrics`): their bodies hold states, counts and ages only.
    if let Some(probe) = super::metrics::answer(&request.method, &path, api) {
        return response(
            probe.status,
            probe.reason,
            probe.content_type,
            probe.body.as_bytes(),
            probe.headers,
            listener,
        );
    }
    api.await_start();
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
    // Decision D2: a bearer that equals the control token is `control-plane` on either listener;
    // while the file names one, any other bearer is the Identity service's to admit on the bearer
    // listener and nobody's on loopback, each refusal the same `401`. While it names none, every
    // listener admits as before.
    let control = api.control(request);
    let who = match (listener, control) {
        (_, Control::Admitted) => Who::ControlPlane,
        (Listener::Loopback(_), Control::Refused) => return not_admitted(listener, api),
        (Listener::Loopback(_), _) => Who::Local,
        (Listener::Bearers, Control::Refused) => match bearer(request, api) {
            Ok(who) => who,
            Err(_) => return not_admitted(listener, api),
        },
        (Listener::Bearers, _) => match bearer(request, api) {
            Ok(who) => who,
            // The refusal names the Identity service, where `conductor login` obtains a bearer
            // (`story:daemon-remote`).
            Err(why) => {
                let identity = api.identity.as_ref().map(Identity::origin);
                return reply(
                    401,
                    "Unauthorized",
                    &json!({"error": why, "identity": identity}),
                    listener,
                );
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
        if request.method == "POST" {
            return instance_write(request, stream, listener, &who, api, None);
        }
        if request.method != "GET" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{INSTANCES} answers GET and POST only")}),
                listener,
            );
        }
        if !api.is_reconciled() {
            return api.unpublished(listener);
        }
        return reply(200, "OK", &Value::Array(api.rows()), listener);
    }
    // `/v1/instances/{name}`: one instance, read, replaced or removed (design A6).
    if let Some(name) = path
        .strip_prefix("/v1/instances/")
        .filter(|name| !name.is_empty() && !name.contains('/'))
    {
        return match request.method.as_str() {
            "GET" => {
                if !api.is_reconciled() {
                    return api.unpublished(listener);
                }
                let (code, reason, answer) = instance_row(api, name);
                reply(code, reason, &answer, listener)
            }
            "PUT" | "DELETE" => instance_write(request, stream, listener, &who, api, Some(name)),
            _ => reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{path} answers GET, PUT and DELETE only")}),
                listener,
            ),
        };
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
    // story:instance-secret-state: whether the secrets the instance names are set is read from
    // the daemon's own config file and its own source, never from the runtime's copy of them.
    if route == INSTANCE_SECRETS {
        if request.method != "GET" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{path} answers GET only")}),
                listener,
            );
        }
        let (code, reason, answer) = instance_secrets(api, name);
        return reply(code, reason, &answer, listener);
    }
    // story:token-pool: the instance's token pool is the daemon's own.
    if TOKEN_ROUTES.contains(&route) {
        return pool(request, stream, listener, &who, api, name, route);
    }
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
    if route == "restart" {
        if request.method != "POST" {
            return reply(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{path} answers POST only")}),
                listener,
            );
        }
        let actor = match admit_instance(request, listener, &who, route, api) {
            Ok(actor) => actor,
            Err(refusal) => return refusal,
        };
        if let Err(refusal) = optional_body(request, stream) {
            return refusal(listener);
        }
        let (status, answer) = super::restart::post(name, actor);
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
        // Before the reconciler published, no route is known, not none running.
        if !api.is_reconciled() {
            return api.unpublished(listener);
        }
        return reply(
            404,
            "Not Found",
            &json!({"error": format!("no running instance {name}")}),
            listener,
        );
    };
    // story:control-plane-work-handler: the reference is checked before it reaches the runtime's
    // path, so no `/`, `?` or `%` form is split there.
    if let Some(reference) = route.strip_prefix(WORK_PREFIX)
        && !crate::dispatch::valid_reference(reference)
    {
        return reply(
            422,
            "Unprocessable Content",
            &json!({
                "error": "conductor.dispatch.InvalidReference",
                "reference": reference,
                "detail": format!(
                    "a reference is 1 to {} characters of A-Z a-z 0-9 . _ - led by a letter or \
                     a digit",
                    crate::dispatch::REFERENCE_MOST
                ),
            }),
            listener,
        );
    }
    // Only an instance's writing routes are admitted and carry its token; any other method is
    // forwarded bare, and the instance answers it 405. The work call admits the control plane
    // ([`CONTROL_PLANE_ROUTES`]) and takes a larger body.
    let write = writes(route, &request.method);
    let actor = if !write {
        None
    } else if route == WORK {
        match admit_instance(request, listener, &who, route, api) {
            Ok(actor) => Some(actor),
            Err(refusal) => return refusal,
        }
    } else {
        match admit_write(request, listener, &who, route, api) {
            Ok(operator) => Some(crate::actor::operator(&operator)),
            Err(refusal) => return refusal,
        }
    };
    let body = if !write {
        Vec::new()
    } else if route == WORK {
        match read_body_within::<{ dashboard::work::MAX_BODY }, S>(request, stream) {
            Ok(body) => body,
            Err(refusal) => return refusal(listener),
        }
    } else {
        match read_body(request, stream) {
            Ok(body) => body,
            Err(refusal) => return refusal(listener),
        }
    };
    match proxy(&target, request, route, &body, actor.as_ref()) {
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

/// The route word of `GET /v1/instances/{name}/secrets` (`story:instance-secret-state`,
/// `conductor.secret.InstanceSecrets`).
const INSTANCE_SECRETS: &str = "secrets";

/// `GET /v1/instances/{name}/secrets`: the rows of [`crate::secret::instance_view`] over the
/// daemon's config file, read at this call, and the secrets source it started with, never a
/// value. Admitted as `GET /v1/instances/{name}` is, the control token included. `404`
/// `conductor.instance.UnknownInstance` for a name the file does not hold, `503`
/// `conductor.secret.SourceUnreachable` while the source does not answer, `503`
/// `conductor.secret.SourceUnusable` for a source it cannot use, `500` for a config file it
/// cannot read, and `503` when the daemon was given no config file. No body names a path: the
/// config file's path and the source's directory are in the daemon's line for the request
/// ([`instance_secrets_said`]).
fn instance_secrets(api: &Api, name: &str) -> (u16, &'static str, Value) {
    use crate::secret::Unanswered;
    let Some(target) = &api.config else {
        return (
            503,
            "Service Unavailable",
            json!({"error": "this daemon was given no config file to read"}),
        );
    };
    let text = match fs::read_to_string(&target.file) {
        Ok(text) => text,
        Err(error) => {
            instance_secrets_said(
                name,
                500,
                &format!("read {}: {error}", target.file.display()),
            );
            return (
                500,
                "Internal Server Error",
                json!({"error": "the daemon cannot read its config file; its log says why"}),
            );
        }
    };
    let dir = || {
        api.secrets
            .dir
            .as_deref()
            .map_or_else(String::new, |dir| format!(" (directory {dir})"))
    };
    match crate::secret::instance_view(&api.secrets, &text, name, &api.withheld) {
        Ok(Some(rows)) => (200, "OK", serde_json::to_value(rows).unwrap_or_default()),
        Ok(None) => (
            404,
            "Not Found",
            json!({"error": "conductor.instance.UnknownInstance", "name": name}),
        ),
        Err(Unanswered::Unreachable(error)) => {
            instance_secrets_said(
                name,
                503,
                &format!(
                    "SourceUnreachable: the {:?} source{} did not answer",
                    error.source,
                    dir()
                ),
            );
            (
                503,
                "Service Unavailable",
                json!({
                    "error": "conductor.secret.SourceUnreachable",
                    "source": format!("{:?}", error.source),
                }),
            )
        }
        Err(Unanswered::Unusable(unusable)) => {
            instance_secrets_said(name, 503, &format!("{unusable}{}", dir()));
            (
                503,
                "Service Unavailable",
                json!({
                    "error": "conductor.secret.SourceUnusable",
                    "source": format!("{:?}", unusable.error.source),
                }),
            )
        }
    }
}

/// The daemon's line on standard error for a `GET /v1/instances/{name}/secrets` that answered
/// `status`: `why`, with the paths the answer leaves out, never a secret's value.
fn instance_secrets_said(name: &str, status: u16, why: &str) {
    eprintln!("conductor: daemon serve: GET /v1/instances/{name}/secrets answered {status}: {why}");
}

/// The routes of an instance's token pool (`story:token-pool`): `GET tokens`, `POST join` and
/// `POST leave`.
const TOKEN_ROUTES: [&str; 3] = ["tokens", "join", "leave"];

/// `/v1/instances/{name}/<route>` of the token pool ([`crate::token`]): `GET tokens` reads it;
/// `POST join` and `POST leave` are writes of the operator admitted with the `Join` grant.
fn pool<S: Read>(
    request: &Request,
    stream: &mut S,
    listener: Listener,
    who: &Who,
    api: &Api,
    name: &str,
    route: &str,
) -> Vec<u8> {
    let method = if route == "tokens" { "GET" } else { "POST" };
    if request.method != method {
        return reply(
            405,
            "Method Not Allowed",
            &json!({"error": format!("/v1/instances/{name}/{route} answers {method} only")}),
            listener,
        );
    }
    let Some(target) = &api.config else {
        return reply(
            503,
            "Service Unavailable",
            &json!({"error": "this daemon was given no config file"}),
            listener,
        );
    };
    let operators = api.operators();
    let daemon = crate::token::Daemon {
        file: &target.file,
        store: &target.store,
        operators: &operators,
    };
    if route == "tokens" {
        let (code, reason, answer) = crate::token::tokens_route(&daemon, name);
        return reply(code, reason, &answer, listener);
    }
    let operator = match admit_write(request, listener, who, route, api) {
        Ok(operator) => operator,
        Err(refusal) => return refusal,
    };
    // Design B2: a runtime in a pod keeps its own store, which the daemon's pool is not.
    if api.pods {
        return reply(
            409,
            "Conflict",
            &json!({"error": format!(
                "{route} is refused: the daemon runs {name} as a pod, on a store of its own that \
                 does not read the daemon's token pool"
            )}),
            listener,
        );
    }
    let body = match read_body(request, stream) {
        Ok(body) => body,
        Err(refusal) => return refusal(listener),
    };
    let implied = matches!(who, Who::Local) && operators.is_empty();
    let (code, reason, answer) = if route == "join" {
        crate::token::join_route(&daemon, name, &operator, implied, &body)
    } else {
        crate::token::leave_route(&daemon, name, &operator)
    };
    reply(code, reason, &answer, listener)
}

/// Whether `host` names the loopback listener bound to `bound`, or is absent (a client that is no
/// browser): `127.0.0.1`, `localhost` or the address it is bound to (an IPv6 one in brackets,
/// `[::1]`), each with the listener's port or none.
fn own_host(host: Option<&str>, bound: SocketAddr) -> bool {
    let port = bound.port();
    let address = match bound {
        SocketAddr::V4(bound) => bound.ip().to_string(),
        SocketAddr::V6(bound) => format!("[{}]", bound.ip()),
    };
    host.is_none_or(|host| {
        ["127.0.0.1", "localhost", address.as_str()]
            .iter()
            .flat_map(|name| [(*name).to_owned(), format!("{name}:{port}")])
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

/// The bearer of `request`, resolved by the Identity service; without one, the browser session
/// its cookie names (`story:daemon-remote`).
fn bearer(request: &Request, api: &Api) -> Result<Who, String> {
    let Some(given) = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
    else {
        if let Some((name, human)) = api.browsers.who(request.header("cookie")) {
            return Ok(Who::Bearer {
                name,
                human,
                cookie: true,
            });
        }
        return Err(format!(
            "this listener takes an Identity bearer (Authorization: Bearer) or a browser's \
             session cookie from {}",
            super::browser::LOGIN
        ));
    };
    let (name, human) = resolved(api, given)
        .map_err(|error| format!("the Identity service refused the bearer: {error:#}"))?;
    Ok(Who::Bearer {
        name,
        human,
        cookie: false,
    })
}

/// The operator name and whether it is a person, of the access token `given`, as the Identity
/// service resolves it: named as `daemon.operators` names it (its email or subject).
fn resolved(api: &Api, given: &str) -> Result<(String, bool)> {
    let identity = api
        .identity
        .as_ref()
        .ok_or_else(|| anyhow!("this daemon names no Identity service (daemon.tls.identity)"))?;
    let authority = identity.resolve(given)?;
    let operators = api.operators();
    let name = operators
        .iter()
        .map(|operator| operator.name.0.clone())
        .find(|name| {
            authority.email.as_deref() == Some(name.as_str()) || authority.subject == *name
        })
        .or(authority.email)
        .unwrap_or(authority.subject);
    Ok((name, authority.principal_kind == "human"))
}

/// `GET /login` and `GET /login/callback` on the TLS listener ([`super::browser`]).
fn browser_login(request: &Request, path: &str, api: &Api) -> Vec<u8> {
    let listener = Listener::Bearers;
    let Some(identity) = &api.identity else {
        return reply(
            503,
            "Service Unavailable",
            &json!({"error": "this daemon names no Identity service (daemon.tls.identity)"}),
            listener,
        );
    };
    let sessions = &api.browsers;
    if path == super::browser::LOGIN {
        return match super::browser::begin(identity, sessions, request.header("host")) {
            Ok(address) => response(
                302,
                "Found",
                "text/plain; charset=utf-8",
                b"",
                &format!("Location: {address}\r\n"),
                listener,
            ),
            Err(error) => reply(
                502,
                "Bad Gateway",
                &super::browser::refusal(&error),
                listener,
            ),
        };
    }
    match super::browser::finish(
        identity,
        sessions,
        request.query().unwrap_or(""),
        |access| resolved(api, access),
    ) {
        Ok(value) => response(
            302,
            "Found",
            "text/plain; charset=utf-8",
            b"",
            &format!(
                "Location: /\r\nSet-Cookie: {}\r\n",
                super::browser::set_cookie(&value)
            ),
            listener,
        ),
        Err(error) => reply(
            400,
            "Bad Request",
            &super::browser::refusal(&error),
            listener,
        ),
    }
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
    // Decision D2: `control-plane` writes the instance routes only ([`admit_instance`]).
    if matches!(who, Who::ControlPlane) {
        return Err(control_plane_refused(listener));
    }
    if let Listener::Loopback(bound) = listener {
        let header = |name: &str| request.header(name);
        let token = request.header(dashboard::TOKEN_HEADER);
        if !dashboard::same_origin(header, bound)
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
    // A browser's cookie travels with every request to this host, so a write with it must name
    // the listener's own origin (`story:daemon-remote`); a bearer is presented on purpose.
    if let Who::Bearer { cookie: true, .. } = who {
        let own = super::browser::own_origin(request.header("host"));
        if own.is_none() || request.header("origin") != own.as_deref() {
            return Err(reply(
                403,
                "Forbidden",
                &json!({
                    "error": "a write with the session cookie is taken only from this listener's \
                              own origin",
                }),
                listener,
            ));
        }
    }
    let name = match who {
        Who::Local => api.local.clone(),
        Who::Bearer {
            name, human: false, ..
        } => {
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
        Who::Bearer {
            name, human: true, ..
        } => name.clone(),
        Who::ControlPlane => return Err(control_plane_refused(listener)),
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

/// Admits a write of `who` on the instance route `route` ([`CONTROL_PLANE_ROUTES`]): the control
/// plane by its token, anyone else as [`admit_write`] admits a write. Answers the actor.
fn admit_instance(
    request: &Request,
    listener: Listener,
    who: &Who,
    route: &str,
    api: &Api,
) -> Result<Actor, Vec<u8>> {
    if matches!(who, Who::ControlPlane) && CONTROL_PLANE_ROUTES.contains(&route) {
        return Ok(crate::actor::control_plane());
    }
    admit_write(request, listener, who, route, api).map(|name| crate::actor::operator(&name))
}

/// The body of a write that may have none (`DELETE /v1/instances/{name}`, `POST
/// .../restart`): empty without a `Content-Length` or a `Transfer-Encoding`, else [`read_body`].
fn optional_body<S: Read>(request: &Request, stream: &mut S) -> Result<Vec<u8>, Refusal> {
    if request.header("content-length").is_none() && request.header("transfer-encoding").is_none() {
        return Ok(Vec::new());
    }
    read_body(request, stream)
}

/// The body of `POST /v1/instances` and `PUT /v1/instances/{name}`: the instance's mapping as the
/// config file writes it, a JSON object (or its YAML text), and why; `DELETE` takes the reason
/// alone. The actor is the principal admitted and the time is now, as for `PUT /v1/config`; a
/// `name` beside the mapping names the instance where the mapping names none.
#[derive(Deserialize, Default)]
struct InstanceChange {
    #[serde(default)]
    mapping: Option<Value>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /v1/instances` (`path_name` none) or `PUT`/`DELETE /v1/instances/{path_name}`: admitted
/// for the control plane and for an operator with `ChangeSettings`, then the change.
fn instance_write<S: Read>(
    request: &Request,
    stream: &mut S,
    listener: Listener,
    who: &Who,
    api: &Api,
    path_name: Option<&str>,
) -> Vec<u8> {
    let actor = match admit_instance(request, listener, who, "instances", api) {
        Ok(actor) => actor,
        Err(refusal) => return refusal,
    };
    let body = if request.method == "DELETE" {
        optional_body(request, stream)
    } else {
        read_body(request, stream)
    };
    let body = match body {
        Ok(body) => body,
        Err(refusal) => return refusal(listener),
    };
    let (code, reason, answer) = change_instance(api, &request.method, path_name, &body, &actor);
    reply(code, reason, &answer, listener)
}

/// The `422` of a change that does not validate: the error's name, the YAML path of its first
/// problem, and every problem on one line.
fn config_invalid(path: &str, detail: &str) -> (u16, &'static str, Value) {
    (
        422,
        "Unprocessable Content",
        json!({
            "error": "conductor.config.ConfigInvalid",
            "path": path,
            "detail": detail.replace(['\r', '\n'], " "),
        }),
    )
}

/// One change of a whole instance by `actor` ([`crate::config::instance`]), recorded in the
/// daemon's store: `201` (`POST`) or `200` with the instance's name and the revision's id; `403`
/// `ActorNotHuman`, `404` `UnknownInstance`, `409` `InstanceExists`, `422` `ConfigInvalid` naming
/// the YAML path; `400` for a body that is no change, `503` without a config file.
fn change_instance(
    api: &Api,
    method: &str,
    path_name: Option<&str>,
    body: &[u8],
    actor: &Actor,
) -> (u16, &'static str, Value) {
    use crate::config::instance::{self, Refusal as Refused};
    use conductor_model::config::{AddInstance, InstanceName, PutInstance, RemoveInstance};
    let Some(target) = &api.config else {
        return (
            503,
            "Service Unavailable",
            json!({"error": "this daemon was given no config file to change"}),
        );
    };
    let change = if body.iter().all(u8::is_ascii_whitespace) {
        InstanceChange::default()
    } else {
        match serde_json::from_slice::<InstanceChange>(body) {
            Ok(change) => change,
            Err(error) => {
                return (
                    400,
                    "Bad Request",
                    json!({"error": format!(
                        "the body is not an instance change: {error}; write {{\"mapping\": \
                         {{<the instance's keys as the config file writes them>}}, \"reason\"}}"
                    )}),
                );
            }
        }
    };
    let reason = change.reason.clone().unwrap_or_default();
    let mapping = match (method, change.mapping) {
        ("DELETE", _) => None,
        (_, Some(Value::Object(map))) => match serde_yaml::to_string(&map) {
            Ok(text) => Some(text),
            Err(error) => return config_invalid("mapping", &format!("mapping: {error}")),
        },
        (_, Some(Value::String(text))) => Some(text),
        (_, _) => {
            return (
                400,
                "Bad Request",
                json!({"error": "the body holds no mapping: write {\"mapping\": {<the instance's \
                                 keys as the config file writes them>}, \"reason\"}"}),
            );
        }
    };
    let name = match path_name {
        Some(name) => name.to_owned(),
        None => {
            let written = mapping
                .as_deref()
                .and_then(|text| serde_yaml::from_str::<serde_yaml::Value>(text).ok())
                .and_then(|yaml| {
                    yaml.get("name")
                        .and_then(|name| name.as_str().map(str::to_owned))
                });
            match (written, change.name) {
                (Some(written), Some(named)) if written != named => {
                    return config_invalid(
                        "name",
                        &format!("name: the mapping names {written:?}, the body {named:?}"),
                    );
                }
                (Some(name), _) | (None, Some(name)) => name,
                (None, None) => {
                    return config_invalid(
                        "name",
                        "name: the mapping names no instance: write its name",
                    );
                }
            }
        }
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
    let mut store = match crate::store::open_url(&target.store, super::TENANT)
        .map_err(|error| anyhow!("daemon.store: {error:#}"))
    {
        Ok(store) => store,
        Err(error) => return failed(error),
    };
    let (file, home) = (&target.file, &target.home);
    let actor = actor.clone();
    let instance_name = InstanceName(name.clone());
    let (done, created) = match (method, mapping) {
        ("POST", Some(mapping)) => {
            let input = AddInstance {
                actor,
                name: instance_name,
                mapping,
                reason,
                changed_at,
            };
            (instance::add_instance(file, home, &mut store, &input), true)
        }
        ("PUT", Some(mapping)) => {
            let input = PutInstance {
                actor,
                name: instance_name,
                mapping,
                reason,
                changed_at,
            };
            (
                instance::put_instance(file, home, &mut store, &input),
                false,
            )
        }
        _ => {
            let input = RemoveInstance {
                actor,
                name: instance_name,
                reason,
                changed_at,
            };
            (
                instance::remove_instance(file, home, &mut store, &input),
                false,
            )
        }
    };
    match done {
        Ok(Ok(data)) => {
            let verb = match method {
                "POST" => "added",
                "PUT" => "replaced",
                _ => "removed",
            };
            println!(
                "conductord: instance {name} {verb} by {}: {}",
                crate::actor::written(&data.actor),
                data.revision_id.0.0
            );
            let (code, reason) = if created {
                (201, "Created")
            } else {
                (200, "OK")
            };
            (
                code,
                reason,
                json!({
                    "name": name,
                    "revision": data.revision_id.0.0.to_string(),
                    "actor": crate::actor::written(&data.actor),
                    "commit": data.commit,
                }),
            )
        }
        Ok(Err(refusal)) => match &refusal {
            Refused::ActorNotHuman(actor) => (
                403,
                "Forbidden",
                json!({
                    "error": "conductor.operator.ActorNotHuman",
                    "actor": crate::actor::to_json(actor),
                }),
            ),
            Refused::Exists(name) => (
                409,
                "Conflict",
                json!({"error": "conductor.config.InstanceExists", "name": name}),
            ),
            Refused::Unknown(name) => (
                404,
                "Not Found",
                json!({"error": "conductor.instance.UnknownInstance", "name": name}),
            ),
            Refused::Invalid(_) => {
                config_invalid(refusal.path().unwrap_or("."), &refusal.to_string())
            }
        },
        Err(error) => failed(error),
    }
}

/// `GET /v1/instances/{name}` (`conductor.instance.InstanceByName`): the row of `GET
/// /v1/instances` for the instance, while the config file holds it; a `404` naming it while the
/// file holds none of that name. An instance the file holds and the reconciler has not published
/// yet reads `Stopped`, as the reconciler writes one whose runtime it did not start. The row
/// carries the instance's `git_identity`, `model_token_kind` and `standing_rules` as the file
/// holds them, each where it holds one.
fn instance_row(api: &Api, name: &str) -> (u16, &'static str, Value) {
    let unknown = || {
        (
            404,
            "Not Found",
            json!({"error": "conductor.instance.UnknownInstance", "name": name}),
        )
    };
    let published = api
        .rows()
        .into_iter()
        .find(|row| row["name"].as_str() == Some(name));
    let held = api.config.as_ref().and_then(|target| {
        let text = fs::read_to_string(&target.file).ok()?;
        crate::config::parse(&text, &target.home).ok()
    });
    let Some(config) = held else {
        return published.map_or_else(unknown, |row| (200, "OK", row));
    };
    let Some(instance) = config
        .instances
        .iter()
        .find(|instance| instance.name.0 == name)
    else {
        return unknown();
    };
    let mut row = published.unwrap_or_else(|| {
        json!({
            "name": name,
            "state": "Stopped",
            "placement": instance.placement.0,
            "roles": instance.roles.iter().map(|role| role.role.clone()).collect::<Vec<_>>(),
            "sources": instance
                .sources
                .iter()
                .map(super::reconcile::source_json)
                .collect::<Vec<_>>(),
            "binary_version": Value::Null,
            "last_seen": Value::Null,
            "box_id": Value::Null,
            "detail": Value::Null,
            "changed_by": crate::actor::to_json(&crate::actor::daemon()),
        })
    });
    // The instance's git identity as the file holds it now (story:instance-git-identity), no
    // secret; no member where it holds none, as the specification's wire leaves an absent one out.
    if let (Some(identity), Some(row)) = (&instance.git_identity, row.as_object_mut()) {
        row.insert(
            "git_identity".to_owned(),
            json!({"name": identity.name, "email": identity.email}),
        );
    }
    // The kind of its model token as the file holds it now, never the token's value; no member
    // where it holds none.
    if let (Some(kind), Some(row)) = (instance.model_token_kind, row.as_object_mut()) {
        row.insert(
            "model_token_kind".to_owned(),
            json!(crate::config::model_token_kind_word(kind)),
        );
    }
    // Its standing rules as the file holds them now (story:instance-rules-text); no member where
    // it holds none.
    if let (Some(rules), Some(row)) = (&instance.standing_rules, row.as_object_mut()) {
        row.insert("standing_rules".to_owned(), json!(rules));
    }
    (200, "OK", row)
}

/// The route of the secrets' names (`story:secrets-source`): `GET` lists them
/// ([`crate::secret::view`]), never a value.
pub const SECRETS: &str = "/v1/secrets";

/// The route of the daemon's version (`story:daemon-version-json`, `conductor.store.Version`):
/// `GET` answers its one row, `version` and `config`, the label values of
/// `conductor_build_info` on `/metrics` ([`super::metrics::Build::this`]). It reads no
/// publication, so it answers before the first reconcile, and admits whoever `GET`
/// [`INSTANCES`] admits, `control-plane` included.
pub const VERSION: &str = "/v1/version";

/// The answer of the global API's federated routes, `ask`, `secrets` and `version`
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
        // Decision D2: the secrets routes refuse `control-plane` as the writes do.
        if matches!(who, Who::ControlPlane) {
            return Some(control_plane_refused(listener));
        }
        let (code, reason, answer) = secrets(api);
        return Some(reply(code, reason, &answer, listener));
    }
    if path == VERSION {
        if request.method != "GET" {
            return Some(only("GET"));
        }
        let build = super::metrics::Build::this();
        let row = Version {
            version: BinaryVersion(build.version),
            config: build.config,
        };
        let answer = json!([{"version": row.version.0, "config": row.config}]);
        return Some(reply(200, "OK", &answer, listener));
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
        // The instances and the supervisor it is checked against are the reconciler's.
        if !api.is_reconciled() {
            return Some(api.unpublished(listener));
        }
        let (code, reason, answer) = ask(api, &body);
        return Some(reply(code, reason, &answer, listener));
    }
    if let Some(role) = path
        .strip_prefix(ROLES)
        .and_then(|rest| rest.strip_suffix("/start"))
        .filter(|role| !role.is_empty() && !role.contains('/'))
    {
        if request.method != "POST" {
            return Some(only("POST"));
        }
        if let Err(refusal) = admit_write(request, listener, who, "start", api) {
            return Some(refusal);
        }
        if let Err(refusal) = read_body(request, stream) {
            return Some(refusal(listener));
        }
        // The roles `globals.roles` lists are what the reconciler published of them.
        if api.listed_host_roles().is_none() {
            return Some(api.unpublished(listener));
        }
        let (code, reason, answer) = start_role(api, role);
        return Some(reply(code, reason, &answer, listener));
    }
    let route = path
        .strip_prefix("/v1/")
        .filter(|route| federate::ROUTES.contains(route))?;
    if request.method != "GET" {
        return Some(only("GET"));
    }
    // Every federated read asks the instances the reconciler published.
    if !api.is_reconciled() {
        return Some(api.unpublished(listener));
    }
    let seen = |name: &str| api.seen(name);
    if route == "events" {
        let cors = if matches!(listener, Listener::Bearers) {
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
    let rows = crate::secret::view(&store, &text, api.secrets.source);
    (200, "OK", serde_json::to_value(rows).unwrap_or_default())
}

/// The body of `POST /v1/ask`: the question, and the instance whose controller it is for.
#[derive(Deserialize)]
struct Question {
    text: String,
    to: Option<String>,
}

/// `POST /v1/ask` with `body` (decision F4): the question is for the supervisor, or with `to` for
/// that instance's controller through it. While the supervisor's session runs
/// (`story:host-roles-by-daemon`) the question is delivered to it ([`Deliver`]), `202`, a
/// delivery that fails `502` naming `conductor.agent.AgentNotReachable`; while none runs it is
/// refused with `conductor.instance.SupervisorNotRunning` (`503`), and one for an instance the
/// daemon does not list with `conductor.instance.UnknownInstance` (`404`).
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
    let Some(supervisor) = api.supervisor() else {
        let refusal = SupervisorNotRunning {
            instance: question.to.map(InstanceName),
        };
        return (
            503,
            "Service Unavailable",
            json!({
                "error": "conductor.instance.SupervisorNotRunning",
                "instance": refusal.instance.map(|name| name.0),
                "detail": "no supervisor runs to take the question: the daemon starts it when \
                           globals.roles lists supervisor; nothing was delivered",
            }),
        );
    };
    // A question for an instance's controller goes through the supervisor, its first line
    // naming the instance.
    let text = match &question.to {
        Some(instance) => format!(
            "Question for the controller of {instance}:\n{}",
            question.text
        ),
        None => question.text.clone(),
    };
    match (api.deliver)(&supervisor, &text) {
        Ok(()) => (
            202,
            "Accepted",
            json!({"delivered_to": supervisor.name, "instance": question.to}),
        ),
        Err(error) => (
            502,
            "Bad Gateway",
            json!({
                "error": "conductor.agent.AgentNotReachable",
                "name": supervisor.name,
                "detail": format!("{error:#}"),
            }),
        ),
    }
}

/// `POST /v1/roles/{role}/start` (`story:host-roles-by-daemon`): asks the daemon's loop for the
/// host role `role` now ([`super::host_roles::ask`]), `202`; a role that is no host role, or one
/// `globals.roles` does not list, `404` naming `globals.roles`.
fn start_role(api: &Api, role: &str) -> (u16, &'static str, Value) {
    if !super::host_roles::is_host_role(role)
        || !api
            .listed_host_roles()
            .unwrap_or_default()
            .iter()
            .any(|r| r == role)
    {
        return (
            404,
            "Not Found",
            json!({"error": format!(
                "globals.roles lists no host role {role:?}: the host roles are {}, and the daemon \
                 starts one the file lists",
                super::host_roles::HOST_ROLES.join(" and ")
            )}),
        );
    }
    super::host_roles::ask(role);
    (
        202,
        "Accepted",
        json!({"role": role, "placement": super::host_roles::PLACEMENT}),
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

/// The body of a write: what was read with the head, then the rest up to its `Content-Length`,
/// at most [`MAX_FORWARD`].
fn read_body<S: Read>(request: &Request, stream: &mut S) -> Result<Vec<u8>, Refusal> {
    read_body_within::<MAX_FORWARD, S>(request, stream)
}

/// [`read_body`] of at most `MOST` bytes, refused `413` from its `Content-Length` before it is
/// read.
fn read_body_within<const MOST: usize, S: Read>(
    request: &Request,
    stream: &mut S,
) -> Result<Vec<u8>, Refusal> {
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
    if length > MOST {
        return Err(|listener| {
            reply(
                413,
                "Content Too Large",
                &json!({"error": format!("a body is at most {MOST} bytes")}),
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

/// The instance routes that write, each with its method: the answers, the settings, the Plan
/// page's moves and the work call (story:control-plane-work-handler).
const WRITES: [(&str, &str); 4] = [
    ("answers", "POST"),
    ("settings", "PUT"),
    ("plan", "POST"),
    (WORK, "POST"),
];

/// The route word of the work call: `POST /v1/instances/{name}/work`, and `GET .../work/
/// {reference}` under [`WORK_PREFIX`], forwarded to the runtime's `/v1/work`
/// ([`crate::dashboard::work`]).
const WORK: &str = "work";

/// The prefix of the work call's read.
const WORK_PREFIX: &str = "work/";

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
/// and its `actor`, and answers what the instance answered, byte for byte ([`send`]). The actor is
/// an operator, named in `X-Conductor-Operator`, or the control plane, named by
/// `X-Conductor-Actor-Kind: control-plane` ([`dashboard::work::ACTOR_KIND_HEADER`]); a caller's
/// own header of either name is never forwarded ([`FORWARDED`]).
fn proxy(
    target: &Route,
    request: &Request,
    route: &str,
    body: &[u8],
    actor: Option<&Actor>,
) -> Result<Vec<u8>> {
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
    if let Some(actor) = actor {
        let who = match (actor.kind, &actor.operator) {
            (conductor_model::operator::ActorKind::ControlPlane, _) => format!(
                "{}: {}",
                dashboard::work::ACTOR_KIND_HEADER,
                dashboard::work::CONTROL_PLANE
            ),
            (_, Some(operator)) => format!("{}: {}", dashboard::OPERATOR_HEADER, operator.0),
            (_, None) => bail!("a write is forwarded as an operator or the control plane"),
        };
        head.push_str(&format!(
            "{}: {}\r\n{who}\r\nContent-Length: {}\r\n",
            dashboard::TOKEN_HEADER,
            target.token,
            body.len()
        ));
    }
    head.push_str("\r\n");
    let mut whole = head.into_bytes();
    whole.extend_from_slice(body);
    send(target, &whole)
}

/// Sends `request`, one whole HTTP request, to the instance at `target` and answers what it
/// answered, byte for byte: over its socket; for a runtime alone in a pod, to its TCP address
/// with its token, reads too ([`Route::tcp`]); or, for a runtime in a box on a remote substrate
/// daemon, through one relay session in its box (`story:placement-substrate-remote`,
/// [`crate::placement::RemoteRoute`]).
///
/// # Errors
///
/// It does not connect, or closes the connection without an answer.
pub fn send(target: &Route, request: &[u8]) -> Result<Vec<u8>> {
    if let Some(remote) = &target.remote {
        return remote.send(request);
    }
    let mut stream = target.connect(IO_TIMEOUT)?;
    stream.set_read_timeout(Some(PROXY_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.write_all(&target.carrying(request))?;
    stream.flush()?;
    stream.end_writes()?;
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
    if !matches!(listener, Listener::Bearers) {
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
    let cors = if matches!(listener, Listener::Bearers) {
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

    /// The service's origin, `https://host:port` (`http://` on this host or cluster).
    #[must_use]
    pub fn origin(&self) -> String {
        format!(
            "{}://{}:{}",
            if self.https { "https" } else { "http" },
            self.host,
            self.port
        )
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

pub(crate) fn exchange<S: Read + Write>(stream: &mut S, request: &str) -> Result<Vec<u8>> {
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
pub(crate) fn client_config() -> Result<Arc<rustls::ClientConfig>> {
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
