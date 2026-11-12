//! Conformance checks of the safety contract of instance pods (decisions D2, D7, D8): what a
//! caller of the instance API can and cannot put into a pod, the Kubernetes client's handling of
//! its token and of the names it puts into a path, the rights the daemon asks for against the
//! rights it uses, and the runtime's refusal of a TCP listener without a token.
//!
//! Every case runs against the fake API server of `tests/fake_kube/` or the daemon's API in this
//! process; none touches a cluster.

mod active;
mod common;
mod fake_kube;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::daemon::api::{self, Api};
use conductor_cli::daemon::pods::{self, Pods};
use conductor_cli::daemon::reconcile::{Binary, Daemon, RUNTIMES_FILE};
use conductor_cli::daemon::units::Manager;
use conductor_cli::kube::{Error as KubeError, Kind};
use conductor_model::config::SecretsSource;
use conductor_model::instance::ReconcileReason;
use conductor_model::secret::{SecretName, Source};
use fake_kube::FakeKube;
use serde_json::{Value, json};

/// The instance namespace of every case.
const NAMESPACE: &str = "conductor-instances";

/// The control token: the value of the secret `control` of the case's secrets source, which
/// `daemon.control_token` names.
const CONTROL: &str = "control-token-0123456789abcdef0123456789abcdef";

/// The other values of the case's secrets source.
const MODEL_TOKEN: &str = "model-token-value-0000";
const FORGE_TOKEN: &str = "forge-token-value-1111";

/// A case's directory under this target's temporary directory.
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
            .join("sec-pods")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        fs::write(dir.join("secrets/control"), CONTROL).expect("write a secret");
        fs::write(dir.join("secrets/alpha-model"), MODEL_TOKEN).expect("write a secret");
        fs::write(dir.join("secrets/forge-token"), FORGE_TOKEN).expect("write a secret");
        Self { dir }
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

    fn secrets(&self) -> SecretsSource {
        SecretsSource {
            source: Source::Files,
            namespace: None,
            url: None,
            dir: Some(self.dir.join("secrets").display().to_string()),
        }
    }

    /// The daemon's file: instance pods, the control token `secret:control`, and `instances`
    /// (`" []"` for none, else entries starting with a newline).
    fn daemon_text(&self, instances: &str) -> String {
        format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: {store:?}\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 control_token: secret:control\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: registry.example.com/conductor:1.0.0\n\
             \x20   owner: example-org\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G}}\n\
             \x20   env: [{{name: GH_TOKEN, secret: secret:forge-token}}]\n\
             instances:{instances}\n",
            store = self.store(),
            secrets = self.dir.join("secrets"),
        )
    }

    fn write(&self, instances: &str) {
        fs::write(self.file(), self.daemon_text(instances)).expect("write the config file");
    }

    fn text(&self) -> String {
        fs::read_to_string(self.file()).expect("read the config file")
    }

    fn daemon(&self, fake: &FakeKube, dir: &str) -> Daemon<Manager> {
        Daemon::open(
            self.file(),
            self.home(),
            self.dir.join(dir),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Manager::Pods(Box::new(Pods::new(fake.client()))),
        )
        .expect("open the daemon")
    }

    /// The daemon's API over the case's file with the control token, on a loopback port.
    fn serve(&self) -> u16 {
        let api = Api::new(
            "4444444444444444444444444444444444444444444444444444444444444444".to_owned(),
            "operator".to_owned(),
            None,
        )
        .with_config(self.file(), self.home(), self.store())
        .with_control_token(self.secrets(), Some(SecretName("control".to_owned())))
        .with_pods(true);
        api.publish(Vec::new(), BTreeMap::new(), Vec::new());
        api.publish_host(Vec::new(), None);
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
        let port = listener.local_addr().expect("address").port();
        let api = Arc::new(api);
        thread::spawn(move || api::serve(&listener, &api));
        port
    }
}

/// The request's bytes, its body the JSON `body` where there is one.
fn request(port: u16, method: &str, target: &str, body: Option<&Value>) -> Vec<u8> {
    let mut text = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Authorization: Bearer {CONTROL}\r\n"
    );
    match body {
        Some(body) => {
            let body = body.to_string();
            text.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ));
        }
        None => text.push_str("\r\n"),
    }
    text.into_bytes()
}

/// Sends `bytes` to the listener on `port`, ends the sending side, and answers the status and the
/// body.
fn exchange(port: u16, bytes: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream.write_all(bytes).expect("send");
    let _ = stream.shutdown(Shutdown::Write);
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    let text = String::from_utf8_lossy(&answer).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, body.to_owned())
}

/// A whole instance mapping of `name` as a consumer sends it, with `extra` keys.
fn mapping(name: &str, extra: &Value) -> Value {
    let mut mapping = json!({
        "name": name,
        "placement": "none",
        "session_prefix": "beta",
        "sources": [{"github": "example-org"}],
    });
    if let (Some(map), Some(extra)) = (mapping.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            map.insert(key.clone(), value.clone());
        }
    }
    mapping
}

/// Decision D2 refuses the control plane every route that answers a secret value, and
/// `crate::secret::referenced` says the daemon's own secrets (`daemon.*`) are no instance's. A
/// mapping the control plane sends that names the daemon's control token as a secret of the
/// instance must therefore not carry that value into the instance's pod: it is refused, or the
/// pod's Secret does not hold it.
#[test]
fn the_daemons_control_token_reaches_no_instance_pod() {
    let case = Case::new("control-token-in-pod");
    case.write(" []");
    let fake = FakeKube::start(&case.dir);
    let port = case.serve();

    let sent = mapping("beta", &json!({"model_token": "secret:control"}));
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    if status != 201 {
        assert!(
            (400..500).contains(&status),
            "refused, but not as a refusal: {status} {body}"
        );
        assert!(!case.text().contains("name: beta"), "{}", case.text());
        return;
    }

    let mut daemon = case.daemon(&fake, "daemon");
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    let Some(secret) = fake.object(NAMESPACE, "secrets", "conductor-instance-beta") else {
        return;
    };
    let data = secret["data"].as_object().cloned().unwrap_or_default();
    for (key, value) in &data {
        let value = value
            .as_str()
            .and_then(pods::base64_decode)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        assert_ne!(
            value,
            CONTROL,
            "the Secret conductor-instance-beta holds the daemon's control token under the key \
             {key:?}, mounted at /run/secrets/conductor/{key} in a pod the control plane \
             configured; its keys: {:?}",
            data.keys().collect::<Vec<_>>()
        );
    }
}

/// Decision D8: nothing of a mapping but the instance's name reaches the pod or the claim. Two
/// mappings of one name that differ in every other key the instance pod takes make the same pod
/// and the same claim, but for the digest annotation of the rendered file.
#[test]
fn the_pod_and_the_claim_are_the_same_whatever_the_mapping_says_beside_the_name() {
    let case = Case::new("d8-property");
    let plain = "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
                 sources: [{github: example-org}]";
    let shaped = "\n  - name: beta\n    placement: none\n    session_prefix: other\n    \
                  sources: [{github: another-org}]\n    \
                  model_token: secret:alpha-model\n    \
                  checkouts: {root: /var/lib/conductor/data/k, trees: /var/lib/conductor/data/t}\n    \
                  records: /var/lib/conductor/data/r\n    \
                  state: /var/lib/conductor/data/s\n    \
                  cache: /var/lib/conductor/data/c";
    let made = |instances: &str| {
        let text = case.daemon_text(instances);
        let config = conductor_cli::config::parse(&text, &case.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let mut made = pods::manifests(&kubernetes, &config, &config.instances[0]);
        made.pod["metadata"]
            .as_object_mut()
            .map(|metadata| metadata.remove("annotations"));
        (made.pod, made.claim)
    };
    let (pod, claim) = made(plain);
    let (shaped_pod, shaped_claim) = made(shaped);
    assert_eq!(
        serde_json::to_string_pretty(&shaped_pod).unwrap_or_default(),
        serde_json::to_string_pretty(&pod).unwrap_or_default(),
        "a key of the mapping beside its name reached the pod"
    );
    assert_eq!(
        shaped_claim, claim,
        "a key of the mapping reached the claim"
    );
}

/// Decision D8 at the instance API: a name that is no DNS label of at most 39 characters, a path
/// outside the data mount, a key that would shape a pod, a YAML merge key or alias, and a body
/// past the bound are each refused before anything is written; a path name that is no instance
/// is unknown. The file is the same after every one.
#[test]
fn the_instance_routes_refuse_what_would_shape_a_pod() {
    let case = Case::new("d8-routes");
    case.write(" []");
    let before = case.text();
    let port = case.serve();
    let post = |body: &Value| exchange(port, &request(port, "POST", "/v1/instances", Some(body)));

    let mut refusals: Vec<(String, Value)> = Vec::new();
    for name in [
        "Beta",
        "be/ta",
        "..",
        "../beta",
        "beta/../../secrets",
        "beta.gamma",
        "beta_gamma",
        "beta?labelSelector=x",
        "-beta",
        "beta-",
        &"a".repeat(40),
    ] {
        refusals.push((
            format!("name {name:?}"),
            json!({"mapping": mapping(name, &json!({})), "reason": "r"}),
        ));
    }
    for (what, extra) in [
        ("state outside", json!({"state": "/etc/conductor"})),
        (
            "records up",
            json!({"records": "/var/lib/conductor/data/../../../etc"}),
        ),
        (
            "cache beside",
            json!({"cache": "/var/lib/conductor/database"}),
        ),
        (
            "checkouts root",
            json!({"checkouts": {"root": "/var/lib/conductor", "trees": "/var/lib/conductor/data/t"}}),
        ),
        ("secret mount", json!({"state": "/run/secrets/conductor"})),
        ("image", json!({"image": "registry.example.com/other:1"})),
        ("command", json!({"command": ["sh", "-c", "true"]})),
        (
            "env",
            json!({"env": [{"name": "LD_PRELOAD", "value": "/x.so"}]}),
        ),
        ("env-shaped key", json!({"CONDUCTOR_API_TOKEN": "x"})),
        ("namespace", json!({"namespace": "kube-system"})),
        ("service account", json!({"serviceAccountName": "default"})),
        (
            "security context",
            json!({"securityContext": {"privileged": true}}),
        ),
        (
            "volumes",
            json!({"volumes": [{"name": "h", "hostPath": {"path": "/"}}]}),
        ),
        ("labels", json!({"labels": {"conductor/daemon": "another"}})),
        (
            "annotations",
            json!({"annotations": {"conductor/digest": "x"}}),
        ),
        ("placement box", json!({"placement": "systemd"})),
    ] {
        refusals.push((
            what.to_owned(),
            json!({"mapping": mapping("beta", &extra), "reason": "r"}),
        ));
    }
    let yaml_base =
        "name: beta\nplacement: none\nsession_prefix: beta\nsources: [{github: example-org}]\n";
    for (what, text) in [
        (
            "merge key",
            format!("{yaml_base}<<: {{image: registry.example.com/other:1}}\n"),
        ),
        (
            "merge of an anchor",
            format!(
                "{yaml_base}session_prefix_copy: &shape {{state: /etc/conductor}}\n<<: *shape\n"
            ),
        ),
        (
            "alias into a path",
            "name: beta\nplacement: none\nsession_prefix: &outside /etc/conductor\n\
             sources: [{github: example-org}]\nstate: *outside\n"
                .to_owned(),
        ),
        (
            "many aliases",
            format!(
                "{yaml_base}repositories: &a [x, x, x, x, x, x, x, x, x]\n\
                 roles: [*a, *a, *a, *a, *a, *a, *a, *a, *a]\n"
            ),
        ),
    ] {
        refusals.push((what.to_owned(), json!({"mapping": text, "reason": "r"})));
    }
    for (what, body) in &refusals {
        let (status, answer) = post(body);
        assert!(
            (400..500).contains(&status) && status != 401 && status != 403,
            "{what}: answered {status}, not refused as invalid: {answer}\n{}",
            case.text()
        );
        assert_eq!(case.text(), before, "{what}: the file changed");
    }

    // A body announced at 10 MiB is refused by its length before it is read.
    let head = format!(
        "POST /v1/instances HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Authorization: Bearer {CONTROL}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{{\"mapping\": ",
        10 * 1024 * 1024
    );
    let (status, answer) = exchange(port, head.as_bytes());
    assert_eq!(status, 413, "a 10 MiB body: {answer}");
    assert_eq!(case.text(), before, "a 10 MiB body: the file changed");

    // A path name that is no instance is unknown, whatever it says.
    for target in [
        "/v1/instances/..",
        "/v1/instances/%2e%2e",
        "/v1/instances/conductor-instance-beta",
        "/v1/instances/beta%2F..%2F..",
    ] {
        for method in ["PUT", "DELETE"] {
            let body = (method == "PUT")
                .then(|| json!({"mapping": mapping("beta", &json!({})), "reason": "r"}));
            let (status, answer) = exchange(port, &request(port, method, target, body.as_ref()));
            assert!(
                (400..500).contains(&status) && status != 401 && status != 403,
                "{method} {target}: {status} {answer}"
            );
            assert_eq!(case.text(), before, "{method} {target}: the file changed");
        }
    }
}

/// The client follows no redirect: a `3xx` is the call's error, after one request, and the token
/// is in the text of no error: of a redirect, of a refusal, or of a token file that holds no
/// token.
#[test]
fn the_kube_client_follows_no_redirect_and_says_no_token() {
    let case = Case::new("kube-redirect");
    let fake = FakeKube::start(&case.dir);
    let client = fake.client();

    for (code, reason) in [
        (301, "MovedPermanently"),
        (302, "Found"),
        (307, "TemporaryRedirect"),
        (308, "PermanentRedirect"),
    ] {
        let before = fake.requests().len();
        fake.fail_next(code, reason, "see https://elsewhere.example.com/api");
        let error = client
            .get(NAMESPACE, Kind::Pod, "conductor-instance-beta")
            .expect_err("a redirect is no answer");
        assert!(
            matches!(&error, KubeError::Api(status) if status.code == code),
            "{code}: {error:?}"
        );
        assert_eq!(
            fake.requests().len(),
            before + 1,
            "{code}: one request only"
        );
        let said = format!("{error} {error:?}");
        assert!(!said.contains(fake_kube::TOKEN), "{code}: {said}");
    }

    fake.fail_next(401, "Unauthorized", "Unauthorized");
    let error = client
        .list(NAMESPACE, Kind::Secret, "a=b")
        .expect_err("a refusal");
    let said = format!("{error} {error:?}");
    assert!(!said.contains(fake_kube::TOKEN), "{said}");

    let unusable = "a leaked value with spaces";
    fs::write(fake.dir().join("token"), format!("{unusable}\n")).expect("write the token");
    let error = client.get(NAMESPACE, Kind::Pod, "x").expect_err("no token");
    let said = format!("{error} {error:?}");
    assert!(!said.contains(unusable), "{said}");
}

/// The names and the selector a call is given reach the API inside their own path segment and
/// their own query value: a `?`, `&` or `#` in them starts no query, parameter or fragment.
#[test]
fn names_and_selectors_stay_inside_their_segment_of_the_path() {
    let case = Case::new("kube-encoding");
    let fake = FakeKube::start(&case.dir);
    let client = fake.client();
    let last = || fake.requests().last().cloned().expect("a request");

    let _ = client.get(NAMESPACE, Kind::Pod, "x?labelSelector=y");
    let seen = last();
    assert_eq!(
        seen.path,
        format!("/api/v1/namespaces/{NAMESPACE}/pods/x?labelSelector=y")
    );
    assert!(seen.query.is_empty(), "{seen:?}");

    let _ = client.delete(NAMESPACE, Kind::Secret, "x&dryRun=All#f");
    let seen = last();
    assert_eq!(
        seen.path,
        format!("/api/v1/namespaces/{NAMESPACE}/secrets/x&dryRun=All#f")
    );
    assert!(seen.query.is_empty(), "{seen:?}");

    let _ = client.get("ns?x=y", Kind::Secret, "a");
    let seen = last();
    assert_eq!(seen.path, "/api/v1/namespaces/ns?x=y/secrets/a");
    assert!(seen.query.is_empty(), "{seen:?}");

    let selector = format!(
        "{}&fieldSelector=metadata.name=x",
        pods::selector("example-org")
    );
    let _ = client.list(NAMESPACE, Kind::Secret, &selector);
    let seen = last();
    assert_eq!(seen.query.len(), 1, "{seen:?}");
    assert_eq!(seen.query.get("labelSelector"), Some(&selector), "{seen:?}");
}

/// The rights `conductor doctor` asks a daemon of instance pods for (`doctor::POD_RIGHTS`, which
/// the Role of a run manifest grants) are rights some path of the supervisor calls: a start, a
/// failed pod's log, an adoption, a removal and a sweep together use every one of them.
#[test]
fn every_right_the_doctor_asks_for_is_one_the_supervisor_uses() {
    let case = Case::new("rights");
    let alpha = "\n  - name: alpha\n    placement: none\n    session_prefix: alpha\n    \
                 sources: [{github: example-org}]\n    model_token: secret:alpha-model";
    case.write(alpha);
    let fake = FakeKube::start(&case.dir);
    let start = Instant::now();
    let pod = |status: Value| {
        let mut pod = fake
            .object(NAMESPACE, "pods", "conductor-instance-alpha")
            .expect("the pod");
        pod["status"] = status;
        fake.put(NAMESPACE, "pods", pod);
    };

    // A start, then a failed pod with no message: its log's last lines.
    let mut daemon = case.daemon(&fake, "daemon");
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    pod(json!({"phase": "Failed"}));
    fake.set_log(
        NAMESPACE,
        "conductor-instance-alpha",
        "starting\nit ended\n",
    );
    daemon.tick(start + Duration::from_secs(2)).expect("a tick");
    daemon
        .tick(start + Duration::from_secs(13))
        .expect("a tick");
    drop(daemon);

    // An adoption by a daemon that kept nothing: the token read back from the Secret.
    pod(json!({
        "phase": "Running",
        "podIP": "127.0.0.1",
        "containerStatuses": [
            {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
        ],
    }));
    let _ = fs::remove_file(case.dir.join("daemon").join(RUNTIMES_FILE));
    let mut daemon = case.daemon(&fake, "daemon");
    daemon.tick(Instant::now()).expect("a tick");

    // A Secret of the owner whose instance is gone, then the instance removed: the sweep and the
    // stop.
    let mut stray = fake
        .object(NAMESPACE, "secrets", "conductor-instance-alpha")
        .expect("the Secret");
    stray["metadata"]["name"] = json!("conductor-instance-gone");
    stray["metadata"]["labels"]["conductor/instance"] = json!("gone");
    fake.put(NAMESPACE, "secrets", stray);
    case.write(" []");
    daemon
        .tick(Instant::now() + Duration::from_secs(30))
        .expect("a tick");

    let mut used: BTreeSet<(String, String)> = BTreeSet::new();
    for request in fake.requests() {
        let segments: Vec<&str> = request.path.trim_start_matches('/').split('/').collect();
        let right = match (request.method.as_str(), segments.as_slice()) {
            ("GET", ["api", "v1", "namespaces", _, "pods", _, "log"]) => ("get", "pods/log"),
            ("POST", ["api", "v1", "namespaces", _, resource]) => ("create", *resource),
            ("GET", ["api", "v1", "namespaces", _, resource]) => ("list", *resource),
            ("GET", ["api", "v1", "namespaces", _, resource, _]) => ("get", *resource),
            ("DELETE", ["api", "v1", "namespaces", _, resource, _]) => ("delete", *resource),
            _ => continue,
        };
        used.insert((right.0.to_owned(), right.1.to_owned()));
    }
    let unused: Vec<String> = conductor_cli::doctor::POD_RIGHTS
        .iter()
        .filter(|(verb, resource)| !used.contains(&((*verb).to_owned(), (*resource).to_owned())))
        .map(|(verb, resource)| format!("{verb} {resource}"))
        .collect();
    assert!(
        unused.is_empty(),
        "the doctor asks for rights no path of the supervisor calls: {unused:?}; used: {used:?}"
    );
}

/// A runtime asked for a TCP listener with a token that is blank refuses to start, as one with
/// none does: a blank token would admit a blank header.
#[test]
fn a_blank_token_with_a_tcp_listener_stops_the_start() {
    let case = Case::new("blank-token");
    let root = case.dir.join("i");
    fs::create_dir_all(root.join("state")).expect("create the state directory");
    let file = case.dir.join("runtime.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   placement: local\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = format!("sqlite:{}", case.dir.join("runtime.sqlite").display()),
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        ),
    )
    .expect("write the config file");
    for blank in ["", " ", "\t\n"] {
        let mut child = common::conductor(case.home())
            .arg("--config")
            .arg(&file)
            .args(["instance", "serve"])
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
            .env(conductor_cli::instance::TOKEN_VARIABLE, blank)
            .env(conductor_cli::instance::LISTEN_VARIABLE, "127.0.0.1:0")
            .env_remove(conductor_cli::instance::CONFIG_SOURCE_VARIABLE)
            .env_remove(conductor_cli::instance::SLOTS_VARIABLE)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start instance serve");
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Ok(Some(status)) = child.try_wait() {
                break Some(status);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            thread::sleep(Duration::from_millis(50));
        };
        let mut printed = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_string(&mut printed);
        }
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut printed);
        }
        assert_eq!(
            status.and_then(|status| status.code()),
            Some(1),
            "{blank:?}: the runtime started on a blank token: {printed}"
        );
        assert!(
            printed.contains(conductor_cli::instance::TOKEN_VARIABLE)
                && printed.contains(conductor_cli::instance::LISTEN_VARIABLE),
            "{blank:?}: it ended for another reason than its token: {printed}"
        );
    }
}
