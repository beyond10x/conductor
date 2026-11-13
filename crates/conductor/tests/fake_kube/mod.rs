//! A fake Kubernetes API server for the cases of the client (`conductor_cli::kube`) and of what
//! drives it: [`FakeKube`].
//!
//! It serves TLS on a loopback port under a certificate authority made for it, and writes a
//! service account directory (`ca.crt`, `token`, `namespace`) the client reads, as the kubelet
//! mounts one in a pod. It keeps the objects it is given in memory, by namespace, resource and
//! name, and answers what a case needs of the API:
//!
//! | request | answer |
//! |---|---|
//! | `POST /api/v1/namespaces/{ns}/{resource}` | `201` and the object; `409 AlreadyExists` when the name is taken |
//! | `GET /api/v1/namespaces/{ns}/{resource}?labelSelector=…` | `200` and a list of the objects whose labels match |
//! | `GET /api/v1/namespaces/{ns}/{resource}/{name}` | `200` and the object; `404 NotFound` |
//! | `PUT /api/v1/namespaces/{ns}/{resource}/{name}` | `200` and the object, which replaces the one held; `404 NotFound`; `400` when the body names another object |
//! | `DELETE /api/v1/namespaces/{ns}/{resource}/{name}` | `200` and the object, which is gone; `404 NotFound` |
//! | `GET /api/v1/namespaces/{ns}/pods/{name}/log?tailLines=n` | `200`, the last `n` lines [`FakeKube::set_log`] gave |
//! | `POST /apis/authorization.k8s.io/v1/selfsubjectaccessreviews` | the review, `status.allowed` as [`FakeKube::allow`] said |
//!
//! A bearer other than the token of the directory answers `401 Unauthorized`; [`FakeKube::fail_next`]
//! makes the next request answer a `Status` of the case's choosing. Every answer is a `Status`
//! object where the API answers one, with a `Content-Length`, or in chunks after
//! [`FakeKube::chunked`]. [`FakeKube::requests`] lists every request as it was received.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use conductor_cli::kube::{Client, Config};
use rcgen::{BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};

/// The namespace the fake's service account directory names: the daemon's own.
pub const NAMESPACE: &str = "conductor-system";

/// The token the fake's service account directory holds at start.
pub const TOKEN: &str = "first-token";

/// One request as the fake received it.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// The path, percent-decoded, without the query.
    pub path: String,
    /// The query's parameters, percent-decoded.
    pub query: BTreeMap<String, String>,
    /// The bearer of the `Authorization` header, if any.
    pub bearer: Option<String>,
    /// The body read as JSON, if it was any.
    pub body: Option<Value>,
}

#[derive(Default)]
struct State {
    token: String,
    objects: BTreeMap<(String, String, String), Value>,
    requests: Vec<Request>,
    chunked: bool,
    stall: Option<Duration>,
    allowed: Vec<(String, String, String)>,
    logs: BTreeMap<(String, String), String>,
    fail: Option<(u16, String, String)>,
    serial: u64,
}

/// A fake API server on loopback with its service account directory; see the module.
pub struct FakeKube {
    port: u16,
    dir: PathBuf,
    state: Arc<Mutex<State>>,
}

impl FakeKube {
    /// Starts a fake under a certificate authority of its own, and writes its service account
    /// directory to `<dir>/serviceaccount`.
    pub fn start(dir: &Path) -> Self {
        let authority = Authority::new("fake cluster authority");
        let fake = Self::serve(dir, &authority);
        fs::write(fake.dir.join("ca.crt"), authority.cert.pem()).expect("write ca.crt");
        fake
    }

    /// Starts a fake whose certificate another authority signed, and writes into
    /// `<dir>/serviceaccount` the `ca.crt` of `trusted`, a fresh authority: a client from that
    /// directory reaches a server it must refuse.
    pub fn start_untrusted(dir: &Path) -> Self {
        let served = Authority::new("another authority");
        let trusted = Authority::new("fake cluster authority");
        let fake = Self::serve(dir, &served);
        fs::write(fake.dir.join("ca.crt"), trusted.cert.pem()).expect("write ca.crt");
        fake
    }

    fn serve(dir: &Path, authority: &Authority) -> Self {
        let dir = dir.join("serviceaccount");
        fs::create_dir_all(&dir).expect("create the service account directory");
        fs::write(dir.join("token"), format!("{TOKEN}\n")).expect("write token");
        fs::write(dir.join("namespace"), NAMESPACE).expect("write namespace");
        let config = authority.server_config();
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the fake");
        let port = listener.local_addr().expect("the fake's address").port();
        let state = Arc::new(Mutex::new(State {
            token: TOKEN.to_owned(),
            ..State::default()
        }));
        let shared = Arc::clone(&state);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (config, state) = (Arc::clone(&config), Arc::clone(&shared));
                thread::spawn(move || {
                    let _ = connection(stream, config, &state);
                });
            }
        });
        Self { port, dir, state }
    }

    /// The fake's port on `127.0.0.1`.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The service account directory the fake wrote.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The client's config for this fake: `127.0.0.1`, its port, its directory, a 5 s deadline.
    pub fn config(&self) -> Config {
        Config {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: self.port,
            dir: self.dir.clone(),
            deadline: Duration::from_secs(5),
        }
    }

    /// A client of this fake.
    pub fn client(&self) -> Client {
        Client::new(self.config()).expect("a client of the fake")
    }

    /// Rotates the token: the directory's file holds `token`, and only `token` is admitted.
    pub fn rotate(&self, token: &str) {
        fs::write(self.dir.join("token"), format!("{token}\n")).expect("write token");
        self.lock().token = token.to_owned();
    }

    /// Every request received so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.lock().requests.clone()
    }

    /// Answers in chunks (`Transfer-Encoding: chunked`) from now on, or with a length again.
    pub fn chunked(&self, on: bool) {
        self.lock().chunked = on;
    }

    /// Waits `delay` before every answer from now on.
    pub fn stall(&self, delay: Duration) {
        self.lock().stall = Some(delay);
    }

    /// The access review allows `verb` on `resource` (`pods`, `pods/log`, …) in `namespace`.
    pub fn allow(&self, namespace: &str, verb: &str, resource: &str) {
        self.lock()
            .allowed
            .push((namespace.to_owned(), verb.to_owned(), resource.to_owned()));
    }

    /// The log the pod `name` of `namespace` has written.
    pub fn set_log(&self, namespace: &str, name: &str, text: &str) {
        self.lock()
            .logs
            .insert((namespace.to_owned(), name.to_owned()), text.to_owned());
    }

    /// The next request answers a `Status` of `code`, `reason` and `message`, whatever it asks.
    pub fn fail_next(&self, code: u16, reason: &str, message: &str) {
        self.lock().fail = Some((code, reason.to_owned(), message.to_owned()));
    }

    /// Puts `object` into `resource` (`pods`, `secrets`, `persistentvolumeclaims`) of
    /// `namespace` as it is, replacing one of its name: a case sets a pod's status this way.
    pub fn put(&self, namespace: &str, resource: &str, object: Value) {
        let name = object["metadata"]["name"]
            .as_str()
            .expect("the object has a name")
            .to_owned();
        self.lock()
            .objects
            .insert((namespace.to_owned(), resource.to_owned(), name), object);
    }

    /// The object `name` of `resource` in `namespace`, if the fake holds it.
    pub fn object(&self, namespace: &str, resource: &str, name: &str) -> Option<Value> {
        self.lock()
            .objects
            .get(&(namespace.to_owned(), resource.to_owned(), name.to_owned()))
            .cloned()
    }

    /// The names of the objects of `resource` in `namespace`, sorted.
    pub fn names(&self, namespace: &str, resource: &str) -> Vec<String> {
        self.lock()
            .objects
            .keys()
            .filter(|(ns, kind, _)| ns == namespace && kind == resource)
            .map(|(_, _, name)| name.clone())
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

struct Authority {
    cert: rcgen::Certificate,
    key: KeyPair,
}

impl Authority {
    fn new(name: &str) -> Self {
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
        params.distinguished_name.push(DnType::CommonName, name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let key = KeyPair::generate().expect("a CA key");
        let cert = params.self_signed(&key).expect("a CA certificate");
        Self { cert, key }
    }

    /// A server config whose certificate, for `127.0.0.1` and `localhost`, this authority signed.
    fn server_config(&self) -> Arc<rustls::ServerConfig> {
        let mut params =
            CertificateParams::new(vec!["127.0.0.1".to_owned(), "localhost".to_owned()])
                .expect("server parameters");
        params
            .distinguished_name
            .push(DnType::CommonName, "kube-apiserver");
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("a server key");
        let cert = params
            .signed_by(&key, &self.cert, &self.key)
            .expect("a server certificate");
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("TLS versions")
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], key)
        .expect("the fake's TLS config");
        Arc::new(config)
    }
}

/// An answer: its code, content type and body.
struct Answer(u16, &'static str, Vec<u8>);

fn json_answer(code: u16, value: &Value) -> Answer {
    Answer(code, "application/json", value.to_string().into_bytes())
}

fn status(code: u16, reason: &str, message: &str) -> Answer {
    json_answer(
        code,
        &json!({
            "kind": "Status",
            "apiVersion": "v1",
            "metadata": {},
            "status": "Failure",
            "message": message,
            "reason": reason,
            "code": code,
        }),
    )
}

fn connection(
    stream: TcpStream,
    config: Arc<rustls::ServerConfig>,
    state: &Mutex<State>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let server = rustls::ServerConnection::new(config).map_err(std::io::Error::other)?;
    let mut tls = rustls::StreamOwned::new(server, stream);
    let Some(request) = read_request(&mut tls)? else {
        return Ok(());
    };
    let (answer, chunked, stall) = {
        let mut state = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.requests.push(request.clone());
        let answer = answer(&mut state, &request);
        (answer, state.chunked, state.stall)
    };
    if let Some(delay) = stall {
        thread::sleep(delay);
    }
    write_answer(&mut tls, &answer, chunked)?;
    tls.conn.send_close_notify();
    tls.flush()
}

fn read_request<S: Read>(stream: &mut S) -> std::io::Result<Option<Request>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let end = loop {
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break end;
        }
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(None),
            Ok(n) => bytes.extend_from_slice(&buffer[..n]),
            Err(error) if error.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(error) => return Err(error),
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split(' ');
    let method = first.next().unwrap_or_default().to_owned();
    let target = first.next().unwrap_or_default().to_owned();
    let mut length = 0;
    let mut bearer = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse().unwrap_or(0),
            "authorization" => bearer = value.strip_prefix("Bearer ").map(str::to_owned),
            _ => {}
        }
    }
    let mut body = bytes[end + 4..].to_vec();
    while body.len() < length {
        match stream.read(&mut buffer)? {
            0 => break,
            n => body.extend_from_slice(&buffer[..n]),
        }
    }
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let query = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect();
    Ok(Some(Request {
        method,
        path: decode(path),
        query,
        bearer,
        body: serde_json::from_slice(&body).ok(),
    }))
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && let Some(byte) = text
                .get(at + 1..at + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            at += 3;
            continue;
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn write_answer<S: Write>(stream: &mut S, answer: &Answer, chunked: bool) -> std::io::Result<()> {
    let Answer(code, content_type, body) = answer;
    let mut out = format!("HTTP/1.1 {code} Fake\r\nContent-Type: {content_type}\r\n").into_bytes();
    if chunked {
        out.extend_from_slice(b"Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
        for chunk in body.chunks(7) {
            out.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            out.extend_from_slice(chunk);
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"0\r\n\r\n");
    } else {
        out.extend_from_slice(
            format!(
                "Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(body);
    }
    stream.write_all(&out)
}

/// The labels of `object` match every `key=value` (or `key==value`, `key!=value`, `key`) term of
/// `selector`.
fn selects(selector: &str, object: &Value) -> bool {
    let labels = &object["metadata"]["labels"];
    selector
        .split(',')
        .filter(|term| !term.is_empty())
        .all(|term| {
            if let Some((key, value)) = term.split_once("!=") {
                labels[key].as_str() != Some(value)
            } else if let Some((key, value)) = term.split_once('=') {
                labels[key].as_str() == Some(value.trim_start_matches('='))
            } else {
                labels.get(term).is_some()
            }
        })
}

fn answer(state: &mut State, request: &Request) -> Answer {
    if let Some((code, reason, message)) = state.fail.take() {
        return status(code, &reason, &message);
    }
    if request.bearer.as_deref() != Some(state.token.as_str()) {
        return status(401, "Unauthorized", "Unauthorized");
    }
    let segments: Vec<&str> = request.path.trim_start_matches('/').split('/').collect();
    match (request.method.as_str(), segments.as_slice()) {
        (
            "POST",
            [
                "apis",
                "authorization.k8s.io",
                "v1",
                "selfsubjectaccessreviews",
            ],
        ) => {
            let mut review = request.body.clone().unwrap_or(Value::Null);
            let attributes = &review["spec"]["resourceAttributes"];
            let field = |name: &str| attributes[name].as_str().unwrap_or_default().to_owned();
            let resource = match field("subresource").as_str() {
                "" => field("resource"),
                sub => format!("{}/{sub}", field("resource")),
            };
            let allowed = state
                .allowed
                .contains(&(field("namespace"), field("verb"), resource));
            review["status"] = json!({"allowed": allowed});
            json_answer(201, &review)
        }
        ("GET", ["api", "v1", "namespaces", namespace, "pods", name, "log"]) => {
            let Some(log) = state
                .logs
                .get(&((*namespace).to_owned(), (*name).to_owned()))
            else {
                return status(404, "NotFound", &format!("pods \"{name}\" not found"));
            };
            let lines: Vec<&str> = log.lines().collect();
            let tail = request
                .query
                .get("tailLines")
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(lines.len())
                .min(lines.len());
            let mut text = lines[lines.len() - tail..].join("\n");
            text.push('\n');
            Answer(200, "text/plain", text.into_bytes())
        }
        ("POST", ["api", "v1", "namespaces", namespace, resource]) => {
            let mut object = request.body.clone().unwrap_or(Value::Null);
            let Some(name) = object["metadata"]["name"].as_str().map(str::to_owned) else {
                return status(422, "Invalid", "metadata.name: Required value");
            };
            let key = (
                (*namespace).to_owned(),
                (*resource).to_owned(),
                name.clone(),
            );
            if state.objects.contains_key(&key) {
                return status(
                    409,
                    "AlreadyExists",
                    &format!("{resource} \"{name}\" already exists"),
                );
            }
            state.serial += 1;
            object["metadata"]["namespace"] = json!(namespace);
            object["metadata"]["uid"] = json!(format!("uid-{}", state.serial));
            if *resource == "pods" && object["status"]["phase"].is_null() {
                object["status"] = json!({"phase": "Pending"});
            }
            state.objects.insert(key, object.clone());
            json_answer(201, &object)
        }
        ("GET", ["api", "v1", "namespaces", namespace, resource]) => {
            let selector = request
                .query
                .get("labelSelector")
                .cloned()
                .unwrap_or_default();
            let items: Vec<Value> = state
                .objects
                .iter()
                .filter(|((ns, kind, _), object)| {
                    ns == namespace && kind == resource && selects(&selector, object)
                })
                .map(|(_, object)| object.clone())
                .collect();
            json_answer(
                200,
                &json!({"kind": "List", "apiVersion": "v1", "metadata": {}, "items": items}),
            )
        }
        ("PUT", ["api", "v1", "namespaces", namespace, resource, name]) => {
            let mut object = request.body.clone().unwrap_or(Value::Null);
            if object["metadata"]["name"].as_str() != Some(*name) {
                return status(
                    400,
                    "BadRequest",
                    "the name of the object does not match the name on the URL",
                );
            }
            let key = (
                (*namespace).to_owned(),
                (*resource).to_owned(),
                (*name).to_owned(),
            );
            let Some(held) = state.objects.get(&key) else {
                return status(404, "NotFound", &format!("{resource} \"{name}\" not found"));
            };
            object["metadata"]["namespace"] = json!(namespace);
            object["metadata"]["uid"] = held["metadata"]["uid"].clone();
            state.objects.insert(key, object.clone());
            json_answer(200, &object)
        }
        (method @ ("GET" | "DELETE"), ["api", "v1", "namespaces", namespace, resource, name]) => {
            let key = (
                (*namespace).to_owned(),
                (*resource).to_owned(),
                (*name).to_owned(),
            );
            let found = if method == "GET" {
                state.objects.get(&key).cloned()
            } else {
                state.objects.remove(&key)
            };
            match found {
                Some(object) => json_answer(200, &object),
                None => status(404, "NotFound", &format!("{resource} \"{name}\" not found")),
            }
        }
        _ => status(
            404,
            "NotFound",
            "the server could not find the requested resource",
        ),
    }
}
