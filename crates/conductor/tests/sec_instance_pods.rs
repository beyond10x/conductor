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

/// An instance's `forge_token` (story:instance-forge-token) that names one of the daemon's own
/// secrets reaches no pod: a file that writes it does not validate, its YAML path named with "a
/// secret of the daemon's own" and not the daemon's key; a mapping that sends it is refused `422`
/// at that path, the file unchanged; and a config read before that rule gets the pod supervisor
/// to refuse the start before any secret is resolved, nothing created.
#[test]
fn a_forge_token_naming_a_secret_of_the_daemons_own_reaches_no_pod() {
    let case = Case::new("forge-token-own");
    let beta = |forge: &str| {
        format!(
            "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
             sources: [{{github: example-org}}]\n    forge_token: secret:{forge}"
        )
    };
    let problems = conductor_cli::config::parse(&case.daemon_text(&beta("control")), &case.home())
        .expect_err("a forge_token naming the control token does not validate");
    let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert!(
        problems
            .iter()
            .any(|problem| problem.path == "instances[0].forge_token"
                && problem.message.contains("a secret of the daemon's own")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|line| line.contains("control_token")),
        "{said:?}"
    );

    case.write(" []");
    let before = case.text();
    let port = case.serve();
    let sent = mapping("beta", &json!({"forge_token": "secret:control"}));
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 422, "{body}");
    let answer: Value = serde_json::from_str(&body).expect("a JSON body");
    assert_eq!(answer["path"], "instances[0].forge_token", "{answer}");
    assert_eq!(case.text(), before, "the file changed");

    let fake = FakeKube::start(&case.dir);
    let text = case.daemon_text(&beta("beta-forge"));
    let valid = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let mut config = valid.clone();
    config.daemon.control_token = Some(SecretName("beta-forge".to_owned()));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let mut pods = Pods::new(fake.client());
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
        "{error}"
    );
    assert!(asked.borrow().is_empty(), "resolved {:?}", asked.borrow());
    assert!(fake.requests().is_empty(), "{:?}", fake.requests());
}

/// Decision D8: what of a mapping reaches the pod or the claim is the instance's name; the one
/// variable its `forge_token` fills, by the key that entry reads and never by a variable's name
/// (story:instance-forge-token); and, through the kinds and the `gitlab` hosts of its sources,
/// which of the daemon's forge token entries the pod keeps and which
/// `credential.https://<host>.helper` keys git gets (story:pod-git-reaches-forge, decision F3-D2).
/// No text of a caller reaches a variable's name or a helper's value. Two mappings of one name
/// whose sources are of one kind, on GitHub, and that differ in every other key the instance pod
/// takes make the same pod and the same claim, but for the digest annotation of the rendered
/// file and the secret volume's `items`, which list the keys of the pod's Secret: the
/// names of the secrets it gets, as validation takes them, and nothing else.
/// `tests/daemon_pods.rs` holds the forge token and the sources' share.
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
        // The secret volume lists the keys of the pod's Secret, which are the names of the
        // secrets the pod gets (`made.secrets`) and nothing else of the mapping: held here, then
        // left out of the comparison.
        let volumes = made.pod["spec"]["volumes"].as_array_mut().expect("volumes");
        let secret = volumes
            .iter_mut()
            .find(|volume| volume["name"] == "secret")
            .expect("the secret volume");
        let items = secret["secret"]
            .as_object_mut()
            .and_then(|volume| volume.remove("items"))
            .unwrap_or_default();
        let wanted: Vec<Value> = std::iter::once("conductor.yaml")
            .chain(made.secrets.iter().map(String::as_str))
            .map(|key| json!({"key": key, "path": key}))
            .collect();
        assert_eq!(items, json!(wanted), "{text}");
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

/// git's credential helper of a `gitlab` origin (story:pod-git-reaches-forge): fixed text, no
/// caller's.
const GITLAB_HELPER: &str =
    r#"!f() { test "$1" = get && printf 'username=%s\npassword=%s\n' oauth2 "$GITLAB_TOKEN"; }; f"#;

/// The one text of a mapping that reaches git's configuration in a pod is a `gitlab` source's
/// `host` (story:pod-git-reaches-forge), and only in the key of a credential helper,
/// `credential.https://<host>.helper`, never in the helper's value, which the shell runs. The
/// instance API refuses every host that could end that key, add a scheme, user or path to the
/// origin, widen it to other hosts, or reach a shell, before anything is written; a host it
/// takes appears in the key as a host name with an optional port, lowercased, and the value is
/// the fixed helper whatever the host.
#[test]
fn a_gitlab_host_reaches_only_the_key_of_gits_credential_helper() {
    let case = Case::new("git-helper-host");
    case.write(" []");
    let before = case.text();
    let port = case.serve();
    let source = |host: &str| json!({"sources": [{"gitlab": "example-org", "host": host}]});
    for host in [
        "gitlab.example.com\n[core]",
        "gitlab.example.com\nhelper",
        "gitlab.example.com;id",
        "$(id).example.com",
        "`id`.example.com",
        "gitlab example.com",
        "gitlab.example.com/group",
        "user@gitlab.example.com",
        "gitlab.example.com#x",
        "gitlab.example.com?x",
        "gitlab.example.com=x",
        "gitlab.example.com'",
        "gitlab.example.com\"",
        "gitlab.example.com\\",
        "*.example.com",
        ".example.com",
        "https://gitlab.example.com",
        "gitlab.example.com:8443:1",
        "gitlab.example.com:",
        "gitlab.example.com:0",
        "[::1]",
        "gitlab.example.com\u{0}",
    ] {
        let body = json!({"mapping": mapping("beta", &source(host)), "reason": "r"});
        let (status, answer) = exchange(port, &request(port, "POST", "/v1/instances", Some(&body)));
        assert!(
            (400..500).contains(&status) && status != 401 && status != 403,
            "host {host:?}: answered {status}, not refused as invalid: {answer}"
        );
        assert_eq!(case.text(), before, "host {host:?}: the file changed");
    }
    // The same mapping with a host name and a port is taken: the refusals above are the host's.
    let body = json!({
        "mapping": mapping("beta", &source("gitlab.example.com:8443")),
        "reason": "r",
    });
    let (status, answer) = exchange(port, &request(port, "POST", "/v1/instances", Some(&body)));
    assert!((200..300).contains(&status), "{status}: {answer}");

    for (host, origin) in [
        ("gitlab.example.com", "https://gitlab.example.com"),
        ("GitLab.Example.COM:8443", "https://gitlab.example.com:8443"),
        ("a-b.c1.example", "https://a-b.c1.example"),
    ] {
        let text = case.daemon_text(&format!(
            "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
             sources: [{{gitlab: example-org, host: {host:?}}}]\n    \
             forge_token: secret:beta-forge"
        ));
        let config = conductor_cli::config::parse(&text, &case.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
        let git: Vec<(String, String)> = made.pod["spec"]["containers"][0]["env"]
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
            .collect();
        assert_eq!(
            git,
            [
                ("GIT_CONFIG_COUNT".to_owned(), "1".to_owned()),
                (
                    "GIT_CONFIG_KEY_0".to_owned(),
                    format!("credential.{origin}.helper")
                ),
                ("GIT_CONFIG_VALUE_0".to_owned(), GITLAB_HELPER.to_owned()),
            ],
            "host {host:?}: {}",
            made.pod
        );
    }
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

/// An instance's `git_identity` (story:instance-git-identity) is text of a caller that reaches
/// the pod: only as the values of `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`
/// and `GIT_COMMITTER_EMAIL`, never a variable's name, the rendered file, the Secret's keys or the
/// claim. A value that is not one line, holds a control character, `<` or `>`, or an e-mail
/// without exactly one `@` is refused `422` `conductor.config.ConfigInvalid` at its YAML path by
/// `POST /v1/instances` and by `PUT /v1/instances/{name}`, the file unchanged.
#[test]
fn a_git_identity_reaches_only_the_values_of_the_four_variables_and_is_refused_at_the_routes() {
    let case = Case::new("git-identity");
    case.write(" []");
    let port = case.serve();
    let shell = "Example Bot $(id) `id` ; & | \\ \" '";
    let identity = json!({"name": shell, "email": "bot+ci@example.com"});
    let sent = mapping("beta", &json!({"git_identity": identity}));
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 201, "{body}");

    let text = case.text();
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    let mut carried: Vec<(String, String)> = pod_env(&made.pod)
        .iter()
        .filter(|entry| {
            let value = entry["value"].as_str().unwrap_or_default();
            value.contains("Example Bot") || value.contains("bot+ci")
        })
        .map(|entry| {
            (
                entry["name"].as_str().unwrap_or_default().to_owned(),
                entry["value"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    carried.sort();
    assert_eq!(
        carried,
        [
            (
                "GIT_AUTHOR_EMAIL".to_owned(),
                "bot+ci@example.com".to_owned()
            ),
            ("GIT_AUTHOR_NAME".to_owned(), shell.to_owned()),
            (
                "GIT_COMMITTER_EMAIL".to_owned(),
                "bot+ci@example.com".to_owned()
            ),
            ("GIT_COMMITTER_NAME".to_owned(), shell.to_owned()),
        ],
        "{}",
        made.pod
    );
    let mut pod = made.pod.clone();
    if let Some(env) = pod["spec"]["containers"][0]["env"].as_array_mut() {
        env.retain(|entry| {
            !entry["name"]
                .as_str()
                .unwrap_or_default()
                .starts_with("GIT_")
        });
    }
    for (what, text) in [
        ("the pod beside the four values", pod.to_string()),
        ("the claim", made.claim.to_string()),
        ("the rendered file", made.rendered.clone()),
        ("the Secret's keys", format!("{:?}", made.secrets)),
    ] {
        assert!(
            !text.contains("Example Bot") && !text.contains("bot+ci"),
            "{what}: {text}"
        );
    }

    let before = case.text();
    for (what, identity, path) in [
        (
            "a newline",
            json!({"name": "Example\nBot", "email": "bot@example.com"}),
            "instances[0].git_identity.name",
        ),
        (
            "a carriage return",
            json!({"name": "Example Bot", "email": "bot@example.com\r"}),
            "instances[0].git_identity.email",
        ),
        (
            "a NUL",
            json!({"name": "Example\u{0}Bot", "email": "bot@example.com"}),
            "instances[0].git_identity.name",
        ),
        (
            "an angle bracket",
            json!({"name": "Example Bot", "email": "bot@example.com> <x@example.com"}),
            "instances[0].git_identity.email",
        ),
        (
            "two ats",
            json!({"name": "Example Bot", "email": "bot@x@example.com"}),
            "instances[0].git_identity.email",
        ),
    ] {
        let sent = mapping(
            "gamma",
            &json!({"session_prefix": "gamma", "git_identity": identity}),
        );
        let sent_put = mapping("beta", &json!({"git_identity": identity}));
        for (method, target, body, path) in [
            (
                "POST",
                "/v1/instances",
                json!({"mapping": sent, "reason": "r"}),
                path.replace("instances[0]", "instances[1]"),
            ),
            (
                "PUT",
                "/v1/instances/beta",
                json!({"mapping": sent_put, "reason": "r"}),
                path.to_owned(),
            ),
        ] {
            let (status, answer) = exchange(port, &request(port, method, target, Some(&body)));
            assert_eq!(status, 422, "{method} {what}: {answer}");
            let answer: Value = serde_json::from_str(&answer).expect("a JSON body");
            assert_eq!(
                answer["error"], "conductor.config.ConfigInvalid",
                "{method} {what}: {answer}"
            );
            assert_eq!(answer["path"], json!(path), "{method} {what}: {answer}");
            assert_eq!(case.text(), before, "{method} {what}: the file changed");
        }
    }
}

/// An instance's `model_token_kind` takes the two kinds the specification declares: a kind it
/// does not declare, and a kind beside no `model_token`, are refused `422`
/// `conductor.config.ConfigInvalid` at `instances[<i>].model_token_kind` by `POST /v1/instances`
/// and by `PUT /v1/instances/{name}`, the file unchanged; a declared kind is taken.
#[test]
fn a_model_token_kind_the_specification_does_not_declare_is_refused_at_the_routes() {
    let case = Case::new("model-token-kind");
    case.write(" []");
    let port = case.serve();
    let sent = mapping(
        "beta",
        &json!({"model_token": "secret:alpha-model", "model_token_kind": "subscription_token"}),
    );
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 201, "{body}");
    assert!(
        case.text().contains("model_token_kind: subscription_token"),
        "{}",
        case.text()
    );

    let before = case.text();
    for (what, extra) in [
        (
            "an undeclared kind",
            json!({"model_token": "secret:alpha-model", "model_token_kind": "oauth_token"}),
        ),
        (
            "a kind of no model_token",
            json!({"model_token_kind": "api_key"}),
        ),
    ] {
        let mut gamma = extra.clone();
        gamma["session_prefix"] = json!("gamma");
        for (method, target, body, path) in [
            (
                "POST",
                "/v1/instances",
                json!({"mapping": mapping("gamma", &gamma), "reason": "r"}),
                "instances[1].model_token_kind",
            ),
            (
                "PUT",
                "/v1/instances/beta",
                json!({"mapping": mapping("beta", &extra), "reason": "r"}),
                "instances[0].model_token_kind",
            ),
        ] {
            let (status, answer) = exchange(port, &request(port, method, target, Some(&body)));
            assert_eq!(status, 422, "{method} {what}: {answer}");
            let answer: Value = serde_json::from_str(&answer).expect("a JSON body");
            assert_eq!(
                answer["error"], "conductor.config.ConfigInvalid",
                "{method} {what}: {answer}"
            );
            assert_eq!(answer["path"], json!(path), "{method} {what}: {answer}");
            assert_eq!(case.text(), before, "{method} {what}: the file changed");
        }
    }
}

/// An instance's `standing_rules` (story:instance-rules-text) is text of a caller that reaches
/// the pod in its rendered file only: not the pod's spec, not a variable, not the claim, and never
/// as a secret: a sentence that holds the word `secret:<name>`, here naming the daemon's own
/// control token, which no pod gets, names no secret ([`conductor_cli::secret::referenced`]), and
/// the pod's Secret has no key beyond the one of the same instance without rules. Above 16384
/// bytes of UTF-8, with a NUL, or with a whole text, white space aside, that reads as a secret
/// reference, it is refused `422` `conductor.config.ConfigInvalid` at its YAML path by
/// `POST /v1/instances` and by `PUT /v1/instances/{name}`, the file unchanged and no part of the
/// text in the answer.
#[test]
fn standing_rules_reach_the_rendered_file_only_and_are_refused_above_the_bound_at_the_routes() {
    let case = Case::new("standing-rules");
    case.write(" []");
    let port = case.serve();
    let shell = "Rule: $(id) `id` ; & | \\ \" ' GH_TOKEN=x\n";
    let sentence = "Never print the value of secret:control into a log.\n";
    for (name, prefix, text) in [("beta", "beta", shell), ("delta", "delta", sentence)] {
        let sent = mapping(
            name,
            &json!({"session_prefix": prefix, "standing_rules": text}),
        );
        let (status, body) = exchange(
            port,
            &request(
                port,
                "POST",
                "/v1/instances",
                Some(&json!({"mapping": sent, "reason": "a tenant"})),
            ),
        );
        assert_eq!(status, 201, "{name}: {body}");
    }

    let text = case.text();
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let referenced = conductor_cli::secret::referenced(&text);
    for (index, written) in [(0, shell), (1, sentence)] {
        let instance = &config.instances[index];
        assert_eq!(instance.standing_rules.as_deref(), Some(written));
        assert!(
            !referenced
                .get(&instance.name.0)
                .is_some_and(|names| names.contains("control")),
            "{referenced:?}"
        );
        let made = pods::manifests(&kubernetes, &config, instance);
        let without = conductor_model::config::Instance {
            standing_rules: None,
            ..instance.clone()
        };
        assert_eq!(
            made.secrets,
            pods::manifests(&kubernetes, &config, &without).secrets,
            "the Secret's keys with the rules are those without"
        );
        let rendered: serde_yaml::Value =
            serde_yaml::from_str(&made.rendered).expect("the rendered file");
        assert_eq!(
            rendered["instances"][0]["standing_rules"].as_str(),
            Some(written),
            "{}",
            made.rendered
        );
        for (what, held) in [
            ("the pod", made.pod.to_string()),
            ("the claim", made.claim.to_string()),
        ] {
            assert!(
                !held.contains("$(id)") && !held.contains("secret:control"),
                "{what}: {held}"
            );
        }
        assert!(
            !made.secrets.contains("control"),
            "the Secret's keys: {:?}",
            made.secrets
        );
    }

    let before = case.text();
    for (what, rules, says) in [
        ("16385 bytes", "a".repeat(16385), "is 16385 bytes"),
        ("a NUL", "one rule\0and another".to_owned(), "a NUL"),
        (
            "a secret reference",
            "secret:control".to_owned(),
            "no secret reference",
        ),
        (
            "a secret reference in white space",
            "  secret:alpha-model\n".to_owned(),
            "no secret reference",
        ),
    ] {
        let sent = mapping(
            "gamma",
            &json!({"session_prefix": "gamma", "standing_rules": rules}),
        );
        let sent_put = mapping("beta", &json!({"standing_rules": rules}));
        for (method, target, body, path) in [
            (
                "POST",
                "/v1/instances",
                json!({"mapping": sent, "reason": "r"}),
                "instances[2].standing_rules",
            ),
            (
                "PUT",
                "/v1/instances/beta",
                json!({"mapping": sent_put, "reason": "r"}),
                "instances[0].standing_rules",
            ),
        ] {
            let (status, answer) = exchange(port, &request(port, method, target, Some(&body)));
            assert_eq!(status, 422, "{method} {what}: {answer}");
            let answer: Value = serde_json::from_str(&answer).expect("a JSON body");
            assert_eq!(
                answer["error"], "conductor.config.ConfigInvalid",
                "{method} {what}: {answer}"
            );
            assert_eq!(answer["path"], json!(path), "{method} {what}: {answer}");
            assert!(
                answer.to_string().contains(says),
                "{method} {what}: {answer}"
            );
            for part in ["secret:control", "alpha-model", "one rule"] {
                assert!(
                    !answer.to_string().contains(part),
                    "{method} {what}: the answer quotes the text: {answer}"
                );
            }
            assert_eq!(case.text(), before, "{method} {what}: the file changed");
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
/// failed pod's log, an adoption, a change of the standing rules alone, a removal and a sweep
/// together use every one of them.
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

    // A change of the instance's standing rules alone: its Secret written again in place
    // (story:instance-rules-text).
    case.write(&format!("{alpha}\n    standing_rules: \"One rule.\""));
    daemon
        .tick(Instant::now() + Duration::from_secs(15))
        .expect("a tick");

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
            ("PUT", ["api", "v1", "namespaces", _, resource, _]) => ("update", *resource),
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

/// The `env` entries of `pod`'s one container.
fn pod_env(pod: &Value) -> Vec<Value> {
    pod["spec"]["containers"][0]["env"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// The control plane chooses a `gitlab` source's `host`. Under a daemon whose `env` names the
/// operator's daemon-wide `GITLAB_TOKEN`, the operator's token must not get a git
/// credential helper for a host the control plane named and the operator did not: otherwise the
/// caller directs the operator's token to a server of its choosing with every HTTPS fetch the
/// runtime or a session in the pod makes (story:pod-git-reaches-forge).
#[test]
fn the_daemons_gitlab_token_gets_no_git_helper_for_a_host_the_caller_names() {
    let case = Case::new("daemon-token-caller-host");
    let text = case.daemon_text(" []").replace(
        "{name: GH_TOKEN, secret: secret:forge-token}",
        "{name: GITLAB_TOKEN, secret: secret:forge-token}",
    );
    fs::write(case.file(), &text).expect("write the config file");
    let port = case.serve();
    let chosen = "forge.elsewhere.example";
    let sent = json!({
        "name": "beta",
        "placement": "none",
        "session_prefix": "beta",
        "sources": [{"gitlab": "example-org", "host": chosen}],
    });
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 201, "the control plane's mapping was taken: {body}");
    let text = case.text();
    let config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    let env = pod_env(&made.pod);
    // The review wrote this case with the pod carrying the daemon-wide GITLAB_TOKEN and asked
    // that git get no helper for the caller's host. The fix goes further: the variable itself
    // stays out of such a pod, which also closes `glab --hostname <host>`.
    assert!(
        !env.iter().any(|entry| entry["name"] == "GITLAB_TOKEN"
            || entry["valueFrom"]["secretKeyRef"]["key"] == "forge-token"),
        "a pod whose gitlab source is on a host the daemon's GitLab token does not belong to \
         carries neither GITLAB_TOKEN nor the secret forge-token: {}",
        made.pod
    );
    assert!(
        !made.secrets.contains("forge-token"),
        "the instance's Secret holds the daemon's GitLab token: {:?}",
        made.secrets
    );
    let wired: Vec<&str> = env
        .iter()
        .filter_map(|entry| {
            let name = entry["name"].as_str()?;
            let value = entry["value"].as_str()?;
            (name.starts_with("GIT_CONFIG_KEY_") && value.contains(chosen)).then_some(value)
        })
        .collect();
    assert!(
        wired.is_empty(),
        "git offers the operator's daemon-wide GITLAB_TOKEN (secret forge-token) to {chosen}, a \
         host only the control plane named: {wired:?}"
    );
}

/// The store's password, the other secret of the daemon's own, named as an instance's
/// `forge_token`: a file that writes it does not validate, its path named with "a secret of the
/// daemon's own" and not the daemon's key; and a config that slipped past that rule gets the pod
/// supervisor to refuse the start before any secret is resolved, nothing created.
#[test]
fn a_forge_token_naming_the_stores_password_reaches_no_pod() {
    let case = Case::new("forge-token-store");
    let sqlite = format!("store: {:?}", case.store());
    let password = ["secret:", "store", "-password"].concat();
    let postgres = format!("store: \"postgres://conductor:{password}@db.example.com/conductor\"");
    let beta = |forge: &str| {
        format!(
            "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
             sources: [{{github: example-org}}]\n    forge_token: {forge}"
        )
    };
    let text = case
        .daemon_text(&beta(&password))
        .replace(&sqlite, &postgres);
    assert!(text.contains("postgres://"), "{text}");
    let problems = conductor_cli::config::parse(&text, &case.home())
        .expect_err("a forge_token naming the store's password does not validate");
    let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert!(
        problems
            .iter()
            .any(|problem| problem.path == "instances[0].forge_token"
                && problem.message.contains("a secret of the daemon's own")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|line| line.contains("daemon.store")),
        "{said:?}"
    );

    let fake = FakeKube::start(&case.dir);
    let text = case.daemon_text(&beta("secret:beta-forge"));
    let mut config = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    config.daemon.store = conductor_model::store::StoreUrl(
        "postgres://conductor:secret:beta-forge@db.example.com/conductor".to_owned(),
    );
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let mut pods = Pods::new(fake.client());
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
        "{error}"
    );
    assert!(asked.borrow().is_empty(), "resolved {:?}", asked.borrow());
    assert!(fake.requests().is_empty(), "{:?}", fake.requests());
}

/// An instance's own forge token, through the whole daemon: its value is in the pod's Secret
/// under the secret's name and nowhere else the daemon writes (the pod, its labels and
/// annotations, the claim, the rendered file in the Secret, the config file, the API's answer);
/// and that Secret holds the daemon-wide forge token under no key.
#[test]
fn an_own_forge_tokens_value_is_only_in_its_pods_secret() {
    let case = Case::new("forge-token-value");
    let value = ["beta-forge", "-value-", "2222"].concat();
    fs::write(case.dir.join("secrets/beta-forge"), &value).expect("write a secret");
    case.write(" []");
    let fake = FakeKube::start(&case.dir);
    let port = case.serve();
    let sent = mapping("beta", &json!({"forge_token": "secret:beta-forge"}));
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 201, "{body}");
    assert!(!body.contains(&value), "{body}");
    assert!(!case.text().contains(&value), "{}", case.text());

    let mut daemon = case.daemon(&fake, "daemon");
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert!(!format!("{done:?}").contains(&value), "{done:?}");
    let pod = fake
        .object(NAMESPACE, "pods", "conductor-instance-beta")
        .expect("the pod");
    let claim = fake
        .object(
            NAMESPACE,
            "persistentvolumeclaims",
            "conductor-instance-beta-data",
        )
        .map(|claim| claim.to_string())
        .unwrap_or_default();
    for (what, text) in [("pod", pod.to_string()), ("claim", claim)] {
        assert!(!text.contains(&value), "{what}: {text}");
    }
    let secret = fake
        .object(NAMESPACE, "secrets", "conductor-instance-beta")
        .expect("the Secret");
    let decoded: BTreeMap<String, String> = secret["data"]
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|(key, data)| {
            let text = data
                .as_str()
                .and_then(pods::base64_decode)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            (key, text)
        })
        .collect();
    assert_eq!(
        decoded.get("beta-forge"),
        Some(&value),
        "{:?}",
        decoded.keys()
    );
    for (key, text) in &decoded {
        assert!(
            key == "beta-forge" || !text.contains(&value),
            "the key {key} holds the instance's forge token"
        );
        assert!(
            !text.contains(FORGE_TOKEN),
            "the key {key} holds the daemon-wide forge token"
        );
    }
    let entries: Vec<Value> = pod_env(&pod)
        .into_iter()
        .filter(|entry| entry["name"] == "GH_TOKEN")
        .collect();
    assert_eq!(
        entries,
        [json!({
            "name": "GH_TOKEN",
            "valueFrom": {"secretKeyRef": {"name": "conductor-instance-beta", "key": "beta-forge"}},
        })],
        "{pod}"
    );
}

/// What a refusal of the secret of a daemon's forge token says (decision F3-D4).
const DAEMONS_FORGE_SECRET: &str = "the secret of a daemon's forge token";

/// The secret a daemon forge-token entry names (decision F3-D4: an entry of
/// `daemon.kubernetes.env` named for a variable a forge's command line reads a token from) is the
/// daemon's to hand out, to the pods it belongs to. Without that rule a caller names it as its
/// own `forge_token` beside a `gitlab` host of its choice and gets the operator's token sent
/// there. So no instance and no `globals` key names it, under `forge_token`, `model_token` or any
/// other key: a file that does is refused at the reference's YAML path, for every such variable;
/// a mapping that does is refused `422` at that path, the file unchanged; and a config read
/// before that rule gets the pod supervisor to refuse the start before any secret is resolved,
/// nothing created.
#[test]
fn the_secret_of_a_daemons_forge_token_is_no_instances_and_reaches_no_pod_as_one() {
    let case = Case::new("daemons-forge-secret");
    let chosen = "forge.elsewhere.example";
    let beta = |key: &str, secret: &str| {
        format!(
            "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
             sources: [{{gitlab: example-org, host: {chosen}}}]\n    {key}: secret:{secret}"
        )
    };
    let refused_at = |text: &str, path: &str| {
        let problems = conductor_cli::config::parse(text, &case.home())
            .expect_err("a reference to the daemon's forge secret does not validate");
        let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
        assert!(
            problems.iter().any(|problem| problem.path == path
                && problem.message.contains(DAEMONS_FORGE_SECRET)
                && problem.message.contains("pods it belongs to")),
            "{path}: {said:?}\n{text}"
        );
    };
    for variable in [
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GH_ENTERPRISE_TOKEN",
        "GITHUB_ENTERPRISE_TOKEN",
        "GITLAB_TOKEN",
        "GITLAB_ACCESS_TOKEN",
        "OAUTH_TOKEN",
    ] {
        for key in ["forge_token", "model_token"] {
            let text = case.daemon_text(&beta(key, "forge-token")).replace(
                "{name: GH_TOKEN, secret: secret:forge-token}",
                &format!("{{name: {variable}, secret: secret:forge-token}}"),
            );
            refused_at(&text, &format!("instances[0].{key}"));
        }
    }
    let globals = case
        .daemon_text(&beta("model_token", "beta-model"))
        .replace(
            "instances:",
            "globals: {model_token: secret:forge-token}\ninstances:",
        );
    refused_at(&globals, "globals.model_token");

    case.write(" []");
    let before = case.text();
    let port = case.serve();
    for key in ["forge_token", "model_token"] {
        let sent = json!({
            "name": "beta",
            "placement": "none",
            "session_prefix": "beta",
            "sources": [{"gitlab": "example-org", "host": chosen}],
            key: "secret:forge-token",
        });
        let (status, body) = exchange(
            port,
            &request(
                port,
                "POST",
                "/v1/instances",
                Some(&json!({"mapping": sent, "reason": "a tenant"})),
            ),
        );
        assert_eq!(status, 422, "{key}: {body}");
        let answer: Value = serde_json::from_str(&body).expect("a JSON body");
        assert_eq!(answer["path"], format!("instances[0].{key}"), "{answer}");
        assert!(
            answer["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains(DAEMONS_FORGE_SECRET)),
            "{answer}"
        );
        assert_eq!(case.text(), before, "{key}: the file changed");
    }

    let fake = FakeKube::start(&case.dir);
    let text = case.daemon_text(&beta("forge_token", "beta-forge"));
    let valid = conductor_cli::config::parse(&text, &case.home())
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    for key in ["forge_token", "model_token"] {
        let mut config = valid.clone();
        let daemons = Some(SecretName("forge-token".to_owned()));
        if key == "forge_token" {
            config.instances[0].forge_token = daemons;
        } else {
            config.instances[0].model_token = daemons;
        }
        let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
        let mut pods = Pods::new(fake.client());
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
            error.contains(DAEMONS_FORGE_SECRET) && error.contains("nothing is created"),
            "{key}: {error}"
        );
        assert!(
            asked.borrow().is_empty(),
            "{key}: resolved {:?}",
            asked.borrow()
        );
        assert!(fake.requests().is_empty(), "{key}: {:?}", fake.requests());
    }
}

/// A pod that got the daemon's GitLab token while its instance's `gitlab` source was on the
/// daemon's GitLab host loses it when a `PUT` moves that source to a host the control plane
/// names: the next pass replaces the pod, and the new pod and its Secret hold neither the
/// variable nor the secret, and git has no helper for the new host.
#[test]
fn a_put_that_moves_a_gitlab_source_off_the_daemons_host_replaces_the_pod_without_its_token() {
    let case = Case::new("gitlab-host-put");
    let beta = "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
                sources: [{gitlab: example-org}]";
    let text = case.daemon_text(beta).replace(
        "{name: GH_TOKEN, secret: secret:forge-token}",
        "{name: GITLAB_TOKEN, secret: secret:forge-token}",
    );
    fs::write(case.file(), &text).expect("write the config file");
    let fake = FakeKube::start(&case.dir);
    let mut daemon = case.daemon(&fake, "daemon");
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    let carries = |pod: &Value| {
        pod_env(pod)
            .iter()
            .any(|entry| entry["name"] == "GITLAB_TOKEN")
    };
    let mut pod = fake
        .object(NAMESPACE, "pods", "conductor-instance-beta")
        .expect("the pod");
    assert!(
        carries(&pod),
        "precondition: on gitlab.com it carries the token: {pod}"
    );
    pod["status"] = json!({
        "phase": "Running",
        "podIP": "127.0.0.1",
        "containerStatuses": [
            {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
        ],
    });
    fake.put(NAMESPACE, "pods", pod);

    let port = case.serve();
    let chosen = "forge.elsewhere.example";
    let sent = json!({
        "name": "beta",
        "placement": "none",
        "session_prefix": "beta",
        "sources": [{"gitlab": "example-org", "host": chosen}],
    });
    let (status, body) = exchange(
        port,
        &request(
            port,
            "PUT",
            "/v1/instances/beta",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 200, "{body}");
    daemon
        .pass(
            ReconcileReason::ConfigChange,
            start + Duration::from_secs(4),
        )
        .expect("a pass");
    let pod = fake
        .object(NAMESPACE, "pods", "conductor-instance-beta")
        .expect("the pod");
    assert!(
        !carries(&pod),
        "the replaced pod carries GITLAB_TOKEN: {pod}"
    );
    assert!(
        !pod.to_string().contains(chosen),
        "git has a helper for {chosen}: {pod}"
    );
    let secret = fake
        .object(NAMESPACE, "secrets", "conductor-instance-beta")
        .expect("the Secret");
    assert!(
        secret["data"].get("forge-token").is_none(),
        "the Secret holds the daemon's GitLab token: {:?}",
        secret["data"]
            .as_object()
            .map(|data| data.keys().collect::<Vec<_>>())
    );
}

/// A caller that holds the control token adds an instance whose sources are on one forge. Its pod
/// carries the daemon's token of that forge only, never the daemon's token of the other forge,
/// on which the instance has no source (decision F4-D1): an instance on GitLab on the daemon's
/// GitLab host gets no `GH_TOKEN` or `GITHUB_TOKEN`, and one on GitHub no `GITLAB_TOKEN` or
/// `CI_JOB_TOKEN`; neither pod's Secret holds the other forge's secret under any key. A mapping
/// that names the secret of the daemon's `CI_JOB_TOKEN` entry as a token of its own is refused
/// `422` at its path (decision F4-D2), the file unchanged.
#[test]
fn a_pod_carries_no_daemon_token_of_a_forge_its_instance_has_no_source_on() {
    let case = Case::new("forge-per-source");
    let gitlab_value = ["gitlab-forge", "-value-", "7777"].concat();
    let job_value = ["gitlab-job", "-value-", "8888"].concat();
    let github_value = ["github-classic", "-value-", "6666"].concat();
    fs::write(case.dir.join("secrets/gitlab-forge"), &gitlab_value).expect("write a secret");
    fs::write(case.dir.join("secrets/gitlab-job"), &job_value).expect("write a secret");
    fs::write(case.dir.join("secrets/github-classic"), &github_value).expect("write a secret");
    let text = case.daemon_text(" []").replace(
        "{name: GH_TOKEN, secret: secret:forge-token}",
        "{name: GH_TOKEN, secret: secret:forge-token}, \
         {name: GITHUB_TOKEN, secret: secret:github-classic}, \
         {name: GITLAB_TOKEN, secret: secret:gitlab-forge}, \
         {name: CI_JOB_TOKEN, secret: secret:gitlab-job}",
    );
    fs::write(case.file(), &text).expect("write the config file");
    let port = case.serve();
    for (name, source) in [
        ("beta", json!({"gitlab": "example-org"})),
        ("gamma", json!({"github": "example-org"})),
    ] {
        let sent = json!({
            "name": name,
            "placement": "none",
            "session_prefix": name,
            "sources": [source],
        });
        let (status, body) = exchange(
            port,
            &request(
                port,
                "POST",
                "/v1/instances",
                Some(&json!({"mapping": sent, "reason": "a tenant"})),
            ),
        );
        assert_eq!(status, 201, "{name}: {body}");
    }
    let before = case.text();
    let sent = json!({
        "name": "delta",
        "placement": "none",
        "session_prefix": "delta",
        "sources": [{"gitlab": "example-org"}],
        "model_token": "secret:gitlab-job",
    });
    let (status, body) = exchange(
        port,
        &request(
            port,
            "POST",
            "/v1/instances",
            Some(&json!({"mapping": sent, "reason": "a tenant"})),
        ),
    );
    assert_eq!(status, 422, "{body}");
    let answer: Value = serde_json::from_str(&body).expect("a JSON body");
    assert_eq!(answer["path"], "instances[2].model_token", "{answer}");
    assert!(
        answer["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains(DAEMONS_FORGE_SECRET)),
        "{answer}"
    );
    assert_eq!(case.text(), before, "the file changed");

    let fake = FakeKube::start(&case.dir);
    let mut daemon = case.daemon(&fake, "daemon");
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    for (name, own, other) in [
        (
            "beta",
            [
                ("GITLAB_TOKEN", "gitlab-forge"),
                ("CI_JOB_TOKEN", "gitlab-job"),
            ],
            [
                ("GH_TOKEN", "forge-token", FORGE_TOKEN),
                ("GITHUB_TOKEN", "github-classic", github_value.as_str()),
            ],
        ),
        (
            "gamma",
            [
                ("GH_TOKEN", "forge-token"),
                ("GITHUB_TOKEN", "github-classic"),
            ],
            [
                ("GITLAB_TOKEN", "gitlab-forge", gitlab_value.as_str()),
                ("CI_JOB_TOKEN", "gitlab-job", job_value.as_str()),
            ],
        ),
    ] {
        let pod = fake
            .object(NAMESPACE, "pods", &format!("conductor-instance-{name}"))
            .unwrap_or_else(|| panic!("the pod of {name}"));
        let env = pod_env(&pod);
        for (variable, _) in own {
            assert!(
                env.iter().any(|entry| entry["name"] == variable),
                "precondition: {name} carries the daemon's {variable} of its own forge: {pod}"
            );
        }
        for (variable, key, _) in other {
            assert!(
                !env.iter().any(|entry| entry["name"] == variable
                    || entry["valueFrom"]["secretKeyRef"]["key"] == key),
                "{name} has no source on that forge and carries the daemon's {variable}: {pod}"
            );
        }
        let secret = fake
            .object(NAMESPACE, "secrets", &format!("conductor-instance-{name}"))
            .unwrap_or_else(|| panic!("the Secret of {name}"));
        let data = secret["data"].as_object().cloned().unwrap_or_default();
        for (key, encoded) in &data {
            let held = encoded
                .as_str()
                .and_then(pods::base64_decode)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            for (variable, _, value) in other {
                assert!(
                    held != value,
                    "the Secret of {name} holds the daemon's {variable} under {key}"
                );
            }
        }
    }
}

/// The pod's `secret` volume lists the keys of the instance's Secret it mounts (`items`),
/// each under its own file name at the volume's mode 0440: the rendered file, the instance's own
/// forge token, its model token and each secret `daemon.kubernetes.env` maps, and not
/// [`pods::TOKEN_KEY`], which the pod takes as its variable alone.
#[test]
fn the_secret_volume_lists_every_key_of_the_secret_but_the_runtimes_token() {
    let case = Case::new("secret-items");
    fs::write(case.dir.join("secrets/beta-forge"), "beta-forge-value-3333").expect("a secret");
    fs::write(
        case.dir.join("secrets/extra-token"),
        "extra-token-value-4444",
    )
    .expect("a secret");
    let text = case
        .daemon_text(
            "\n  - name: beta\n    placement: none\n    session_prefix: beta\n    \
             sources: [{github: example-org}]\n    model_token: secret:alpha-model\n    \
             forge_token: secret:beta-forge",
        )
        .replace(
            "env: [{name: GH_TOKEN, secret: secret:forge-token}]",
            "env: [{name: GH_TOKEN, secret: secret:forge-token}, \
             {name: EXTRA_TOKEN, secret: secret:extra-token}]",
        );
    fs::write(case.file(), text).expect("write the config file");
    let fake = FakeKube::start(&case.dir);
    let mut daemon = case.daemon(&fake, "daemon");
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    let pod = fake
        .object(NAMESPACE, "pods", "conductor-instance-beta")
        .expect("the pod");
    let secret = fake
        .object(NAMESPACE, "secrets", "conductor-instance-beta")
        .expect("the Secret");
    let keys: BTreeSet<String> = secret["data"]
        .as_object()
        .map(|data| data.keys().cloned().collect())
        .unwrap_or_default();
    for key in [
        "conductor.yaml",
        pods::TOKEN_KEY,
        "alpha-model",
        "beta-forge",
        "extra-token",
    ] {
        assert!(keys.contains(key), "the Secret has no key {key}: {keys:?}");
    }
    let volume = pod["spec"]["volumes"]
        .as_array()
        .expect("volumes")
        .iter()
        .find(|volume| volume["name"] == "secret")
        .cloned()
        .expect("the secret volume");
    assert_eq!(volume["secret"]["defaultMode"], 0o440, "{volume}");
    let items = volume["secret"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(!items.is_empty(), "the volume lists no keys: {volume}");
    let listed: BTreeSet<String> = items
        .iter()
        .map(|item| item["key"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert!(!listed.contains(pods::TOKEN_KEY), "{volume}");
    let others: BTreeSet<String> = keys
        .iter()
        .filter(|key| *key != pods::TOKEN_KEY)
        .cloned()
        .collect();
    assert_eq!(listed, others, "{volume}");
    for item in &items {
        assert_eq!(item["path"], item["key"], "{item}");
        assert!(item.get("mode").is_none_or(|mode| *mode == 0o440), "{item}");
    }
}
