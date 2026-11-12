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
        let list = if instances.is_empty() {
            " []\n".to_owned()
        } else {
            format!("\n{}", instances.concat())
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
             \x20   env:\n\
             \x20     - {{name: GH_TOKEN, secret: secret:forge-token}}\n\
             \x20     - {{name: GIT_AUTHOR_NAME, value: conductor}}\n\
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
    drop(runtime);
}
