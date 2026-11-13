//! The pod supervisor (design: instance pods, decisions D1, D7, D8): where `daemon.kubernetes` is
//! set, conductord runs each instance's runtime as a pod of its own in the instance namespace,
//! with a Secret and a volume claim beside it, all three named after the instance and carrying the
//! one label set; it lists its pods once per tick, reads a Pending pod as `Starting`, starts a
//! Failed one again under the failed-start back-off with the pod's message as the detail, adopts
//! its pods after a restart of its own, restarts a pod whose image or rendering changed, deletes
//! the pod and the Secret of an instance it no longer runs and keeps its claim, and reaches a
//! runtime over TCP at its pod's address.
//!
//! Every case runs the library's reconciler against the fake API server of `tests/fake_kube/`;
//! none touches a cluster. The one runtime a case starts is the built binary's `instance serve`
//! on a loopback port, as a pod's would serve on its address.

mod active;
mod common;
mod fake_kube;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use conductor_cli::daemon::api::{self, Api};
use conductor_cli::daemon::pods::{self, Pods};
use conductor_cli::daemon::reconcile::{Binary, Daemon, RUNTIMES_FILE};
use conductor_cli::daemon::units::Manager;
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use fake_kube::FakeKube;
use serde_json::{Value, json};

/// The instance namespace of every case.
const NAMESPACE: &str = "conductor-instances";

/// The owner, `daemon.kubernetes.owner`, of every case's daemon.
const OWNER: &str = "example-org";

/// The image of every case's first start.
const IMAGE: &str = "registry.example.com/conductor:1.0.0";

/// The values the case's secrets source holds.
const MODEL_TOKEN: &str = "model-token-value-0000";
const FORGE_TOKEN: &str = "forge-token-value-1111";

/// One case: its directory, its fake API server, its config file.
struct Case {
    dir: PathBuf,
    fake: FakeKube,
    api_port: u16,
}

impl Case {
    fn new(name: &str) -> Self {
        Self::on_port(name, 7310)
    }

    fn on_port(name: &str, api_port: u16) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dp")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        fs::write(dir.join("secrets/alpha-model"), MODEL_TOKEN).expect("write a secret");
        fs::write(dir.join("secrets/forge-token"), FORGE_TOKEN).expect("write a secret");
        let fake = FakeKube::start(&dir);
        let case = Self {
            dir,
            fake,
            api_port,
        };
        case.config(IMAGE, &[alpha("")]);
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file: `daemon.kubernetes` on `image`, and `instances`.
    fn config(&self, image: &str, instances: &[String]) {
        self.config_env(
            image,
            &[
                "{name: GH_TOKEN, secret: secret:forge-token}",
                "{name: GIT_AUTHOR_NAME, value: conductor}",
            ],
            instances,
        );
    }

    /// Writes the config file: `daemon.kubernetes` on `image`, its `env` the entries `env`, and
    /// `instances`.
    fn config_env(&self, image: &str, env: &[&str], instances: &[String]) {
        let list = if instances.is_empty() {
            " []\n".to_owned()
        } else {
            format!("\n{}", instances.concat())
        };
        let env: String = if env.is_empty() {
            " []".to_owned()
        } else {
            env.iter()
                .map(|entry| format!("\n      - {entry}"))
                .collect()
        };
        let text = format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: {store:?}\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: {image}\n\
             \x20   owner: {OWNER}\n\
             \x20   api_port: {port}\n\
             \x20   volume: {{size: 20G, storage_class: fast-local}}\n\
             \x20   resources: {{requests: {{cpu: 500m, memory: 1Gi}}, limits: {{memory: 2Gi}}}}\n\
             \x20   env:{env}\n\
             instances:{list}",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            secrets = self.dir.join("secrets"),
            port = self.api_port,
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    fn daemon(&self) -> Daemon<Manager> {
        Daemon::open(
            self.file(),
            self.home(),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Manager::Pods(Box::new(Pods::new(self.fake.client()))),
        )
        .expect("open the daemon")
    }

    fn object(&self, resource: &str, name: &str) -> Option<Value> {
        self.fake.object(NAMESPACE, resource, name)
    }

    fn pod(&self) -> Value {
        self.object("pods", "conductor-instance-alpha")
            .expect("the pod conductor-instance-alpha")
    }

    /// Sets the status of the pod `conductor-instance-alpha` as the kubelet would.
    fn set_status(&self, status: Value) {
        let mut pod = self.pod();
        pod["status"] = status;
        self.fake.put(NAMESPACE, "pods", pod);
    }

    /// The pod runs at `127.0.0.1`, its container ready.
    fn set_running(&self) {
        self.set_status(json!({
            "phase": "Running",
            "podIP": "127.0.0.1",
            "containerStatuses": [
                {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
            ],
        }));
    }

    /// The token the daemon put into the instance's Secret.
    fn token(&self) -> String {
        let secret = self
            .object("secrets", "conductor-instance-alpha")
            .expect("the Secret");
        secret_value(&secret, "api_token")
    }

    /// How many requests of `method` reached `path` so far.
    fn count(&self, method: &str, path: &str) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|request| request.method == method && request.path == path)
            .count()
    }
}

/// The instance `alpha`: no paths, so each lies under its data mount; a model token; `extra`
/// lines appended.
fn alpha(extra: &str) -> String {
    format!(
        "  - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-org}}]\n\
         \x20   model_token: secret:alpha-model\n\
         {extra}"
    )
}

/// The value of `key` of `secret`, from its base64 `data` or its `stringData`.
fn secret_value(secret: &Value, key: &str) -> String {
    if let Some(text) = secret["stringData"][key].as_str() {
        return text.to_owned();
    }
    let encoded = secret["data"][key]
        .as_str()
        .unwrap_or_else(|| panic!("the Secret holds no {key}: {secret}"));
    String::from_utf8(pods::base64_decode(encoded).expect("base64")).expect("UTF-8")
}

const PODS: &str = "/api/v1/namespaces/conductor-instances/pods";

fn recorded(daemon: &Daemon<Manager>) -> Vec<(ReconcileAction, ReconcileReason, ReconcileResult)> {
    ReconciliationStorage::list(daemon.store())
        .into_iter()
        .map(|row| (row.data.action, row.data.reason, row.data.result))
        .collect()
}

fn row(daemon: &Daemon<Manager>) -> Value {
    daemon
        .rows()
        .into_iter()
        .find(|row| row["name"] == "alpha")
        .expect("the row of alpha")
}

/// Every key of `value` and its nested objects, with the path to it.
fn keys(value: &Value, at: &str, into: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                into.push(format!("{at}.{key}"));
                keys(inner, &format!("{at}.{key}"), into);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, at, into)),
        _ => {}
    }
}

fn labels() -> Value {
    json!({
        "app.kubernetes.io/managed-by": "conductord",
        "app.kubernetes.io/component": "instance-runtime",
        "conductor/daemon": OWNER,
        "conductor/instance": "alpha",
    })
}

/// One pass creates the instance's three objects, by the names of decision D7, each with the
/// label set; the pod is restricted in every field point 2 names, runs the image of
/// `daemon.kubernetes.image` with the runtime's command, takes its token from the Secret and holds
/// no literal of it; the Secret holds the rendered file, the token and each secret the instance
/// and `daemon.kubernetes.env` name, from the daemon's source; the claim has the size and class of
/// `daemon.kubernetes.volume`.
#[test]
fn one_pass_creates_the_secret_the_claim_and_a_restricted_pod_with_the_label_set() {
    let case = Case::new("objects");
    let mut daemon = case.daemon();
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");

    assert_eq!(
        case.fake.names(NAMESPACE, "pods"),
        ["conductor-instance-alpha"]
    );
    assert_eq!(
        case.fake.names(NAMESPACE, "secrets"),
        ["conductor-instance-alpha"]
    );
    assert_eq!(
        case.fake.names(NAMESPACE, "persistentvolumeclaims"),
        ["conductor-instance-alpha-data"]
    );
    let pod = case.pod();
    let secret = case
        .object("secrets", "conductor-instance-alpha")
        .expect("the Secret");
    let claim = case
        .object("persistentvolumeclaims", "conductor-instance-alpha-data")
        .expect("the claim");
    for object in [&pod, &secret, &claim] {
        assert_eq!(object["metadata"]["labels"], labels(), "{object}");
        assert!(
            object["metadata"].get("ownerReferences").is_none(),
            "no owner reference: {object}"
        );
    }

    // The Secret: the rendered file, the token and the secrets, from the source.
    let rendered = secret_value(&secret, "conductor.yaml");
    assert!(rendered.contains("name: alpha"), "{rendered}");
    assert!(
        rendered.contains("model_token: secret:alpha-model"),
        "{rendered}"
    );
    assert!(!rendered.contains("kubernetes"), "{rendered}");
    assert_eq!(secret_value(&secret, "alpha-model"), MODEL_TOKEN);
    assert_eq!(secret_value(&secret, "forge-token"), FORGE_TOKEN);
    let token = case.token();
    assert_eq!(token.len(), 64, "a minted token: {token}");

    // The claim.
    assert_eq!(claim["spec"]["resources"]["requests"]["storage"], "20Gi");
    assert_eq!(claim["spec"]["storageClassName"], "fast-local");
    assert_eq!(claim["spec"]["accessModes"], json!(["ReadWriteOnce"]));

    // The pod.
    let spec = &pod["spec"];
    assert_eq!(spec["restartPolicy"], "Never", "{pod}");
    assert_eq!(spec["automountServiceAccountToken"], false, "{pod}");
    assert_eq!(
        spec["securityContext"],
        json!({
            "runAsNonRoot": true,
            "runAsUser": 10001,
            "runAsGroup": 10001,
            "fsGroup": 10001,
            "seccompProfile": {"type": "RuntimeDefault"},
        }),
        "{pod}"
    );
    let containers = spec["containers"].as_array().expect("containers");
    assert_eq!(containers.len(), 1, "{pod}");
    let container = &containers[0];
    assert_eq!(container["image"], IMAGE, "{pod}");
    assert_eq!(
        container["command"],
        json!([
            "conductor",
            "--config",
            "/var/lib/conductor/data/conductor.yaml",
            "instance",
            "serve"
        ]),
        "{pod}"
    );
    assert_eq!(
        container["securityContext"]["allowPrivilegeEscalation"], false,
        "{pod}"
    );
    assert_eq!(
        container["securityContext"]["capabilities"]["drop"],
        json!(["ALL"]),
        "{pod}"
    );
    assert_eq!(
        container["resources"],
        json!({"requests": {"cpu": "500m", "memory": "1Gi"}, "limits": {"memory": "2Gi"}}),
        "{pod}"
    );
    for probe in ["livenessProbe", "readinessProbe"] {
        assert_eq!(
            container[probe]["httpGet"],
            json!({"path": "/healthz", "port": 7310}),
            "{probe}: {pod}"
        );
    }
    assert_eq!(container["ports"][0]["containerPort"], 7310, "{pod}");
    let env: Vec<Value> = container["env"].as_array().expect("env").clone();
    let variable = |name: &str| {
        env.iter()
            .find(|variable| variable["name"] == name)
            .cloned()
            .unwrap_or_else(|| panic!("no {name}: {pod}"))
    };
    assert_eq!(
        variable("CONDUCTOR_API_TOKEN")["valueFrom"]["secretKeyRef"],
        json!({"name": "conductor-instance-alpha", "key": "api_token"}),
        "{pod}"
    );
    assert_eq!(
        variable("GH_TOKEN")["valueFrom"]["secretKeyRef"],
        json!({"name": "conductor-instance-alpha", "key": "forge-token"}),
        "{pod}"
    );
    assert_eq!(variable("GIT_AUTHOR_NAME")["value"], "conductor", "{pod}");
    assert_eq!(variable("CONDUCTOR_API_LISTEN")["value"], "0.0.0.0:7310");
    assert_eq!(variable("CONDUCTOR_INSTANCE")["value"], "alpha");
    assert_eq!(
        variable("CONDUCTOR_CONFIG_SOURCE")["value"],
        "/run/secrets/conductor/conductor.yaml"
    );
    let mounts = &container["volumeMounts"];
    assert!(
        mounts.as_array().expect("mounts").contains(&json!({
            "name": "secret",
            "mountPath": "/run/secrets/conductor",
            "readOnly": true,
        })),
        "{pod}"
    );
    assert!(
        mounts.as_array().expect("mounts").contains(&json!({
            "name": "data",
            "mountPath": "/var/lib/conductor/data",
        })),
        "{pod}"
    );
    let volumes = spec["volumes"].as_array().expect("volumes");
    assert!(
        volumes.iter().any(|volume| volume["name"] == "secret"
            && volume["secret"]["secretName"] == "conductor-instance-alpha"),
        "{pod}"
    );
    assert!(
        volumes.iter().any(|volume| volume["name"] == "data"
            && volume["persistentVolumeClaim"]["claimName"] == "conductor-instance-alpha-data"),
        "{pod}"
    );

    // No literal secret, no host path, no host network, no privileged field anywhere.
    let text = pod.to_string();
    for literal in [token.as_str(), MODEL_TOKEN, FORGE_TOKEN] {
        assert!(
            !text.contains(literal),
            "a secret's literal in the pod: {pod}"
        );
    }
    let mut every = Vec::new();
    keys(&pod, "", &mut every);
    for forbidden in [
        "hostPath",
        "hostNetwork",
        "hostPID",
        "hostIPC",
        "privileged",
        "ownerReferences",
    ] {
        assert!(
            !every
                .iter()
                .any(|key| key.ends_with(&format!(".{forbidden}"))),
            "{forbidden} in the pod: {every:?}"
        );
    }
    assert!(
        pod["metadata"]["annotations"]["conductor/digest"].is_string(),
        "the digest annotation: {pod}"
    );
}

/// A secret the instance needs that the source does not hold refuses the start, naming it, and
/// nothing is created: no Secret, no claim, no pod.
#[test]
fn a_secret_the_source_lacks_refuses_the_start_and_creates_nothing() {
    let case = Case::new("missing");
    fs::remove_file(case.dir.join("secrets/forge-token")).expect("remove a secret");
    let mut daemon = case.daemon();
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Start);
    assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
    let detail = done[0].detail.clone().unwrap_or_default();
    assert!(detail.contains("forge-token"), "{detail}");
    assert!(
        !case
            .fake
            .requests()
            .iter()
            .any(|request| request.method == "POST"),
        "nothing is created: {:?}",
        case.fake.requests()
    );
    let row = row(&daemon);
    assert_eq!(row["state"], "Stopped", "{row}");
    assert!(
        row["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("forge-token"),
        "{row}"
    );
}

/// A Pending pod reads `Starting`; one whose image does not pull carries the waiting reason as
/// its row's detail; a Running pod whose container is ready reads `Running`. Each tick lists the
/// pods once, by the label set's first three labels.
#[test]
fn a_pending_pod_reads_starting_and_an_image_pull_back_off_is_the_detail() {
    let case = Case::new("pending");
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    let row = row(&daemon);
    assert_eq!(row["state"], "Starting", "{row}");
    assert_eq!(
        row["sources"],
        json!([{"kind": "GitHub", "value": {"owner": "example-org"}}]),
        "{row}"
    );
    assert_eq!(row["binary_version"], "1.0.0", "{row}");

    case.set_status(json!({
        "phase": "Pending",
        "containerStatuses": [{
            "name": "runtime",
            "ready": false,
            "state": {"waiting": {"reason": "ImagePullBackOff", "message": "Back-off pulling image"}},
        }],
    }));
    let lists = case.count("GET", PODS);
    let done = daemon.tick(start + Duration::from_secs(2)).expect("a tick");
    assert!(
        done.is_empty(),
        "a pod waiting for its image is not lost: {done:?}"
    );
    assert_eq!(case.count("GET", PODS), lists + 1, "one list per tick");
    let selector = case
        .fake
        .requests()
        .iter()
        .rev()
        .find(|request| request.method == "GET" && request.path == PODS)
        .and_then(|request| request.query.get("labelSelector").cloned())
        .unwrap_or_default();
    assert_eq!(
        selector,
        "app.kubernetes.io/managed-by=conductord,app.kubernetes.io/component=instance-runtime,\
         conductor/daemon=example-org"
    );
    let row = self::row(&daemon);
    let detail = row["detail"].as_str().unwrap_or_default();
    assert!(detail.contains("ImagePullBackOff"), "{row}");
    assert_ne!(row["state"], "Running", "{row}");

    case.set_running();
    daemon.tick(start + Duration::from_secs(4)).expect("a tick");
    let row = self::row(&daemon);
    assert_eq!(row["state"], "Running", "{row}");
    assert!(row["detail"].is_null(), "{row}");
    assert_eq!(case.count("POST", PODS), 1, "never started twice");
}

/// A pod that failed is a lost runtime: recorded as a failed start with its container's message,
/// and started again once the back-off passed, not before; a failed one without a message is
/// recorded with the last lines of its log.
#[test]
fn a_failed_pod_is_recorded_with_its_message_and_started_again_after_the_back_off() {
    let case = Case::new("failed");
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_status(json!({
        "phase": "Failed",
        "containerStatuses": [{
            "name": "runtime",
            "ready": false,
            "state": {"terminated": {"exitCode": 1, "reason": "Error", "message": "the store did not open"}},
        }],
    }));
    let first = daemon.tick(start + Duration::from_secs(2)).expect("a tick");
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(first[0].result, ReconcileResult::Refused, "{first:?}");
    assert_eq!(first[0].reason, ReconcileReason::LostRuntime);
    let detail = first[0].detail.clone().unwrap_or_default();
    assert!(detail.contains("the store did not open"), "{detail}");
    let row = row(&daemon);
    assert_eq!(row["state"], "Unreachable", "{row}");
    assert!(
        row["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("the store did not open"),
        "{row}"
    );

    let waiting = daemon.tick(start + Duration::from_secs(4)).expect("a tick");
    assert!(waiting.is_empty(), "{waiting:?}");
    assert_eq!(
        case.count("POST", PODS),
        1,
        "not started again before the back-off"
    );

    let again = daemon
        .tick(start + Duration::from_secs(13))
        .expect("a tick");
    assert_eq!(again.len(), 1, "{again:?}");
    assert_eq!(again[0].action, ReconcileAction::Start);
    assert_eq!(again[0].reason, ReconcileReason::LostRuntime);
    assert_eq!(again[0].result, ReconcileResult::Succeeded, "{again:?}");
    assert_eq!(
        case.count("POST", PODS),
        2,
        "started again after the back-off"
    );
    assert_eq!(case.pod()["status"]["phase"], "Pending");

    // Failed again, with no message: the log's last lines are the detail.
    case.set_status(json!({"phase": "Failed"}));
    case.fake.set_log(
        NAMESPACE,
        "conductor-instance-alpha",
        "starting\nconductor: instance serve: no such checkout\n",
    );
    let second = daemon
        .tick(start + Duration::from_secs(15))
        .expect("a tick");
    assert_eq!(second.len(), 1, "{second:?}");
    assert_eq!(second[0].result, ReconcileResult::Refused, "{second:?}");
    let detail = second[0].detail.clone().unwrap_or_default();
    assert!(detail.contains("no such checkout"), "{detail}");
}

/// A daemon that starts again finds its pod by the label list and adopts it, though what it kept
/// in its directory is gone: it records nothing, deletes nothing, creates nothing, and routes to
/// the runtime with the token it reads back from the instance's Secret.
#[test]
fn a_restarted_daemon_adopts_its_pod_records_no_start_and_deletes_no_pod() {
    let case = Case::new("adopt");
    let start = Instant::now();
    {
        let mut daemon = case.daemon();
        daemon
            .pass(ReconcileReason::Cadence, start)
            .expect("a pass");
    }
    case.set_running();
    fs::remove_file(case.dir.join("daemon").join(RUNTIMES_FILE)).expect("forget the runtimes");
    let before = case.fake.requests().len();

    let mut daemon = case.daemon();
    let done = daemon.tick(Instant::now()).expect("a tick");
    assert!(done.is_empty(), "{done:?}");
    assert_eq!(recorded(&daemon).len(), 1, "only the first start");
    let after: Vec<_> = case.fake.requests()[before..].to_vec();
    assert!(
        after.iter().all(|request| request.method == "GET"),
        "only reads: {after:?}"
    );
    let row = row(&daemon);
    assert_eq!(row["state"], "Running", "{row}");
    let routes = daemon.routes();
    let route = routes.get("alpha").expect("a route to alpha");
    assert_eq!(route.token, case.token());
    assert_eq!(
        route.tcp,
        Some("127.0.0.1:7310".parse().expect("an address"))
    );
}

/// An instance the file no longer holds loses its pod and its Secret, recorded as a stop; its
/// volume claim stays.
#[test]
fn removal_deletes_the_pod_and_the_secret_and_keeps_the_claim() {
    let case = Case::new("removal");
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();
    case.config(IMAGE, &[]);
    let done = daemon.tick(start + Duration::from_secs(2)).expect("a tick");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Stop);
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert!(case.fake.names(NAMESPACE, "pods").is_empty());
    assert!(case.fake.names(NAMESPACE, "secrets").is_empty());
    assert_eq!(
        case.fake.names(NAMESPACE, "persistentvolumeclaims"),
        ["conductor-instance-alpha-data"]
    );
}

/// A new image restarts the pod on it: the pod is deleted, then created again, recorded as one
/// restart; so does a change of the rendered file. A pass with nothing changed restarts nothing.
#[test]
fn an_image_change_or_a_changed_rendering_restarts_the_pod() {
    let case = Case::new("image");
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();
    let same = daemon
        .pass(ReconcileReason::Cadence, start + Duration::from_secs(2))
        .expect("a pass");
    assert!(same.is_empty(), "{same:?}");
    let digest = case.pod()["metadata"]["annotations"]["conductor/digest"].clone();

    let newer = "registry.example.com/conductor:1.1.0";
    case.config(newer, &[alpha("")]);
    let marker = case.fake.requests().len();
    let done = daemon
        .pass(
            ReconcileReason::ConfigChange,
            start + Duration::from_secs(4),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Restart);
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    let pod = case.pod();
    assert_eq!(pod["spec"]["containers"][0]["image"], newer, "{pod}");
    assert_ne!(pod["metadata"]["annotations"]["conductor/digest"], digest);
    let order: Vec<String> = case.fake.requests()[marker..]
        .iter()
        .filter(|request| {
            request.path.ends_with("/pods/conductor-instance-alpha") || request.path == PODS
        })
        .filter(|request| request.method != "GET")
        .map(|request| request.method.clone())
        .collect();
    assert_eq!(order, ["DELETE", "POST"], "deleted, then created again");
    assert_eq!(row(&daemon)["binary_version"], "1.1.0");

    case.set_running();
    case.config(newer, &[alpha("    thresholds: {disk_low: 50G}\n")]);
    let done = daemon
        .pass(
            ReconcileReason::ConfigChange,
            start + Duration::from_secs(6),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Restart);
    let rendered = secret_value(
        &case
            .object("secrets", "conductor-instance-alpha")
            .expect("the Secret"),
        "conductor.yaml",
    );
    assert!(rendered.contains("50G"), "{rendered}");
}

/// A labelled pod and Secret of this owner whose instance the file does not hold are deleted; a
/// pod of another owner is never touched, not even read by name.
#[test]
fn an_orphan_of_this_owner_is_deleted_and_one_of_another_owner_is_left() {
    let case = Case::new("orphans");
    let labelled = |name: &str, owner: &str| {
        json!({
            "apiVersion": "v1",
            "metadata": {
                "name": format!("conductor-instance-{name}"),
                "labels": {
                    "app.kubernetes.io/managed-by": "conductord",
                    "app.kubernetes.io/component": "instance-runtime",
                    "conductor/daemon": owner,
                    "conductor/instance": name,
                },
            },
            "status": {"phase": "Running"},
        })
    };
    case.fake.put(NAMESPACE, "pods", labelled("ghost", OWNER));
    case.fake
        .put(NAMESPACE, "secrets", labelled("ghost", OWNER));
    case.fake.put(
        NAMESPACE,
        "persistentvolumeclaims",
        labelled("ghost", OWNER),
    );
    case.fake
        .put(NAMESPACE, "pods", labelled("other", "another-owner"));
    case.fake
        .put(NAMESPACE, "secrets", labelled("other", "another-owner"));
    let mut daemon = case.daemon();
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert_eq!(
        case.fake.names(NAMESPACE, "pods"),
        ["conductor-instance-alpha", "conductor-instance-other"]
    );
    assert_eq!(
        case.fake.names(NAMESPACE, "secrets"),
        ["conductor-instance-alpha", "conductor-instance-other"]
    );
    assert!(
        case.fake
            .names(NAMESPACE, "persistentvolumeclaims")
            .contains(&"conductor-instance-ghost".to_owned()),
        "a claim is never deleted"
    );
    assert!(
        !case
            .fake
            .requests()
            .iter()
            .any(|request| request.path.contains("conductor-instance-other")),
        "another owner's objects are never named: {:?}",
        case.fake.requests()
    );
}

/// `POST /v1/instances/{name}/upgrade` of a pod-run instance answers `409` in one line: the
/// image names the version.
#[test]
fn an_upgrade_of_a_pod_run_instance_answers_409() {
    let case = Case::new("upgrade");
    let mut daemon = case.daemon();
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    let (reply, answer) = mpsc::channel();
    conductor_cli::daemon::answer_upgrade(
        &mut daemon,
        conductor_cli::daemon::upgrade::Request {
            name: "alpha".to_owned(),
            tag: "v1.1.0".to_owned(),
            actor: conductor_cli::actor::operator("operator-one"),
            reply,
        },
    );
    let (status, body) = answer.recv().expect("an answer");
    assert_eq!(status, "409 Conflict", "{body}");
    let error = body["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("daemon.kubernetes.image") && !error.contains('\n'),
        "{body}"
    );
    assert_eq!(recorded(&daemon).len(), 1, "nothing more is recorded");
}

/// `conductor doctor`'s rows of a daemon that runs pods: one per right it needs in the instance
/// namespace, a missing one named and failing the verdict; out of a cluster, one line that says
/// so and fails.
#[test]
fn the_doctor_names_each_right_and_fails_on_a_missing_one() {
    let case = Case::new("doctor");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    let kubernetes = config.daemon.kubernetes.expect("daemon.kubernetes");
    for resource in ["pods", "persistentvolumeclaims", "secrets"] {
        for verb in ["create", "get", "list", "delete"] {
            if (resource, verb) != ("secrets", "delete") {
                case.fake.allow(NAMESPACE, verb, resource);
            }
        }
    }
    case.fake.allow(NAMESPACE, "get", "pods/log");
    let client = case.fake.client();
    let lines = conductor_cli::doctor::kubernetes_lines(&kubernetes, Ok(&client));
    assert_eq!(lines.len(), 10, "{lines:?}");
    // Of the claims, create only: the daemon keeps every claim it made.
    let claims: Vec<&str> = lines
        .iter()
        .map(|(line, _)| line.as_str())
        .filter(|line| line.contains("persistentvolumeclaims"))
        .collect();
    assert_eq!(claims.len(), 1, "{lines:?}");
    assert!(
        claims[0].contains("create persistentvolumeclaims"),
        "{claims:?}"
    );
    let failures: Vec<String> = lines
        .iter()
        .filter_map(|(_, failure)| failure.clone())
        .collect();
    assert_eq!(failures.len(), 1, "{lines:?}");
    assert!(
        failures[0].contains("delete") && failures[0].contains("secrets"),
        "{failures:?}"
    );
    assert!(
        lines
            .iter()
            .any(|(line, failure)| failure.is_none() && line.contains("get pods/log")),
        "{lines:?}"
    );

    let out = conductor_cli::kube::Error::NotInCluster {
        missing: vec!["KUBERNETES_SERVICE_HOST".to_owned()],
    };
    let lines = conductor_cli::doctor::kubernetes_lines(&kubernetes, Err(&out));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].0.contains("not in a cluster"), "{lines:?}");
    assert!(lines[0].1.is_some(), "it fails the verdict: {lines:?}");
}

/// A config read before validation refused the daemon's own secrets in an instance or in `env`
/// still gets none into a pod: the start is refused before any secret is resolved, with "a secret
/// of the daemon's own" as its detail, and nothing reaches the API.
#[test]
fn a_start_naming_a_secret_of_the_daemons_own_creates_nothing() {
    use conductor_model::secret::SecretName;
    let case = Case::new("daemons-own");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let valid = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    // The control token is the instance's model token, or the secret `env` gives the pods.
    for own in ["alpha-model", "forge-token"] {
        let mut config = valid.clone();
        config.daemon.control_token = Some(SecretName(own.to_owned()));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let mut pods = Pods::new(case.fake.client());
        let asked = std::cell::RefCell::new(Vec::new());
        let error = pods
            .start(
                &kubernetes,
                &config,
                &config.instances[0],
                "an-instance-token",
                &|name: &SecretName| {
                    asked.borrow_mut().push(name.0.clone());
                    None
                },
            )
            .expect_err("refused");
        let error = format!("{error:#}");
        assert!(
            error.contains("a secret of the daemon's own") && error.contains("nothing is created"),
            "{own}: {error}"
        );
        assert!(!error.contains(own), "{own}: {error}");
        assert_eq!(
            asked.borrow().len(),
            0,
            "{own}: resolved {:?}",
            asked.borrow()
        );
        assert_eq!(
            pods.refused("alpha"),
            Some(error),
            "{own}: the row's detail"
        );
        assert!(
            case.fake.requests().is_empty(),
            "{own}: {:?}",
            case.fake.requests()
        );
    }
}

/// An instance pod's git has no identity: the image has no git config and the pod's home is not
/// kept. So the pod carries conductor's own for the records' commits, `GIT_AUTHOR_NAME`,
/// `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME` and `GIT_COMMITTER_EMAIL`, each where
/// `daemon.kubernetes.env` does not name it; a variable env names keeps env's value, once.
#[test]
fn an_instance_pod_commits_as_conductor_where_env_names_no_identity() {
    let case = Case::new("identity");
    let text = fs::read_to_string(case.file())
        .expect("the config file")
        .replace(
            "{name: GIT_AUTHOR_NAME, value: conductor}",
            "{name: GIT_AUTHOR_NAME, value: Example Org}",
        );
    let config = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    let env = made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
        .clone();
    let values = |name: &str| -> Vec<Value> {
        env.iter()
            .filter(|variable| variable["name"] == name)
            .map(|variable| variable["value"].clone())
            .collect()
    };
    assert_eq!(values("GIT_AUTHOR_NAME"), [json!("Example Org")], "{env:?}");
    assert_eq!(
        values("GIT_COMMITTER_NAME"),
        [json!("conductor")],
        "{env:?}"
    );
    for email in ["GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL"] {
        assert_eq!(values(email), [json!("conductor@localhost")], "{env:?}");
    }
}

/// The git identity of an instance that names one (story:instance-git-identity).
const BOT: &str = "    git_identity: {name: Example Bot, email: bot@example.com}\n";

/// A commit made in a fresh repository under `dir` with exactly the variables `made`'s pod gives
/// by value, a home of its own and no system or global git config: its author and committer as
/// `git log --format='%an <%ae> %cn <%ce>'` reads them.
fn commit_in_pod(dir: &std::path::Path, made: &pods::Manifests) -> String {
    let repository = dir.join("repository");
    let home = dir.join("git-home");
    fs::create_dir_all(&repository).expect("create the repository's directory");
    fs::create_dir_all(&home).expect("create git's home");
    let git = |args: &[&str]| {
        let mut command = std::process::Command::new("git");
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default());
        for entry in made.pod["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
        {
            if let (Some(name), Some(value)) = (entry["name"].as_str(), entry["value"].as_str()) {
                command.env(name, value);
            }
        }
        let output = command
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CEILING_DIRECTORIES", dir)
            .current_dir(&repository)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    };
    git(&["init", "--quiet"]);
    git(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "--message",
        "a commit",
    ]);
    git(&["log", "--format=%an <%ae> %cn <%ce>"])
}

/// An instance's `git_identity` (story:instance-git-identity) gives its pod `GIT_AUTHOR_NAME` and
/// `GIT_COMMITTER_NAME` from `name`, `GIT_AUTHOR_EMAIL` and `GIT_COMMITTER_EMAIL` from `email`,
/// each once and over a `daemon.kubernetes.env` entry of the variable; a commit made with the
/// pod's variables carries it as author and as committer. The rendered file does not write it.
/// An instance of the same file without it keeps `env`'s entries and the fixed default.
#[test]
fn an_instances_git_identity_is_its_pods_author_and_committer_over_env() {
    let case = Case::new("git-identity");
    case.config_env(
        IMAGE,
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GIT_AUTHOR_NAME, value: conductor}",
            "{name: GIT_AUTHOR_EMAIL, value: ops@example.com}",
        ],
        &[alpha(BOT), instance("beta", "{github: example-org}", "")],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");

    let alpha = pods::manifests(&kubernetes, &config, &config.instances[0]);
    for (variable, value) in [
        ("GIT_AUTHOR_NAME", "Example Bot"),
        ("GIT_AUTHOR_EMAIL", "bot@example.com"),
        ("GIT_COMMITTER_NAME", "Example Bot"),
        ("GIT_COMMITTER_EMAIL", "bot@example.com"),
    ] {
        assert_eq!(
            variables(&alpha, variable),
            [json!({"name": variable, "value": value})],
            "{}",
            alpha.pod
        );
    }
    assert!(
        !alpha.rendered.contains("git_identity") && !alpha.rendered.contains("Example Bot"),
        "{}",
        alpha.rendered
    );
    assert_eq!(
        commit_in_pod(&case.dir.join("alpha-commit"), &alpha),
        "Example Bot <bot@example.com> Example Bot <bot@example.com>"
    );

    let beta = pods::manifests(&kubernetes, &config, &config.instances[1]);
    for (variable, value) in [
        ("GIT_AUTHOR_NAME", "conductor"),
        ("GIT_AUTHOR_EMAIL", "ops@example.com"),
        ("GIT_COMMITTER_NAME", "conductor"),
        ("GIT_COMMITTER_EMAIL", "conductor@localhost"),
    ] {
        assert_eq!(
            variables(&beta, variable),
            [json!({"name": variable, "value": value})],
            "{}",
            beta.pod
        );
    }
    assert_eq!(
        commit_in_pod(&case.dir.join("beta-commit"), &beta),
        "conductor <ops@example.com> conductor <conductor@localhost>"
    );
}

/// An instance pod's environment carries `GLAB_SEND_TELEMETRY=false`, so no `glab` the pod runs
/// (the collectors', doctor's, the watch's, a session's) sends usage data to the forge; where
/// `daemon.kubernetes.env` names the variable, the operator's value stands, once.
#[test]
fn an_instance_pod_sends_no_glab_usage_data_unless_env_names_the_variable() {
    let case = Case::new("glab-telemetry");
    let made = |case: &Case| {
        let text = fs::read_to_string(case.file()).expect("the config file");
        let config = conductor_cli::config::parse(&text, &case.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        pods::manifests(&kubernetes, &config, &config.instances[0])
    };
    let fixed = made(&case);
    assert_eq!(
        variables(&fixed, "GLAB_SEND_TELEMETRY"),
        [json!({"name": "GLAB_SEND_TELEMETRY", "value": "false"})],
        "{}",
        fixed.pod
    );

    case.config_env(
        IMAGE,
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GLAB_SEND_TELEMETRY, value: \"true\"}",
        ],
        &[alpha("")],
    );
    let operators = made(&case);
    assert_eq!(
        variables(&operators, "GLAB_SEND_TELEMETRY"),
        [json!({"name": "GLAB_SEND_TELEMETRY", "value": "true"})],
        "{}",
        operators.pod
    );
}

/// The runtime is not the first process of its pod: the pod shares one process namespace, so its
/// pause process is, reaps what the agents leave behind, and a `SIGTERM` ends the runtime at once
/// rather than being ignored as the first process's would be until the grace period kills it.
/// That is what `docker run --init` gives the container.
#[test]
fn the_runtime_is_not_the_first_process_of_its_pod() {
    let case = Case::new("first-process");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    assert_eq!(
        made.pod["spec"]["shareProcessNamespace"],
        json!(true),
        "{}",
        made.pod
    );
}

/// The daemon's own seeding skips an instance that runs as a pod: its runtime seeds itself. (The
/// seeding itself is not run here: it reads the process's home.)
#[test]
fn the_daemons_seeding_skips_pod_run_instances() {
    let case = Case::new("seeding");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    assert!(!conductor_cli::daemon::unattended::seeds_here(&config));
    let mut unit_run = config.clone();
    unit_run.daemon.kubernetes = None;
    assert!(conductor_cli::daemon::unattended::seeds_here(&unit_run));
}

/// The pod of an instance that names no `forge_token` (story:instance-forge-token) and has a
/// `github` source under the daemon's `GH_TOKEN` is the pod the supervisor of v1.4.0 made for
/// this case's file, and git's three variables after the git identity
/// (story:pod-git-reaches-forge), and `GLAB_SEND_TELEMETRY` after them, and the secret volume's
/// `items`: its spec, its claim, its rendered file, the secrets its Secret holds and its
/// digest, held to `tests/fixtures/k8s/pod-alpha.json`. The pods that carry no token of a forge
/// of theirs are held to v1.4.0, `GLAB_SEND_TELEMETRY` and the secret volume's `items`, byte for
/// byte by the three fixtures
/// `pod-alpha-no-token.json`, `pod-alpha-local.json` and `pod-alpha-git-config.json`.
#[test]
fn the_pod_of_an_instance_under_the_daemons_token_is_the_one_of_v1_4_0_and_gits_variables() {
    let case = Case::new("before-the-key");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home()).expect("a valid file");
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    let made = json!({
        "pod": made.pod,
        "claim": made.claim,
        "rendered": made.rendered,
        "secrets": made.secrets,
        "digest": made.digest,
    });
    let made = format!(
        "{}\n",
        serde_json::to_string_pretty(&made).expect("the objects as JSON")
    );
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/k8s/pod-alpha.json");
    let before = fs::read_to_string(&fixture).unwrap_or_default();
    if made != before {
        let now = case.dir.join("pod-alpha.json");
        fs::write(&now, &made).expect("write what is made now");
        panic!(
            "the pod of an instance under the daemon's token differs from {}; what is made now is {}",
            fixture.display(),
            now.display()
        );
    }
}

/// The instance `name` of a case's file: no paths, the source `source`, `extra` lines appended.
fn instance(name: &str, source: &str, extra: &str) -> String {
    format!(
        "  - name: {name}\n\
         \x20   placement: none\n\
         \x20   session_prefix: {name}\n\
         \x20   sources: [{source}]\n\
         {extra}"
    )
}

/// The `env` entries of `made`'s pod named `variable`.
fn variables(made: &pods::Manifests, variable: &str) -> Vec<Value> {
    made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
        .iter()
        .filter(|entry| entry["name"] == variable)
        .cloned()
        .collect()
}

/// The `env` entry `variable` read from the key `key` of `instance`'s Secret.
fn from_secret(variable: &str, instance: &str, key: &str) -> Value {
    json!({
        "name": variable,
        "valueFrom": {"secretKeyRef": {"name": format!("conductor-instance-{instance}"), "key": key}},
    })
}

/// The values of the forge tokens each instance of the forge-token cases names.
const ALPHA_FORGE: &str = "alpha-forge-value-2222";
const GAMMA_FORGE: &str = "gamma-forge-value-3333";

/// An instance's `forge_token` (story:instance-forge-token): its pod reads the variable of its
/// forge, `GH_TOKEN` for a `github` source and `GITLAB_TOKEN` for a `gitlab` one, from the key of
/// that secret in its own Secret, once, in place of the `daemon.kubernetes.env` entry of that
/// variable; every other entry stays, and its Secret holds the instance's token and not the
/// daemon-wide one. An instance that names none keeps the daemon-wide entry and holds no other
/// instance's token. The rendered file leaves the key out and validates in the pod, where no
/// `daemon.kubernetes` is set.
#[test]
fn an_instances_forge_token_replaces_the_daemons_variable_in_its_own_pod_only() {
    let case = Case::new("forge-token");
    fs::write(case.dir.join("secrets/alpha-forge"), ALPHA_FORGE).expect("write a secret");
    fs::write(case.dir.join("secrets/gamma-forge"), GAMMA_FORGE).expect("write a secret");
    case.config(
        IMAGE,
        &[
            alpha("    forge_token: secret:alpha-forge\n"),
            instance("beta", "{github: example-org}", ""),
            instance(
                "gamma",
                "{gitlab: example-org}",
                "    forge_token: secret:gamma-forge\n",
            ),
        ],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = |name: &str| {
        let instance = config
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .unwrap_or_else(|| panic!("the instance {name}"));
        pods::manifests(&kubernetes, &config, instance)
    };

    let alpha = made("alpha");
    assert_eq!(
        variables(&alpha, "GH_TOKEN"),
        [from_secret("GH_TOKEN", "alpha", "alpha-forge")],
        "{}",
        alpha.pod
    );
    assert_eq!(
        variables(&alpha, "GIT_AUTHOR_NAME"),
        [json!({"name": "GIT_AUTHOR_NAME", "value": "conductor"})],
        "every other entry of daemon.kubernetes.env stays: {}",
        alpha.pod
    );
    assert!(
        variables(&alpha, "GITLAB_TOKEN").is_empty(),
        "{}",
        alpha.pod
    );
    assert!(alpha.secrets.contains("alpha-forge"), "{:?}", alpha.secrets);
    assert!(
        !alpha.secrets.contains("forge-token"),
        "{:?}",
        alpha.secrets
    );
    assert!(
        !alpha.rendered.contains("forge_token:") && !alpha.rendered.contains("alpha-forge"),
        "{}",
        alpha.rendered
    );
    conductor_cli::config::parse(&alpha.rendered, &case.home()).unwrap_or_else(|problems| {
        panic!(
            "the rendered file does not validate: {problems:?}\n{}",
            alpha.rendered
        )
    });

    let beta = made("beta");
    assert_eq!(
        variables(&beta, "GH_TOKEN"),
        [from_secret("GH_TOKEN", "beta", "forge-token")],
        "{}",
        beta.pod
    );
    assert!(beta.secrets.contains("forge-token"), "{:?}", beta.secrets);
    for other in ["alpha-forge", "gamma-forge"] {
        assert!(!beta.secrets.contains(other), "{:?}", beta.secrets);
    }

    let gamma = made("gamma");
    assert_eq!(
        variables(&gamma, "GITLAB_TOKEN"),
        [from_secret("GITLAB_TOKEN", "gamma", "gamma-forge")],
        "{}",
        gamma.pod
    );
    assert!(
        variables(&gamma, "GH_TOKEN").is_empty(),
        "an instance with a forge token of its own carries none of the daemon's: {}",
        gamma.pod
    );
    assert!(
        !gamma.secrets.contains("forge-token"),
        "{:?}",
        gamma.secrets
    );

    // One pass: each Secret holds its own instance's token, from the daemon's secrets source.
    let mut daemon = case.daemon();
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert!(
        done.iter()
            .all(|row| row.result == ReconcileResult::Succeeded),
        "{done:?}"
    );
    let secret = |name: &str| {
        case.object("secrets", &format!("conductor-instance-{name}"))
            .unwrap_or_else(|| panic!("the Secret of {name}"))
    };
    let keys = |secret: &Value| -> Vec<String> {
        secret["data"]
            .as_object()
            .map(|data| data.keys().cloned().collect())
            .unwrap_or_default()
    };
    let (alpha, beta, gamma) = (secret("alpha"), secret("beta"), secret("gamma"));
    assert_eq!(secret_value(&alpha, "alpha-forge"), ALPHA_FORGE);
    assert!(
        !keys(&alpha).contains(&"forge-token".to_owned()),
        "{:?}",
        keys(&alpha)
    );
    assert_eq!(secret_value(&beta, "forge-token"), FORGE_TOKEN);
    assert!(
        !keys(&beta).contains(&"alpha-forge".to_owned()),
        "{:?}",
        keys(&beta)
    );
    assert_eq!(secret_value(&gamma, "gamma-forge"), GAMMA_FORGE);
    assert!(
        !keys(&gamma).contains(&"forge-token".to_owned()),
        "{:?}",
        keys(&gamma)
    );
}

/// An instance that names its own `forge_token` carries none of the daemon's forge token
/// variables (decision F2-D7): a `gitlab` instance under a daemon whose `env` names `GH_TOKEN`
/// gets `GITLAB_TOKEN` from its own secret and no `GH_TOKEN`, and a `github` instance under a
/// daemon-wide `GITLAB_TOKEN` no `GITLAB_TOKEN`; its Secret holds the instance's secret and not
/// the daemon's forge secret, unless an entry the pod keeps names that secret too. Every other
/// entry stays. git's helper follows the variables the pod carries after that.
#[test]
fn an_instances_own_forge_token_leaves_every_daemon_forge_variable_out_of_its_pod() {
    let case = Case::new("forge-token-only-own");
    case.config_env(
        IMAGE,
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GITLAB_TOKEN, secret: secret:gitlab-forge}",
            "{name: GIT_AUTHOR_NAME, value: conductor}",
        ],
        &[
            instance(
                "gamma",
                "{gitlab: example-org}",
                "    forge_token: secret:gamma-forge\n",
            ),
            instance(
                "delta",
                "{github: example-org}",
                "    forge_token: secret:delta-forge\n",
            ),
        ],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let made = |config: &conductor_model::config::Config, name: &str| {
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let instance = config
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .unwrap_or_else(|| panic!("the instance {name}"));
        pods::manifests(&kubernetes, config, instance)
    };

    let gamma = made(&config, "gamma");
    assert_eq!(
        variables(&gamma, "GITLAB_TOKEN"),
        [from_secret("GITLAB_TOKEN", "gamma", "gamma-forge")],
        "{}",
        gamma.pod
    );
    assert!(variables(&gamma, "GH_TOKEN").is_empty(), "{}", gamma.pod);
    assert_eq!(
        variables(&gamma, "GIT_AUTHOR_NAME"),
        [json!({"name": "GIT_AUTHOR_NAME", "value": "conductor"})],
        "every other entry stays: {}",
        gamma.pod
    );
    assert!(gamma.secrets.contains("gamma-forge"), "{:?}", gamma.secrets);
    for daemons in ["forge-token", "gitlab-forge"] {
        assert!(!gamma.secrets.contains(daemons), "{:?}", gamma.secrets);
    }
    assert_eq!(
        git_config(&gamma),
        helpers(&[("https://gitlab.com", GITLAB_HELPER)]),
        "{}",
        gamma.pod
    );

    let delta = made(&config, "delta");
    assert_eq!(
        variables(&delta, "GH_TOKEN"),
        [from_secret("GH_TOKEN", "delta", "delta-forge")],
        "{}",
        delta.pod
    );
    assert!(
        variables(&delta, "GITLAB_TOKEN").is_empty(),
        "{}",
        delta.pod
    );
    assert!(delta.secrets.contains("delta-forge"), "{:?}", delta.secrets);
    for daemons in ["forge-token", "gitlab-forge"] {
        assert!(!delta.secrets.contains(daemons), "{:?}", delta.secrets);
    }
    assert_eq!(
        git_config(&delta),
        helpers(&[("https://github.com", GITHUB_HELPER)]),
        "{}",
        delta.pod
    );

    // A secret a kept entry names too stays in the Secret, for that entry.
    let shared = text.replace(
        "{name: GIT_AUTHOR_NAME, value: conductor}",
        "{name: FORGE_MIRROR_TOKEN, secret: secret:forge-token}",
    );
    let config = conductor_cli::config::parse(&shared, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{shared}"));
    let gamma = made(&config, "gamma");
    assert!(variables(&gamma, "GH_TOKEN").is_empty(), "{}", gamma.pod);
    assert_eq!(
        variables(&gamma, "FORGE_MIRROR_TOKEN"),
        [from_secret("FORGE_MIRROR_TOKEN", "gamma", "forge-token")],
        "{}",
        gamma.pod
    );
    assert!(gamma.secrets.contains("forge-token"), "{:?}", gamma.secrets);
    assert!(
        !gamma.secrets.contains("gitlab-forge"),
        "{:?}",
        gamma.secrets
    );
}

/// git's credential helper for `https://github.com` in a pod that carries `GH_TOKEN`
/// (story:pod-git-reaches-forge): it answers `get` only, with the token read from the variable at
/// the time of the call; its text holds no secret.
const GITHUB_HELPER: &str = r#"!f() { test "$1" = get && printf 'username=%s\npassword=%s\n' x-access-token "$GH_TOKEN"; }; f"#;

/// git's credential helper for a `gitlab` source's origin in a pod that carries `GITLAB_TOKEN`.
const GITLAB_HELPER: &str =
    r#"!f() { test "$1" = get && printf 'username=%s\npassword=%s\n' oauth2 "$GITLAB_TOKEN"; }; f"#;

/// The `GIT_CONFIG_*` entries of `made`'s pod, in their order, as name and value.
fn git_config(made: &pods::Manifests) -> Vec<(String, String)> {
    made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
        .iter()
        .filter_map(|entry| {
            let name = entry["name"].as_str()?;
            name.starts_with("GIT_CONFIG_").then(|| {
                (
                    name.to_owned(),
                    entry["value"].as_str().unwrap_or_default().to_owned(),
                )
            })
        })
        .collect()
}

/// The entries one helper per `(origin, helper)` of `helpers` gives a pod, in that order.
fn helpers(helpers: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut entries = vec![("GIT_CONFIG_COUNT".to_owned(), helpers.len().to_string())];
    for (index, (origin, helper)) in helpers.iter().enumerate() {
        entries.push((
            format!("GIT_CONFIG_KEY_{index}"),
            format!("credential.{origin}.helper"),
        ));
        entries.push((format!("GIT_CONFIG_VALUE_{index}"), (*helper).to_owned()));
    }
    entries
}

/// The index of the first `env` entry of `made`'s pod named `variable`.
fn position(made: &pods::Manifests, variable: &str) -> usize {
    made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
        .iter()
        .position(|entry| entry["name"] == variable)
        .unwrap_or_else(|| panic!("the pod has no {variable}: {}", made.pod))
}

/// What git itself answers to `git credential fill` for `protocol` and `host` in the environment
/// of `made`'s pod, with no network: the pod's plain entries exported, each of `tokens` set as the
/// value a `secretKeyRef` would give, an isolated home under `dir`, no system or global config
/// and no terminal prompt. Answers whether git succeeded, its standard output and its error.
fn fill(
    dir: &std::path::Path,
    made: &pods::Manifests,
    tokens: &[(&str, &str)],
    protocol: &str,
    host: &str,
) -> (bool, String, String) {
    let home = dir.join("git-home");
    fs::create_dir_all(&home).expect("create git's home");
    let mut command = std::process::Command::new("git");
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default());
    for entry in made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
    {
        if let (Some(name), Some(value)) = (entry["name"].as_str(), entry["value"].as_str()) {
            command.env(name, value);
        }
    }
    for (name, value) in tokens {
        command.env(name, value);
    }
    command
        .env("HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CEILING_DIRECTORIES", dir)
        .current_dir(&home)
        .args(["credential", "fill"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("start git");
    child
        .stdin
        .take()
        .expect("git's input")
        .write_all(format!("protocol={protocol}\nhost={host}\n\n").as_bytes())
        .expect("write git's input");
    let output = child.wait_with_output().expect("git ends");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// git answers `user` and `token` for `host` over HTTPS in `made`'s pod.
fn answers(
    dir: &std::path::Path,
    made: &pods::Manifests,
    tokens: &[(&str, &str)],
    host: &str,
    user: &str,
    token: &str,
) {
    let (ok, out, err) = fill(dir, made, tokens, "https", host);
    assert!(ok, "git credential fill for {host} failed: {err}\n{out}");
    assert!(
        out.contains(&format!("username={user}\npassword={token}\n")),
        "git credential fill for {host} answered: {out}"
    );
}

/// git offers no password for `host` over `protocol` in `made`'s pod: no helper answers, so it
/// asks for a user name, which no terminal gives, and fails.
fn answers_none(
    dir: &std::path::Path,
    made: &pods::Manifests,
    tokens: &[(&str, &str)],
    protocol: &str,
    host: &str,
) {
    let (ok, out, err) = fill(dir, made, tokens, protocol, host);
    assert!(
        !ok && !out.contains("password=") && err.contains("terminal prompts disabled"),
        "git credential fill for {protocol}://{host} answered: ok {ok}, out {out:?}, error {err:?}"
    );
}

/// git over HTTPS in an instance pod that carries a forge token (story:pod-git-reaches-forge):
/// the pod's environment gives git one credential helper, for the origin of the instance's forge,
/// `https://github.com` for a `github` source and `https://<host>` for a `gitlab` one, whether
/// the token comes from the instance's `forge_token` or from `daemon.kubernetes.env`; after the
/// token's entry and before the git identity. git itself, run in that environment, answers the
/// token for that host, as the variable holds it at the time of the call, and none for another
/// host, a look-alike host, the same host over HTTP or the same name on another port. A pod that
/// carries the token of a forge its instance has no source on gets no helper for it.
#[test]
fn git_in_a_pod_with_a_forge_token_reaches_its_forge_with_it_and_no_other_host() {
    let case = Case::new("git-forge");
    case.config(
        IMAGE,
        &[
            alpha("    forge_token: secret:alpha-forge\n"),
            instance("beta", "{github: example-org}", ""),
            instance(
                "gamma",
                "{gitlab: example-org, host: gitlab.example.com:8443}",
                "    forge_token: secret:gamma-forge\n",
            ),
        ],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = |name: &str| {
        let instance = config
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .unwrap_or_else(|| panic!("the instance {name}"));
        pods::manifests(&kubernetes, &config, instance)
    };
    let (first, second) = (
        ["fixture", "value", "one"].join("-"),
        ["fixture", "value", "two"].join("-"),
    );

    for (name, token) in [("alpha", "GH_TOKEN"), ("beta", "GH_TOKEN")] {
        let made = made(name);
        assert_eq!(
            git_config(&made),
            helpers(&[("https://github.com", GITHUB_HELPER)]),
            "{name}: {}",
            made.pod
        );
        let count = position(&made, "GIT_CONFIG_COUNT");
        assert!(
            position(&made, token) < count && count < position(&made, "GIT_AUTHOR_EMAIL"),
            "{name}: the helper stands after the token and before the identity: {}",
            made.pod
        );
        let spec = made.pod.to_string();
        for value in [ALPHA_FORGE, FORGE_TOKEN] {
            assert!(!spec.contains(value), "{name}: a token in the pod: {spec}");
        }
        for value in [first.as_str(), second.as_str()] {
            answers(
                &case.dir,
                &made,
                &[("GH_TOKEN", value)],
                "github.com",
                "x-access-token",
                value,
            );
        }
        for host in ["example.com", "github.com.example.com", "api.github.com"] {
            answers_none(
                &case.dir,
                &made,
                &[("GH_TOKEN", first.as_str())],
                "https",
                host,
            );
        }
        answers_none(
            &case.dir,
            &made,
            &[("GH_TOKEN", first.as_str())],
            "http",
            "github.com",
        );
    }

    let gamma = made("gamma");
    assert_eq!(
        git_config(&gamma),
        helpers(&[("https://gitlab.example.com:8443", GITLAB_HELPER)]),
        "gamma has no github source: {}",
        gamma.pod
    );
    let count = position(&gamma, "GIT_CONFIG_COUNT");
    assert!(
        position(&gamma, "GITLAB_TOKEN") < count && count < position(&gamma, "GIT_AUTHOR_EMAIL"),
        "{}",
        gamma.pod
    );
    assert!(
        !gamma.pod.to_string().contains(GAMMA_FORGE),
        "{}",
        gamma.pod
    );
    let tokens = [
        ("GITLAB_TOKEN", first.as_str()),
        ("GH_TOKEN", second.as_str()),
    ];
    answers(
        &case.dir,
        &gamma,
        &tokens,
        "gitlab.example.com:8443",
        "oauth2",
        &first,
    );
    for host in [
        "gitlab.example.com",
        "gitlab.com",
        "github.com",
        "example.com",
    ] {
        answers_none(&case.dir, &gamma, &tokens, "https", host);
    }
    answers_none(
        &case.dir,
        &gamma,
        &tokens,
        "http",
        "gitlab.example.com:8443",
    );
}

/// An instance with sources on both forges, under a daemon whose `env` gives both variables, one
/// as a `secret` and one as a `value`, and names its GitLab host (story:pod-git-reaches-forge,
/// decision F3-D2): one helper per origin, `github.com` first, then the `gitlab` host once
/// however its sources write it. An instance with a GitLab token of its own on several `gitlab`
/// hosts gets each host once, sorted, so the pod's digest does not move with the order of the
/// sources. git answers each forge's token for its host only.
#[test]
fn an_instance_on_both_forges_gets_one_helper_per_origin_in_a_fixed_order() {
    let case = Case::new("git-both-forges");
    let gitlab_value = ["fixture", "gitlab", "value"].join("-");
    case.config_env(
        IMAGE,
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            &format!("{{name: GITLAB_TOKEN, value: {gitlab_value}}}"),
            "{name: GITLAB_HOST, value: gitlab.example.org}",
        ],
        &[
            instance(
                "delta",
                "{gitlab: example-org, host: GitLab.Example.org}, {github: example-org}, \
                 {gitlab: other-group, host: gitlab.example.org}",
                "",
            ),
            instance(
                "gamma",
                "{gitlab: example-org, host: gitlab.example.org}, {gitlab: example-group}, \
                 {gitlab: other-group, host: gitlab.example.org}",
                "    forge_token: secret:gamma-forge\n",
            ),
        ],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    assert_eq!(
        git_config(&made),
        helpers(&[
            ("https://github.com", GITHUB_HELPER),
            ("https://gitlab.example.org", GITLAB_HELPER),
        ]),
        "{}",
        made.pod
    );
    let github = ["fixture", "github", "value"].join("-");
    let tokens = [
        ("GH_TOKEN", github.as_str()),
        ("GITLAB_TOKEN", gitlab_value.as_str()),
    ];
    answers(
        &case.dir,
        &made,
        &tokens,
        "github.com",
        "x-access-token",
        &github,
    );
    answers(
        &case.dir,
        &made,
        &tokens,
        "gitlab.example.org",
        "oauth2",
        &gitlab_value,
    );
    for host in ["gitlab.com", "gitlab.example.com", "example.org"] {
        answers_none(&case.dir, &made, &tokens, "https", host);
    }

    let gamma = pods::manifests(&kubernetes, &config, &config.instances[1]);
    assert_eq!(
        git_config(&gamma),
        helpers(&[
            ("https://gitlab.com", GITLAB_HELPER),
            ("https://gitlab.example.org", GITLAB_HELPER),
        ]),
        "{}",
        gamma.pod
    );
    let own = [("GITLAB_TOKEN", gitlab_value.as_str())];
    for host in ["gitlab.com", "gitlab.example.org"] {
        answers(&case.dir, &gamma, &own, host, "oauth2", &gitlab_value);
    }
    answers_none(&case.dir, &gamma, &own, "https", "github.com");
}

/// The daemon's entries of the variables `glab` reads a token from: one secret for two of them,
/// another for the third.
const DAEMON_GITLAB_ENV: [&str; 3] = [
    "{name: GITLAB_TOKEN, secret: secret:gitlab-forge}",
    "{name: GITLAB_ACCESS_TOKEN, secret: secret:gitlab-forge}",
    "{name: OAUTH_TOKEN, secret: secret:gitlab-oauth}",
];

/// The variables `glab` reads a token from, and the secret the daemon's entry of each names.
const GITLAB_VARIABLES: [(&str, &str); 3] = [
    ("GITLAB_TOKEN", "gitlab-forge"),
    ("GITLAB_ACCESS_TOKEN", "gitlab-forge"),
    ("OAUTH_TOKEN", "gitlab-oauth"),
];

/// The daemon's GitLab token belongs to one host (decision F3-D2): the value of a `GITLAB_HOST`
/// entry of `daemon.kubernetes.env`, `gitlab.com` where there is none, compared lowercased as
/// git's key is. A pod carries the daemon's entries of every variable `glab` reads a token from,
/// and git's helper for that host, only where every `gitlab` source of its instance is on that
/// host. An instance on another host, on that host and another, or on that host with a port
/// written (`host:443` is another text than `host`) gets none of those entries and no helper, its
/// Secret holds none of their secrets, and git offers the token to its host from nowhere. An
/// instance with no `gitlab` source gets none of those entries either (decision F4-D1), and no
/// helper.
#[test]
fn the_daemons_gitlab_token_reaches_only_pods_whose_gitlab_sources_are_all_on_its_host() {
    for (name, named, host) in [
        (
            "gitlab-host-named",
            Some("GitLab.Example.org"),
            "gitlab.example.org",
        ),
        ("gitlab-host-default", None, "gitlab.com"),
    ] {
        let case = Case::new(name);
        let value = ["gitlab-forge", "-value-", "4444"].concat();
        let oauth = ["gitlab-oauth", "-value-", "5555"].concat();
        fs::write(case.dir.join("secrets/gitlab-forge"), &value).expect("write a secret");
        fs::write(case.dir.join("secrets/gitlab-oauth"), &oauth).expect("write a secret");
        let named_entry = named.map(|named| format!("{{name: GITLAB_HOST, value: {named}}}"));
        let mut env: Vec<&str> = DAEMON_GITLAB_ENV.to_vec();
        env.push("{name: GIT_AUTHOR_NAME, value: conductor}");
        env.extend(named_entry.as_deref());
        let other = "gitlab.other.example";
        let on = |group: &str, host: &str| format!("{{gitlab: {group}, host: {host:?}}}");
        let upper = host.to_ascii_uppercase();
        let with_port = format!("{host}:443");
        case.config_env(
            IMAGE,
            &env,
            &[
                instance(
                    "alpha",
                    &format!(
                        "{}, {}",
                        on("example-org", &upper),
                        on("example-group", host)
                    ),
                    "",
                ),
                instance("beta", &on("example-org", other), ""),
                instance(
                    "gamma",
                    &format!(
                        "{}, {}",
                        on("example-org", host),
                        on("example-group", other)
                    ),
                    "",
                ),
                instance("delta", &on("example-org", &with_port), ""),
                instance("epsilon", "{github: example-org}", ""),
            ],
        );
        let text = fs::read_to_string(case.file()).expect("the config file");
        let config = conductor_cli::config::parse(&text, &case.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let made = |instance: &str| {
            let instance = config
                .instances
                .iter()
                .find(|named| named.name.0 == instance)
                .unwrap_or_else(|| panic!("the instance {instance}"));
            pods::manifests(&kubernetes, &config, instance)
        };

        let alpha = made("alpha");
        for (variable, secret) in GITLAB_VARIABLES {
            assert_eq!(
                variables(&alpha, variable),
                [from_secret(variable, "alpha", secret)],
                "{name}: alpha keeps {variable}: {}",
                alpha.pod
            );
        }
        for secret in ["gitlab-forge", "gitlab-oauth"] {
            assert!(
                alpha.secrets.contains(secret),
                "{name}: alpha: {:?}",
                alpha.secrets
            );
        }
        assert_eq!(
            git_config(&alpha),
            helpers(&[(&format!("https://{host}"), GITLAB_HELPER)]),
            "{name}: {}",
            alpha.pod
        );
        let tokens = [("GITLAB_TOKEN", value.as_str())];
        answers(&case.dir, &alpha, &tokens, host, "oauth2", &value);
        answers_none(&case.dir, &alpha, &tokens, "https", other);
        let epsilon = made("epsilon");
        for (variable, _) in GITLAB_VARIABLES {
            assert!(
                variables(&epsilon, variable).is_empty(),
                "{name}: epsilon has no gitlab source and carries the daemon's {variable}: {}",
                epsilon.pod
            );
        }
        for secret in ["gitlab-forge", "gitlab-oauth"] {
            assert!(
                !epsilon.secrets.contains(secret),
                "{name}: epsilon: {:?}",
                epsilon.secrets
            );
        }
        assert!(
            git_config(&epsilon).is_empty(),
            "{name}: epsilon has no gitlab source and carries no GitHub token: {}",
            epsilon.pod
        );

        for elsewhere in ["beta", "gamma", "delta"] {
            let made = made(elsewhere);
            for (variable, _) in GITLAB_VARIABLES {
                assert!(
                    variables(&made, variable).is_empty(),
                    "{name}: {elsewhere} carries the daemon's {variable}: {}",
                    made.pod
                );
            }
            assert!(
                git_config(&made).is_empty(),
                "{name}: {elsewhere} gets a helper for the daemon's GitLab token: {}",
                made.pod
            );
            for secret in ["gitlab-forge", "gitlab-oauth"] {
                assert!(
                    !made.secrets.contains(secret),
                    "{name}: {elsewhere}: {:?}",
                    made.secrets
                );
            }
            assert_eq!(
                variables(&made, "GIT_AUTHOR_NAME"),
                [json!({"name": "GIT_AUTHOR_NAME", "value": "conductor"})],
                "{name}: {elsewhere}: every other entry stays: {}",
                made.pod
            );
            if let Some(named) = named {
                assert_eq!(
                    variables(&made, "GITLAB_HOST"),
                    [json!({"name": "GITLAB_HOST", "value": named})],
                    "{name}: {elsewhere}: {}",
                    made.pod
                );
            }
            for at in [other, with_port.as_str()] {
                answers_none(&case.dir, &made, &tokens, "https", at);
            }
        }

        // One pass: the Secret of each instance elsewhere holds neither secret under any key.
        let mut daemon = case.daemon();
        let done = daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("a pass");
        assert!(
            done.iter()
                .all(|row| row.result == ReconcileResult::Succeeded),
            "{name}: {done:?}"
        );
        let secret = |instance: &str| {
            case.object("secrets", &format!("conductor-instance-{instance}"))
                .unwrap_or_else(|| panic!("{name}: the Secret of {instance}"))
        };
        assert_eq!(secret_value(&secret("alpha"), "gitlab-forge"), value);
        for elsewhere in ["beta", "gamma", "delta"] {
            let secret = secret(elsewhere);
            let data = secret["data"].as_object().cloned().unwrap_or_default();
            for (key, encoded) in &data {
                let held = encoded
                    .as_str()
                    .and_then(pods::base64_decode)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_default();
                assert!(
                    held != value && held != oauth,
                    "{name}: the Secret of {elsewhere} holds the daemon's GitLab token under {key}"
                );
            }
        }
    }
}

/// Every variable a forge's command line reads a token from (decisions F3-D1, F4-D2): `GH_TOKEN`,
/// `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN` and `GITHUB_ENTERPRISE_TOKEN` for `gh`; `GITLAB_TOKEN`,
/// `GITLAB_ACCESS_TOKEN`, `OAUTH_TOKEN` and `CI_JOB_TOKEN` for `glab`. An instance with a forge
/// token of its own carries the daemon's entry of none of them (decision F3-D5), only its own token
/// in its forge's first variable, and its Secret none of their secrets; every other entry stays.
/// An instance on GitHub that names none keeps the daemon's entry of each `gh` variable (decision
/// F3-D3).
#[test]
fn an_own_forge_token_leaves_every_token_variable_of_both_forge_clis_out_of_its_pod() {
    let case = Case::new("forge-token-every-variable");
    let daemons: [(&str, &str); 8] = [
        ("GH_TOKEN", "forge-token"),
        ("GITHUB_TOKEN", "github-classic"),
        ("GH_ENTERPRISE_TOKEN", "github-enterprise"),
        ("GITHUB_ENTERPRISE_TOKEN", "github-enterprise-two"),
        ("GITLAB_TOKEN", "gitlab-forge"),
        ("GITLAB_ACCESS_TOKEN", "gitlab-access"),
        ("OAUTH_TOKEN", "gitlab-oauth"),
        ("CI_JOB_TOKEN", "gitlab-job"),
    ];
    let listed: Vec<&str> = conductor_cli::config::FORGE_CLI_TOKEN_VARIABLES
        .iter()
        .flat_map(|(_, variables)| variables.iter().copied())
        .collect();
    assert_eq!(
        listed,
        daemons.map(|(variable, _)| variable),
        "the daemon's forge token variables"
    );
    for (forge, first) in conductor_cli::config::FORGE_TOKEN_VARIABLES {
        assert!(
            conductor_cli::config::FORGE_CLI_TOKEN_VARIABLES
                .iter()
                .any(|(named, variables)| *named == forge && variables.first() == Some(&first)),
            "{forge}: the variable a forge_token fills is the first of its forge's list"
        );
    }
    let mut env: Vec<String> = daemons
        .iter()
        .map(|(variable, secret)| format!("{{name: {variable}, secret: secret:{secret}}}"))
        .collect();
    env.push("{name: GIT_AUTHOR_NAME, value: conductor}".to_owned());
    let env: Vec<&str> = env.iter().map(String::as_str).collect();
    case.config_env(
        IMAGE,
        &env,
        &[
            instance(
                "alpha",
                "{github: example-org}",
                "    forge_token: secret:alpha-forge\n",
            ),
            instance(
                "gamma",
                "{gitlab: example-org}",
                "    forge_token: secret:gamma-forge\n",
            ),
            instance("beta", "{github: example-org}", ""),
        ],
    );
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    for (index, (name, own_variable, own_secret)) in [
        ("alpha", "GH_TOKEN", "alpha-forge"),
        ("gamma", "GITLAB_TOKEN", "gamma-forge"),
    ]
    .into_iter()
    .enumerate()
    {
        let made = pods::manifests(&kubernetes, &config, &config.instances[index]);
        assert_eq!(
            variables(&made, own_variable),
            [from_secret(own_variable, name, own_secret)],
            "{name}: {}",
            made.pod
        );
        for (variable, secret) in daemons {
            if variable != own_variable {
                assert!(
                    variables(&made, variable).is_empty(),
                    "{name} carries the daemon's {variable}: {}",
                    made.pod
                );
            }
            assert!(!made.secrets.contains(secret), "{name}: {:?}", made.secrets);
        }
        assert_eq!(
            variables(&made, "GIT_AUTHOR_NAME"),
            [json!({"name": "GIT_AUTHOR_NAME", "value": "conductor"})],
            "{name}: {}",
            made.pod
        );
    }
    let beta = pods::manifests(&kubernetes, &config, &config.instances[2]);
    for (variable, secret) in &daemons[..4] {
        assert_eq!(
            variables(&beta, variable),
            [from_secret(variable, "beta", secret)],
            "{}",
            beta.pod
        );
    }
}

/// The daemon's forge token entries of each forge, the secret each names, and its value in the
/// case's secrets source: two of GitHub's, two of GitLab's.
const DAEMON_FORGES: [(&str, &str, &str, &str); 4] = [
    ("github", "GH_TOKEN", "forge-token", FORGE_TOKEN),
    (
        "github",
        "GITHUB_TOKEN",
        "github-classic",
        "github-classic-value-6666",
    ),
    (
        "gitlab",
        "GITLAB_TOKEN",
        "gitlab-forge",
        "gitlab-forge-value-7777",
    ),
    (
        "gitlab",
        "CI_JOB_TOKEN",
        "gitlab-job",
        "gitlab-job-value-8888",
    ),
];

/// One instance of the case below: its name, its sources, the forges whose daemon entries its pod
/// keeps, and git's helpers, each as an origin and its helper.
type ForgeCase<'a> = (&'a str, &'a str, &'a [&'a str], &'a [(&'a str, &'a str)]);

/// A pod of an instance without a forge token of its own carries the daemon's forge token entries
/// of a forge only where its instance has a source on that forge (decision F4-D1): GitHub's where
/// it has a `github` source; GitLab's, `CI_JOB_TOKEN` among them (decision F4-D2), where it has a
/// `gitlab` source and every `gitlab` source is on the daemon's GitLab host. An instance whose
/// sources are all `local` names no forge and keeps both forges' entries, as before. An entry left
/// out takes its secret out of the instance's Secret; every other entry stays; git gets a helper
/// for each forge whose token the pod carries and its instance has a source on.
#[test]
fn the_daemons_forge_tokens_reach_only_pods_of_instances_with_a_source_on_that_forge() {
    let case = Case::new("forge-per-source");
    for (_, _, secret, value) in DAEMON_FORGES {
        fs::write(case.dir.join("secrets").join(secret), value).expect("write a secret");
    }
    let mut env: Vec<String> = DAEMON_FORGES
        .iter()
        .map(|(_, variable, secret, _)| format!("{{name: {variable}, secret: secret:{secret}}}"))
        .collect();
    env.push("{name: GITLAB_HOST, value: gitlab.example.org}".to_owned());
    env.push("{name: GIT_AUTHOR_NAME, value: conductor}".to_owned());
    let env: Vec<&str> = env.iter().map(String::as_str).collect();
    let on_host = "{gitlab: example-org, host: gitlab.example.org}";
    let elsewhere = "{gitlab: example-org, host: gitlab.other.example}";
    let github = "{github: example-org}";
    let both_on_host = format!("{github}, {on_host}");
    let both_elsewhere = format!("{github}, {elsewhere}");
    let cases: [ForgeCase; 6] = [
        (
            "alpha",
            github,
            &["github"],
            &[("https://github.com", GITHUB_HELPER)],
        ),
        (
            "beta",
            on_host,
            &["gitlab"],
            &[("https://gitlab.example.org", GITLAB_HELPER)],
        ),
        ("gamma", elsewhere, &[], &[]),
        (
            "delta",
            &both_on_host,
            &["github", "gitlab"],
            &[
                ("https://github.com", GITHUB_HELPER),
                ("https://gitlab.example.org", GITLAB_HELPER),
            ],
        ),
        (
            "epsilon",
            &both_elsewhere,
            &["github"],
            &[("https://github.com", GITHUB_HELPER)],
        ),
        (
            "zeta",
            "{local: /var/lib/conductor/data/checkouts}",
            &["github", "gitlab"],
            &[],
        ),
    ];
    let instances: Vec<String> = cases
        .iter()
        .map(|(name, sources, _, _)| instance(name, sources, ""))
        .collect();
    case.config_env(IMAGE, &env, &instances);
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");

    for (index, (name, _, kept, origins)) in cases.iter().enumerate() {
        let made = pods::manifests(&kubernetes, &config, &config.instances[index]);
        for (forge, variable, secret, _) in DAEMON_FORGES {
            if kept.contains(&forge) {
                assert_eq!(
                    variables(&made, variable),
                    [from_secret(variable, name, secret)],
                    "{name} keeps the daemon's {variable}: {}",
                    made.pod
                );
                assert!(made.secrets.contains(secret), "{name}: {:?}", made.secrets);
            } else {
                assert!(
                    variables(&made, variable).is_empty(),
                    "{name} has no source on {forge} and carries the daemon's {variable}: {}",
                    made.pod
                );
                assert!(
                    !made.secrets.contains(secret),
                    "{name}: its Secret holds {secret}: {:?}",
                    made.secrets
                );
            }
        }
        for kept in ["GITLAB_HOST", "GIT_AUTHOR_NAME"] {
            assert_eq!(
                variables(&made, kept).len(),
                1,
                "{name}: every other entry stays: {}",
                made.pod
            );
        }
        let expected = if origins.is_empty() {
            Vec::new()
        } else {
            helpers(origins)
        };
        assert_eq!(git_config(&made), expected, "{name}: {}", made.pod);
    }

    // One pass: each Secret holds the daemon's secrets of the forges its pod keeps, no other.
    let mut daemon = case.daemon();
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert!(
        done.iter()
            .all(|row| row.result == ReconcileResult::Succeeded),
        "{done:?}"
    );
    for (name, _, kept, _) in cases {
        let secret = case
            .object("secrets", &format!("conductor-instance-{name}"))
            .unwrap_or_else(|| panic!("the Secret of {name}"));
        let data = secret["data"].as_object().cloned().unwrap_or_default();
        for (forge, _, key, value) in DAEMON_FORGES {
            if kept.contains(&forge) {
                assert_eq!(secret_value(&secret, key), value, "{name}");
            } else {
                assert!(
                    !data.contains_key(key),
                    "{name}: its Secret holds {key}: {:?}",
                    data.keys().collect::<Vec<_>>()
                );
            }
        }
    }
}

/// The pod `made` of a case is the one held in `tests/fixtures/k8s/<fixture>`, byte for byte:
/// its spec, its claim, its rendered file, the secrets its Secret holds and its digest; else what
/// differs, with what is made now written beside the case.
fn held_to(case: &Case, made: &pods::Manifests, fixture: &str) -> Option<String> {
    let made = json!({
        "pod": made.pod,
        "claim": made.claim,
        "rendered": made.rendered,
        "secrets": made.secrets,
        "digest": made.digest,
    });
    let made = format!(
        "{}\n",
        serde_json::to_string_pretty(&made).expect("the objects as JSON")
    );
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/k8s")
        .join(fixture);
    let before = fs::read_to_string(&path).unwrap_or_default();
    (made != before).then(|| {
        let now = case.dir.join(fixture);
        fs::write(&now, &made).expect("write what is made now");
        format!(
            "the pod differs from {}; what is made now is {}",
            path.display(),
            now.display()
        )
    })
}

/// A pod that git's credential helpers leave as it was (story:pod-git-reaches-forge), each held
/// byte for byte to the pod the supervisor made before them, written from the supervisor of
/// `cda5f4f0e` for the same file, and since then with `GLAB_SEND_TELEMETRY` after the git
/// identity and the secret volume's `items`: a `github` instance's pod that carries no forge token; the pod of an instance with a `local` source only, under a daemon-wide `GH_TOKEN`; and a `github`
/// instance's pod under a daemon-wide `GH_TOKEN` whose `env` names `GIT_CONFIG_COUNT`, where the
/// installer owns git's configuration.
#[test]
fn a_pod_without_a_token_of_its_forge_or_with_gits_config_named_is_the_one_made_before() {
    let render = |case: &Case| {
        let text = fs::read_to_string(case.file()).expect("the config file");
        let config = conductor_cli::config::parse(&text, &case.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        pods::manifests(&kubernetes, &config, &config.instances[0])
    };

    let case = Case::new("git-no-token");
    case.config_env(
        IMAGE,
        &["{name: GIT_AUTHOR_NAME, value: conductor}"],
        &[alpha("")],
    );
    let made = render(&case);
    assert!(git_config(&made).is_empty(), "{}", made.pod);
    let mut differ: Vec<String> = held_to(&case, &made, "pod-alpha-no-token.json")
        .into_iter()
        .collect();

    let case = Case::new("git-local");
    case.config(
        IMAGE,
        &[instance(
            "alpha",
            "{local: /var/lib/conductor/data/checkouts}",
            "    model_token: secret:alpha-model\n",
        )],
    );
    let made = render(&case);
    assert!(git_config(&made).is_empty(), "{}", made.pod);
    differ.extend(held_to(&case, &made, "pod-alpha-local.json"));

    let case = Case::new("git-config-named");
    case.config_env(
        IMAGE,
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GIT_CONFIG_COUNT, value: \"0\"}",
        ],
        &[alpha("")],
    );
    let made = render(&case);
    assert_eq!(
        git_config(&made),
        [("GIT_CONFIG_COUNT".to_owned(), "0".to_owned())],
        "{}",
        made.pod
    );
    differ.extend(held_to(&case, &made, "pod-alpha-git-config.json"));
    assert!(differ.is_empty(), "{}", differ.join("\n"));
}

/// The control token of the forge-token route case: the value of the secret `control`.
const CONTROL: &str = "control-token-0123456789abcdef0123456789abcdef";

/// `PUT /v1/instances/{name}` with the control token on `port`: the status and the body.
fn put(port: u16, name: &str, mapping: &Value) -> (u16, String) {
    let body = json!({"mapping": mapping, "reason": "a forge token of its own"}).to_string();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    write!(
        stream,
        "PUT /v1/instances/{name} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Authorization: Bearer {CONTROL}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

/// `PUT /v1/instances/{name}` that adds or changes the instance's `forge_token` replaces its pod
/// once, on a new digest, its variable from the newly named key; the same body again replaces
/// nothing and leaves the digest as it is.
#[test]
fn a_put_that_changes_the_forge_token_replaces_the_pod_once_and_the_same_body_nothing() {
    let case = Case::new("forge-token-put");
    for (name, value) in [
        ("control", CONTROL),
        ("alpha-forge", ALPHA_FORGE),
        ("alpha-forge-two", GAMMA_FORGE),
    ] {
        fs::write(case.dir.join("secrets").join(name), value).expect("write a secret");
    }
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();

    let api = Api::new("3".repeat(64), "operator".to_owned(), None)
        .with_config(
            case.file(),
            case.home(),
            format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
        )
        .with_control_token(
            conductor_model::config::SecretsSource {
                source: conductor_model::secret::Source::Files,
                namespace: None,
                url: None,
                dir: Some(case.dir.join("secrets").display().to_string()),
            },
            Some(conductor_model::secret::SecretName("control".to_owned())),
        )
        .with_pods(true);
    api.publish(daemon.rows(), daemon.routes(), Vec::new());
    api.publish_host(Vec::new(), None);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let api = Arc::new(api);
    std::thread::spawn(move || api::serve(&listener, &api));

    let mapping = |forge: &str| {
        json!({
            "name": "alpha",
            "placement": "none",
            "session_prefix": "alpha",
            "sources": [{"github": "example-org"}],
            "model_token": "secret:alpha-model",
            "forge_token": format!("secret:{forge}"),
        })
    };
    let digest = || case.pod()["metadata"]["annotations"]["conductor/digest"].clone();
    let mut at = 2;
    let mut pass = |daemon: &mut Daemon<Manager>| {
        at += 2;
        daemon
            .pass(
                ReconcileReason::ConfigChange,
                start + Duration::from_secs(at),
            )
            .expect("a pass")
    };

    let mut before = digest();
    for forge in ["alpha-forge", "alpha-forge-two"] {
        let (status, body) = put(port, "alpha", &mapping(forge));
        assert_eq!(status, 200, "{forge}: {body}");
        let done = pass(&mut daemon);
        assert_eq!(done.len(), 1, "{forge}: {done:?}");
        assert_eq!(
            done[0].action,
            ReconcileAction::Restart,
            "{forge}: {done:?}"
        );
        assert_eq!(
            done[0].result,
            ReconcileResult::Succeeded,
            "{forge}: {done:?}"
        );
        assert_ne!(digest(), before, "{forge}: the digest");
        let made = case.pod();
        let env = made["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
            .clone();
        let named: Vec<&Value> = env
            .iter()
            .filter(|entry| entry["name"] == "GH_TOKEN")
            .collect();
        assert_eq!(
            named,
            [&from_secret("GH_TOKEN", "alpha", forge)],
            "{forge}: {made}"
        );
        case.set_running();
        before = digest();

        let (status, body) = put(port, "alpha", &mapping(forge));
        assert_eq!(status, 200, "{forge} again: {body}");
        let done = pass(&mut daemon);
        assert!(done.is_empty(), "{forge} again: {done:?}");
        assert_eq!(digest(), before, "{forge} again: the digest");
    }
}

/// `PUT /v1/instances/{name}` that adds, changes or removes only the instance's `git_identity`
/// (story:instance-git-identity) replaces its pod once, on a new digest, with the identity's
/// variables; the same body again replaces nothing. `GET /v1/instances/{name}` answers the
/// member as the mapping holds it, and no `git_identity` for a mapping without it.
#[test]
fn a_put_that_changes_only_the_git_identity_replaces_the_pod_once_and_get_answers_it() {
    let case = Case::new("git-identity-put");
    fs::write(case.dir.join("secrets/control"), CONTROL).expect("write a secret");
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();

    let api = Api::new("3".repeat(64), "operator".to_owned(), None)
        .with_config(
            case.file(),
            case.home(),
            format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
        )
        .with_control_token(
            conductor_model::config::SecretsSource {
                source: conductor_model::secret::Source::Files,
                namespace: None,
                url: None,
                dir: Some(case.dir.join("secrets").display().to_string()),
            },
            Some(conductor_model::secret::SecretName("control".to_owned())),
        )
        .with_pods(true);
    api.publish(daemon.rows(), daemon.routes(), Vec::new());
    api.publish_host(Vec::new(), None);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let api = Arc::new(api);
    std::thread::spawn(move || api::serve(&listener, &api));

    let mapping = |identity: Option<(&str, &str)>| {
        let mut mapping = json!({
            "name": "alpha",
            "placement": "none",
            "session_prefix": "alpha",
            "sources": [{"github": "example-org"}],
            "model_token": "secret:alpha-model",
        });
        if let Some((name, email)) = identity {
            mapping["git_identity"] = json!({"name": name, "email": email});
        }
        mapping
    };
    let read = || {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .expect("a read timeout");
        write!(
            stream,
            "GET /v1/instances/alpha HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
             Authorization: Bearer {CONTROL}\r\n\r\n"
        )
        .expect("send");
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).expect("read");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_owned())
            .unwrap_or_default();
        assert!(text.starts_with("HTTP/1.1 200"), "{text}");
        serde_json::from_str::<Value>(&body).unwrap_or_else(|_| panic!("a JSON row: {body}"))
    };
    let digest = || case.pod()["metadata"]["annotations"]["conductor/digest"].clone();
    let mut at = 2;
    let mut pass = |daemon: &mut Daemon<Manager>| {
        at += 2;
        daemon
            .pass(
                ReconcileReason::ConfigChange,
                start + Duration::from_secs(at),
            )
            .expect("a pass")
    };

    let row = read();
    assert!(row.get("git_identity").is_none(), "{row}");
    let mut before = digest();
    for identity in [
        Some(("Example Bot", "bot@example.com")),
        Some(("Example Bot", "bot+ci@example.com")),
        None,
    ] {
        let (status, body) = put(port, "alpha", &mapping(identity));
        assert_eq!(status, 200, "{identity:?}: {body}");
        let row = read();
        match identity {
            Some((name, email)) => assert_eq!(
                row["git_identity"],
                json!({"name": name, "email": email}),
                "{row}"
            ),
            None => assert!(row.get("git_identity").is_none(), "{row}"),
        }
        let done = pass(&mut daemon);
        assert_eq!(done.len(), 1, "{identity:?}: {done:?}");
        assert_eq!(
            done[0].action,
            ReconcileAction::Restart,
            "{identity:?}: {done:?}"
        );
        assert_eq!(
            done[0].result,
            ReconcileResult::Succeeded,
            "{identity:?}: {done:?}"
        );
        assert_ne!(digest(), before, "{identity:?}: the digest");
        let made = case.pod();
        let email: Vec<Value> = made["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
            .iter()
            .filter(|entry| entry["name"] == "GIT_COMMITTER_EMAIL")
            .map(|entry| entry["value"].clone())
            .collect();
        assert_eq!(
            email,
            [json!(
                identity.map_or("conductor@localhost", |(_, email)| email)
            )],
            "{identity:?}: {made}"
        );
        case.set_running();
        before = digest();

        let (status, body) = put(port, "alpha", &mapping(identity));
        assert_eq!(status, 200, "{identity:?} again: {body}");
        let done = pass(&mut daemon);
        assert!(done.is_empty(), "{identity:?} again: {done:?}");
        assert_eq!(digest(), before, "{identity:?} again: the digest");
    }
}

/// A free loopback port.
fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .expect("a free port")
        .port()
}

/// `conductor instance serve` of `alpha`, alone as in a pod: on TCP at `port`, the loop off.
struct Runtime(Child);

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn http(port: u16, target: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    write!(
        stream,
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

/// The daemon reaches a pod-run runtime over TCP at its pod's address and `daemon.kubernetes
/// .api_port`, with the token of its Secret: a proxied read answers.
#[test]
fn a_proxied_read_reaches_the_pod_run_runtime_over_tcp() {
    let port = free_port();
    let case = Case::on_port("proxy", port);
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();
    daemon.tick(start + Duration::from_secs(2)).expect("a tick");

    // The runtime, as the pod runs it, on the loopback address the fake pod names.
    let root = case.dir.join("rt");
    for dir in [
        "records/dispatches",
        "records/decisions",
        "state",
        "home/watch",
    ] {
        fs::create_dir_all(root.join(dir)).expect("create a runtime directory");
    }
    fs::write(root.join("home/agents.json"), "[]").expect("write the session list");
    let file = root.join("conductor.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: none\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = format!("sqlite:{}", root.join("rt.sqlite").display()),
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        ),
    )
    .expect("write the runtime's config");
    let runtime = Runtime(
        common::conductor(root.join("home"))
            .current_dir(root.join("state"))
            .arg("--config")
            .arg(&file)
            .args(["instance", "serve"])
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(conductor_cli::config::INSTANCE_VARIABLE, "alpha")
            .env(conductor_cli::instance::TOKEN_VARIABLE, case.token())
            .env(
                conductor_cli::instance::LISTEN_VARIABLE,
                format!("127.0.0.1:{port}"),
            )
            .env_remove(conductor_cli::instance::CONFIG_SOURCE_VARIABLE)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("instance serve starts"),
    );

    let api = Arc::new(Api::new("2".repeat(64), "owner".to_owned(), None));
    api.publish(daemon.rows(), daemon.routes(), Vec::new());
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let daemon_port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&api);
    std::thread::spawn(move || api::serve(&listener, &served));

    let until = Instant::now() + Duration::from_secs(30);
    let (status, body) = loop {
        let (status, body) = http(daemon_port, "/v1/instances/alpha/settings");
        if status == 200 || Instant::now() > until {
            break (status, body);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(status, 200, "{body}");
    let settings: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(settings["instance"], "alpha", "{body}");
    // The work call's read reaches it too (story:control-plane-work-handler).
    let (status, body) = http(daemon_port, "/v1/instances/alpha/work/WI-1");
    assert_eq!(status, 404, "{body}");
    let refused: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(
        refused["error"], "conductor.dispatch.WorkNotFound",
        "{body}"
    );
    drop(runtime);
}
