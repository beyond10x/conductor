//! A small client of the Kubernetes API for a process that runs in a pod: conductord, which
//! supervises one pod per instance.
//!
//! - **Where the API is.** [`Config::in_cluster`] reads the API server's address from
//!   `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT`, and the service account the kubelet
//!   mounts at [`SERVICE_ACCOUNT_DIR`]: `ca.crt`, `token`, `namespace`.
//!   `CONDUCTOR_SERVICE_ACCOUNT_DIR` ([`DIR_VARIABLE`]) names another directory of those three
//!   files, as a test of the daemon's process does; set empty, or to a relative path, it is
//!   refused. Out of a cluster it answers [`Error::NotInCluster`], naming each variable and file
//!   that is missing.
//! - **Trust.** The cluster's `ca.crt` is the only root: a server another authority signed is
//!   refused ([`Error::Tls`]). The token is read again on every call, since the kubelet rotates it.
//! - **Calls.** Create, get, replace ([`Client::replace`]), list by label selector and delete of
//!   pods, persistent volume claims and secrets ([`Kind`]) in a namespace given per call, over
//!   JSON as [`serde_json::Value`]; a
//!   `SelfSubjectAccessReview` ([`Client::may`]); the last lines of a pod's log
//!   ([`Client::log_tail`]). [`Pod::of`] reads what a supervisor needs of a pod.
//! - **Answers.** One HTTP/1.1 exchange per call, `Connection: close`, its body read by length or
//!   in chunks. An API's `Status` is [`Error::Api`], its code a number: `404` and `409` are told
//!   apart by [`Error::is_not_found`] and [`Error::is_conflict`].
//! - **Deadline.** Every call, connect and handshake included, ends within the config's deadline
//!   ([`Error::Timeout`]); every read and write of the socket waits at most for what is left of it.
//!
//! No watch, no informer, no cache: the daemon lists once per tick.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, ServerName};
use serde_json::{Value, json};

/// Where the kubelet mounts a pod's service account: `ca.crt`, `token` and `namespace`.
pub const SERVICE_ACCOUNT_DIR: &str = "/var/run/secrets/kubernetes.io/serviceaccount";

/// The variable that names the service account directory in place of [`SERVICE_ACCOUNT_DIR`].
pub const DIR_VARIABLE: &str = "CONDUCTOR_SERVICE_ACCOUNT_DIR";

/// The variable that names the API server's address in a pod.
pub const HOST_VARIABLE: &str = "KUBERNETES_SERVICE_HOST";

/// The variable that names the API server's port in a pod.
pub const PORT_VARIABLE: &str = "KUBERNETES_SERVICE_PORT";

/// How long a call may take, connect to last byte, unless the config says otherwise.
pub const DEADLINE: Duration = Duration::from_secs(10);

/// The files of the service account directory the client reads.
const FILES: [&str; 3] = ["ca.crt", "token", "namespace"];

/// The largest answer read; a longer one is refused.
const MAX_ANSWER: usize = 64 << 20;

/// Where the API server is and how the client reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The API server's address; in a pod, `KUBERNETES_SERVICE_HOST`.
    pub host: IpAddr,
    /// The API server's port; in a pod, `KUBERNETES_SERVICE_PORT`.
    pub port: u16,
    /// The service account directory: `ca.crt`, `token`, `namespace`.
    pub dir: PathBuf,
    /// How long one call may take.
    pub deadline: Duration,
}

impl Config {
    /// The config of the pod this process runs in: the process's variables and
    /// [`SERVICE_ACCOUNT_DIR`], or the directory [`DIR_VARIABLE`] names.
    ///
    /// # Errors
    ///
    /// As [`Config::from_variables`].
    pub fn in_cluster() -> Result<Self, Error> {
        Self::from_variables(|name| std::env::var(name).ok())
    }

    /// The config from the variables `var` answers: the service account directory is
    /// [`DIR_VARIABLE`]'s, else [`SERVICE_ACCOUNT_DIR`].
    ///
    /// # Errors
    ///
    /// [`Error::NotInCluster`] naming [`DIR_VARIABLE`] when it is set empty, and as
    /// [`Config::from_env`]; [`Error::Config`] when it names a relative path.
    pub fn from_variables(var: impl Fn(&str) -> Option<String>) -> Result<Self, Error> {
        let dir = match var(DIR_VARIABLE) {
            None => PathBuf::from(SERVICE_ACCOUNT_DIR),
            Some(value) if value.trim().is_empty() => {
                return Err(Error::NotInCluster {
                    missing: vec![format!("{DIR_VARIABLE} (set empty)")],
                });
            }
            Some(value) => {
                let dir = PathBuf::from(value);
                if !dir.is_absolute() {
                    return Err(Error::Config(format!(
                        "{DIR_VARIABLE} {:?} is no absolute path",
                        dir.display()
                    )));
                }
                dir
            }
        };
        Self::from_env(&dir, var)
    }

    /// The config from the service account directory `dir` and the variables `var` answers.
    ///
    /// # Errors
    ///
    /// As [`Config::in_cluster`].
    pub fn from_env(dir: &Path, var: impl Fn(&str) -> Option<String>) -> Result<Self, Error> {
        let host = var(HOST_VARIABLE).filter(|value| !value.trim().is_empty());
        let port = var(PORT_VARIABLE).filter(|value| !value.trim().is_empty());
        let mut missing = Vec::new();
        if host.is_none() {
            missing.push(HOST_VARIABLE.to_owned());
        }
        if port.is_none() {
            missing.push(PORT_VARIABLE.to_owned());
        }
        missing.extend(missing_files(dir));
        let (Some(host), Some(port), true) = (host, port, missing.is_empty()) else {
            return Err(Error::NotInCluster { missing });
        };
        let host = host
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(|_| Error::Config(format!("{HOST_VARIABLE} {host:?} is no IP address")))?;
        let port = port
            .trim()
            .parse()
            .map_err(|_| Error::Config(format!("{PORT_VARIABLE} {port:?} is no port")))?;
        Ok(Self {
            host,
            port,
            dir: dir.to_owned(),
            deadline: DEADLINE,
        })
    }
}

/// The files of [`FILES`] that are not in `dir`, by path.
fn missing_files(dir: &Path) -> Vec<String> {
    FILES
        .iter()
        .map(|file| dir.join(file))
        .filter(|path| !path.is_file())
        .map(|path| path.display().to_string())
        .collect()
}

/// A kind of object the daemon manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Pod,
    PersistentVolumeClaim,
    Secret,
}

impl Kind {
    /// The resource in the API's paths: `pods`, `persistentvolumeclaims`, `secrets`.
    #[must_use]
    pub fn resource(self) -> &'static str {
        match self {
            Self::Pod => "pods",
            Self::PersistentVolumeClaim => "persistentvolumeclaims",
            Self::Secret => "secrets",
        }
    }

    /// The object's `kind`: `Pod`, `PersistentVolumeClaim`, `Secret`.
    #[must_use]
    pub fn kind(self) -> &'static str {
        match self {
            Self::Pod => "Pod",
            Self::PersistentVolumeClaim => "PersistentVolumeClaim",
            Self::Secret => "Secret",
        }
    }
}

/// The API's `Status` of a call it refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The HTTP code of the answer.
    pub code: u16,
    /// The `reason` (`NotFound`, `AlreadyExists`, `Forbidden`, …); empty when it gave none.
    pub reason: String,
    /// The `message`; the answer's text when it was no `Status`.
    pub message: String,
}

/// Why a call or the client's construction failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not in a cluster: each variable and service account file that is missing.
    NotInCluster { missing: Vec<String> },
    /// A variable or a service account file is there but unusable.
    Config(String),
    /// The API server could not be reached, or the connection broke.
    Connect(String),
    /// The TLS handshake failed: the server's certificate is not the cluster authority's, or the
    /// server spoke no TLS.
    Tls(String),
    /// The call did not end within its deadline.
    Timeout(String),
    /// The API answered a code outside `2xx`.
    Api(Status),
    /// The answer could not be read: no HTTP, a broken chunk, no JSON where JSON was due.
    Answer(String),
}

impl Error {
    /// The API answered `404`.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Api(status) if status.code == 404)
    }

    /// The API answered `409`: the name is taken, or the object changed meanwhile.
    #[must_use]
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Api(status) if status.code == 409)
    }

    /// The API answered `403`: the caller's Role does not grant the call.
    #[must_use]
    pub fn is_forbidden(&self) -> bool {
        matches!(self, Self::Api(status) if status.code == 403)
    }

    /// What kind of failure this is, in a few words and with no message of the server: the
    /// status code of an answer outside `2xx`, else the kind of the failure.
    #[must_use]
    pub fn kind_of(&self) -> String {
        match self {
            Self::Api(status) => format!("status {}", status.code),
            Self::NotInCluster { .. } => "not in a cluster".to_owned(),
            Self::Config(_) => "no usable client config".to_owned(),
            Self::Connect(_) => "no connection to the API server".to_owned(),
            Self::Tls(_) => "a failed TLS handshake".to_owned(),
            Self::Timeout(_) => "no answer within the deadline".to_owned(),
            Self::Answer(_) => "an answer that does not read".to_owned(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInCluster { missing } => {
                write!(f, "not in a cluster: missing {}", missing.join(", "))
            }
            Self::Config(text) => write!(f, "the cluster's config: {text}"),
            Self::Connect(text) | Self::Tls(text) | Self::Timeout(text) | Self::Answer(text) => {
                f.write_str(text)
            }
            Self::Api(status) => write!(
                f,
                "the API answered {} {}: {}",
                status.code, status.reason, status.message
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A client of one API server; cheap to clone.
#[derive(Debug, Clone)]
pub struct Client {
    address: SocketAddr,
    authority: String,
    server_name: ServerName<'static>,
    tls: Arc<rustls::ClientConfig>,
    token: PathBuf,
    namespace: String,
    deadline: Duration,
}

impl Client {
    /// The client of the cluster this process runs in.
    ///
    /// # Errors
    ///
    /// As [`Config::in_cluster`] and [`Client::new`].
    pub fn in_cluster() -> Result<Self, Error> {
        Self::new(Config::in_cluster()?)
    }

    /// The client of `config`'s API server, with its `ca.crt` as the only root.
    ///
    /// # Errors
    ///
    /// [`Error::NotInCluster`] when a file of the directory is missing; [`Error::Config`] when
    /// `ca.crt` holds no certificate rustls takes or `namespace` does not read.
    pub fn new(config: Config) -> Result<Self, Error> {
        let missing = missing_files(&config.dir);
        if !missing.is_empty() {
            return Err(Error::NotInCluster { missing });
        }
        let ca = config.dir.join("ca.crt");
        let certificates = CertificateDer::pem_file_iter(&ca)
            .and_then(|certificates| certificates.collect::<Result<Vec<_>, _>>())
            .map_err(|error| Error::Config(format!("read {}: {error}", ca.display())))?;
        if certificates.is_empty() {
            return Err(Error::Config(format!(
                "{} holds no certificate",
                ca.display()
            )));
        }
        let mut roots = rustls::RootCertStore::empty();
        for certificate in certificates {
            roots
                .add(certificate)
                .map_err(|error| Error::Config(format!("{}: {error}", ca.display())))?;
        }
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|error| Error::Tls(format!("the TLS protocol versions: {error}")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let file = config.dir.join("namespace");
        let namespace = fs::read_to_string(&file)
            .map_err(|error| Error::Config(format!("read {}: {error}", file.display())))?
            .trim()
            .to_owned();
        let address = SocketAddr::new(config.host, config.port);
        Ok(Self {
            address,
            authority: address.to_string(),
            server_name: ServerName::IpAddress(config.host.into()),
            tls: Arc::new(tls),
            token: config.dir.join("token"),
            namespace,
            deadline: config.deadline,
        })
    }

    /// The namespace this process runs in, as its service account names it.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Creates `object` of `kind` in `namespace`; the object as the API stored it.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] with `409` when the name is taken; any other [`Error`] of a call.
    pub fn create(&self, namespace: &str, kind: Kind, object: &Value) -> Result<Value, Error> {
        self.json("POST", &collection(namespace, kind), Some(object))
    }

    /// The object `name` of `kind` in `namespace`.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] with `404` when there is none; any other [`Error`] of a call.
    pub fn get(&self, namespace: &str, kind: Kind, name: &str) -> Result<Value, Error> {
        self.json("GET", &item(namespace, kind, name), None)
    }

    /// Replaces the object `name` of `kind` in `namespace` with `object` (`PUT`, the `update`
    /// verb); the object as the API stored it.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] with `404` when there is none, `409` when `object`'s resource version is
    /// not the stored one; any other [`Error`] of a call.
    pub fn replace(
        &self,
        namespace: &str,
        kind: Kind,
        name: &str,
        object: &Value,
    ) -> Result<Value, Error> {
        self.json("PUT", &item(namespace, kind, name), Some(object))
    }

    /// The objects of `kind` in `namespace` whose labels match `selector`
    /// (`key=value,key2=value2`; empty selects all).
    ///
    /// # Errors
    ///
    /// Any [`Error`] of a call; [`Error::Answer`] when the answer holds no `items`.
    pub fn list(&self, namespace: &str, kind: Kind, selector: &str) -> Result<Vec<Value>, Error> {
        let mut path = collection(namespace, kind);
        if !selector.is_empty() {
            path.push_str("?labelSelector=");
            path.push_str(&encode(selector));
        }
        let list = self.json("GET", &path, None)?;
        match list {
            Value::Object(mut fields) => match fields.remove("items") {
                Some(Value::Array(items)) => Ok(items),
                Some(Value::Null) | None => Ok(Vec::new()),
                Some(_) => Err(Error::Answer(format!("GET {path}: its items are no list"))),
            },
            _ => Err(Error::Answer(format!("GET {path}: the answer is no list"))),
        }
    }

    /// Deletes the object `name` of `kind` in `namespace`.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] with `404` when there is none; any other [`Error`] of a call.
    pub fn delete(&self, namespace: &str, kind: Kind, name: &str) -> Result<(), Error> {
        self.call("DELETE", &item(namespace, kind, name), None)
            .map(drop)
    }

    /// Whether this process's service account may `verb` the `resource` in `namespace`, by a
    /// `SelfSubjectAccessReview`. A subresource follows a slash: `pods/log`.
    ///
    /// # Errors
    ///
    /// Any [`Error`] of a call; [`Error::Answer`] when the review holds no `status.allowed`.
    pub fn may(&self, namespace: &str, verb: &str, resource: &str) -> Result<bool, Error> {
        const PATH: &str = "/apis/authorization.k8s.io/v1/selfsubjectaccessreviews";
        let (resource, subresource) = resource.split_once('/').unwrap_or((resource, ""));
        let mut attributes = json!({"namespace": namespace, "verb": verb, "resource": resource});
        if !subresource.is_empty() {
            attributes["subresource"] = json!(subresource);
        }
        let review = json!({
            "apiVersion": "authorization.k8s.io/v1",
            "kind": "SelfSubjectAccessReview",
            "spec": {"resourceAttributes": attributes},
        });
        let answer = self.json("POST", PATH, Some(&review))?;
        answer["status"]["allowed"]
            .as_bool()
            .ok_or_else(|| Error::Answer(format!("POST {PATH}: no status.allowed")))
    }

    /// The last `lines` lines of the log of the pod `name` in `namespace`.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] with `404` when there is no such pod; any other [`Error`] of a call.
    pub fn log_tail(&self, namespace: &str, name: &str, lines: u32) -> Result<String, Error> {
        let path = format!("{}/log?tailLines={lines}", item(namespace, Kind::Pod, name));
        let body = self.call("GET", &path, None)?;
        Ok(String::from_utf8_lossy(&body).into_owned())
    }

    /// One call whose answer is JSON.
    fn json(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, Error> {
        let answer = self.call(method, path, body)?;
        serde_json::from_slice(&answer)
            .map_err(|error| Error::Answer(format!("{method} {path}: no JSON: {error}")))
    }

    /// One exchange: the request with the token as read now, the answer's body when its code is
    /// `2xx`, else its `Status` as [`Error::Api`].
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Vec<u8>, Error> {
        let what = format!("{method} {path}");
        let deadline = Instant::now() + self.deadline;
        let token = self.bearer()?;
        let body = body.map(Value::to_string);
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\n\
             Accept: application/json, */*\r\nUser-Agent: conductord\r\nConnection: close\r\n",
            self.authority
        );
        if let Some(body) = &body {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        request.push_str("\r\n");
        request.push_str(body.as_deref().unwrap_or_default());

        let left = deadline.saturating_duration_since(Instant::now());
        let tcp = TcpStream::connect_timeout(&self.address, left).map_err(|error| {
            match error.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => self.late(&what),
                _ => Error::Connect(format!("{what}: connect to {}: {error}", self.authority)),
            }
        })?;
        let connection =
            rustls::ClientConnection::new(Arc::clone(&self.tls), self.server_name.clone())
                .map_err(|error| Error::Tls(format!("{what}: {error}")))?;
        let mut tls = rustls::StreamOwned::new(connection, Timed { tcp, deadline });
        let sent = tls.write_all(request.as_bytes()).and_then(|()| tls.flush());
        if let Err(error) = sent {
            return Err(self.failure(&what, &error, tls.conn.is_handshaking()));
        }
        let (code, answer) = read_answer(&mut tls).map_err(|error| match error {
            Failed::Io(error) => self.failure(&what, &error, tls.conn.is_handshaking()),
            Failed::Answer(text) => Error::Answer(format!("{what}: {text}")),
        })?;
        if (200..300).contains(&code) {
            return Ok(answer);
        }
        Err(Error::Api(status(code, &answer)))
    }

    /// The token, read now.
    fn bearer(&self) -> Result<String, Error> {
        let token = fs::read_to_string(&self.token)
            .map_err(|error| Error::Config(format!("read {}: {error}", self.token.display())))?;
        let token = token.trim();
        if token.is_empty() || token.contains(|c: char| c.is_whitespace() || c.is_control()) {
            return Err(Error::Config(format!(
                "{} holds no token",
                self.token.display()
            )));
        }
        Ok(token.to_owned())
    }

    fn late(&self, what: &str) -> Error {
        Error::Timeout(format!(
            "{what}: {} did not answer within {:?}",
            self.authority, self.deadline
        ))
    }

    /// A failed read or write of the exchange as its [`Error`].
    fn failure(&self, what: &str, error: &io::Error, handshaking: bool) -> Error {
        if let Some(tls) = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<rustls::Error>())
        {
            return Error::Tls(format!("{what}: TLS with {}: {tls}", self.authority));
        }
        match error.kind() {
            ErrorKind::TimedOut | ErrorKind::WouldBlock => self.late(what),
            _ if handshaking => Error::Tls(format!("{what}: TLS with {}: {error}", self.authority)),
            _ => Error::Connect(format!("{what}: {}: {error}", self.authority)),
        }
    }
}

/// The `Status` of a refused call: the answer's `reason` and `message` when it is one, else its
/// text.
fn status(code: u16, answer: &[u8]) -> Status {
    let parsed: Value = serde_json::from_slice(answer).unwrap_or(Value::Null);
    let text = |field: &str| parsed[field].as_str().map(str::to_owned);
    Status {
        code,
        reason: text("reason").unwrap_or_default(),
        message: text("message").unwrap_or_else(|| {
            String::from_utf8_lossy(answer)
                .trim()
                .chars()
                .take(500)
                .collect()
        }),
    }
}

/// `/api/v1/namespaces/<namespace>/<resource>`.
fn collection(namespace: &str, kind: Kind) -> String {
    format!(
        "/api/v1/namespaces/{}/{}",
        encode(namespace),
        kind.resource()
    )
}

/// `/api/v1/namespaces/<namespace>/<resource>/<name>`.
fn item(namespace: &str, kind: Kind, name: &str) -> String {
    format!("{}/{}", collection(namespace, kind), encode(name))
}

/// `text` percent-encoded but for the unreserved characters, for a path segment or a query value.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The socket, every read and write of which waits at most until the call's deadline.
struct Timed {
    tcp: TcpStream,
    deadline: Instant,
}

impl Timed {
    fn left(&self) -> io::Result<Duration> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(ErrorKind::TimedOut, "the deadline passed"));
        }
        Ok(left)
    }
}

impl Read for Timed {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tcp.set_read_timeout(Some(self.left()?))?;
        self.tcp.read(buf)
    }
}

impl Write for Timed {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp.set_write_timeout(Some(self.left()?))?;
        self.tcp.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp.flush()
    }
}

/// Why an answer could not be read: the stream failed, or what came is no HTTP answer.
enum Failed {
    Io(io::Error),
    Answer(String),
}

impl From<io::Error> for Failed {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// How an answer's body ends.
enum Framing {
    Length(usize),
    Chunked,
    Close,
}

/// Reads one byte block more of `stream` into `bytes`; `false` at the end of the stream.
fn more<S: Read>(stream: &mut S, bytes: &mut Vec<u8>) -> Result<bool, Failed> {
    let mut buffer = [0u8; 16 * 1024];
    let read = match stream.read(&mut buffer) {
        Ok(read) => read,
        // A server that closes without TLS's close_notify: the answer ends here.
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => 0,
        Err(error) => return Err(error.into()),
    };
    if bytes.len() + read > MAX_ANSWER {
        return Err(Failed::Answer(format!(
            "the answer is longer than {MAX_ANSWER} bytes"
        )));
    }
    bytes.extend_from_slice(&buffer[..read]);
    Ok(read > 0)
}

/// The code and the body of the answer `stream` carries, the body read by its length, joined
/// from its chunks, or up to the end of the stream.
fn read_answer<S: Read>(stream: &mut S) -> Result<(u16, Vec<u8>), Failed> {
    let mut bytes = Vec::new();
    let end = loop {
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        if !more(stream, &mut bytes)? {
            return Err(Failed::Answer(
                "the answer ended before its head".to_owned(),
            ));
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let mut lines = head.split("\r\n");
    let code = lines
        .next()
        .and_then(|line| line.strip_prefix("HTTP/1."))
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| Failed::Answer("the answer has no HTTP status line".to_owned()))?;
    let mut framing = Framing::Close;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
        {
            framing = Framing::Chunked;
        } else if name.eq_ignore_ascii_case("content-length")
            && !matches!(framing, Framing::Chunked)
        {
            let length = value
                .parse()
                .map_err(|_| Failed::Answer(format!("Content-Length {value:?} is no length")))?;
            framing = Framing::Length(length);
        }
    }
    let mut body = bytes.split_off(end + 4);
    match framing {
        Framing::Length(length) => {
            while body.len() < length {
                if !more(stream, &mut body)? {
                    return Err(Failed::Answer(format!(
                        "the answer ended after {} of its {length} bytes",
                        body.len()
                    )));
                }
            }
            body.truncate(length);
            Ok((code, body))
        }
        Framing::Chunked => loop {
            // The last chunk ends in an empty line; only then is the body worth decoding.
            if body.ends_with(b"\r\n\r\n")
                && let Some(joined) = chunks(&body)?
            {
                return Ok((code, joined));
            }
            if !more(stream, &mut body)? {
                return Err(Failed::Answer(
                    "the answer ended inside its chunks".to_owned(),
                ));
            }
        },
        Framing::Close => {
            while more(stream, &mut body)? {}
            Ok((code, body))
        }
    }
}

/// The body of the chunks `bytes` holds, once it holds the last one; `None` before.
fn chunks(mut bytes: &[u8]) -> Result<Option<Vec<u8>>, Failed> {
    let mut joined = Vec::new();
    loop {
        let Some(line) = bytes.windows(2).position(|pair| pair == b"\r\n") else {
            return Ok(None);
        };
        let size = String::from_utf8_lossy(&bytes[..line]);
        let size = size.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size, 16)
            .map_err(|_| Failed::Answer(format!("the chunk size {size:?} is no number")))?;
        bytes = &bytes[line + 2..];
        if size == 0 {
            return Ok(Some(joined));
        }
        if bytes.len() < size + 2 {
            return Ok(None);
        }
        if &bytes[size..size + 2] != b"\r\n" {
            return Err(Failed::Answer(
                "a chunk does not end where its size says".to_owned(),
            ));
        }
        joined.extend_from_slice(&bytes[..size]);
        bytes = &bytes[size + 2..];
    }
}

/// The metadata of an object a supervisor reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Meta {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    /// `metadata.deletionTimestamp`: set once a delete was asked and the object is going.
    pub deletion_timestamp: Option<String>,
}

impl Meta {
    /// The metadata of `object`; what it lacks reads empty.
    #[must_use]
    pub fn of(object: &Value) -> Self {
        let metadata = &object["metadata"];
        Self {
            name: metadata["name"].as_str().unwrap_or_default().to_owned(),
            labels: metadata["labels"]
                .as_object()
                .map(|labels| {
                    labels
                        .iter()
                        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
                        .collect()
                })
                .unwrap_or_default(),
            deletion_timestamp: text(&metadata["deletionTimestamp"]),
        }
    }
}

/// What a supervisor reads of a pod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pod {
    pub meta: Meta,
    /// `status.phase`: `Pending`, `Running`, `Succeeded`, `Failed`, `Unknown`.
    pub phase: Option<String>,
    /// `status.podIP`.
    pub ip: Option<String>,
    /// `status.containerStatuses`, in their order.
    pub containers: Vec<Container>,
}

/// One container's state in a pod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub name: String,
    pub state: ContainerState,
}

/// A container's `state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerState {
    Waiting {
        reason: Option<String>,
        message: Option<String>,
    },
    Running,
    Terminated {
        exit_code: i32,
        reason: Option<String>,
        message: Option<String>,
    },
    /// No state, or one this client does not know.
    Unknown,
}

impl Pod {
    /// What `object`, a pod, says; what it lacks reads empty.
    #[must_use]
    pub fn of(object: &Value) -> Self {
        let status = &object["status"];
        Self {
            meta: Meta::of(object),
            phase: text(&status["phase"]),
            ip: text(&status["podIP"]),
            containers: status["containerStatuses"]
                .as_array()
                .map(|containers| {
                    containers
                        .iter()
                        .map(|container| Container {
                            name: container["name"].as_str().unwrap_or_default().to_owned(),
                            state: ContainerState::of(&container["state"]),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl ContainerState {
    fn of(state: &Value) -> Self {
        if let Some(waiting) = state.get("waiting") {
            Self::Waiting {
                reason: text(&waiting["reason"]),
                message: text(&waiting["message"]),
            }
        } else if state.get("running").is_some() {
            Self::Running
        } else if let Some(terminated) = state.get("terminated") {
            Self::Terminated {
                exit_code: terminated["exitCode"]
                    .as_i64()
                    .and_then(|code| i32::try_from(code).ok())
                    .unwrap_or_default(),
                reason: text(&terminated["reason"]),
                message: text(&terminated["message"]),
            }
        } else {
            Self::Unknown
        }
    }
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}
