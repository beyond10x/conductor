//! The daemon's version in one JSON answer (`story:daemon-version-json`): `GET /v1/version`
//! answers `200` and the one row of `conductor.store.Version` in an array: `version` (the
//! binary's) and `config` (the config format it reads), the two label values of
//! `conductor_build_info` on `/metrics`. It is authenticated as every other read under `/v1`, the control
//! token admitted, and it does not wait for the reconciler's first publication. The daemon's API
//! runs in this process, on a loopback listener and on one bound to every address, which takes
//! bearers.

mod active;

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use conductor_cli::daemon::api::{self, Api};
use conductor_model::config::SecretsSource;
use conductor_model::secret::{SecretName, Source};
use serde_json::{Value, json};

/// The control token: the value of the secret `control` in the case's `files` secrets source.
const CONTROL: &str = "control-token-0123456789abcdef0123456789abcdef";

/// The daemon's own token, which no case presents.
const TOKEN: &str = "4444444444444444444444444444444444444444444444444444444444444444";

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
            .join("daemon-version")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self { dir };
        let root = case.dir.join("alpha");
        fs::write(
            case.file(),
            format!(
                "version: conductor.config/2\ndaemon: {{store: {:?}}}\ninstances:\n\
                 \x20 - name: alpha\n    placement: local\n    session_prefix: alpha\n    \
                 sources: [{{github: example-org}}]\n    \
                 checkouts: {{root: {:?}, trees: {:?}}}\n    records: {:?}\n    state: {:?}\n    \
                 cache: {:?}\n",
                case.store(),
                root.join("checkouts"),
                root.join("trees"),
                root.join("records"),
                root.join("state"),
                root.join("cache"),
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

    /// The case's `files` secrets source, its secret `control` holding [`CONTROL`].
    fn secrets(&self) -> SecretsSource {
        let dir = self.dir.join("secrets");
        fs::create_dir_all(&dir).expect("create the secrets directory");
        fs::write(dir.join("control"), CONTROL).expect("write the secret");
        SecretsSource {
            source: Source::Files,
            namespace: None,
            url: None,
            dir: Some(dir.display().to_string()),
        }
    }

    /// The daemon's API of the case with the control token and no Identity service.
    fn api(&self) -> Api {
        Api::new(TOKEN.to_owned(), "operator".to_owned(), None)
            .with_config(self.file(), self.home(), self.store())
            .with_control_token(self.secrets(), Some(SecretName("control".to_owned())))
    }

    /// The daemon's API of the case while it starts, before its reconciler published.
    fn starting(&self) -> Api {
        self.api().starting(Vec::new())
    }

    /// The daemon's API of the case, the instance `alpha` published Running.
    fn controlled(&self) -> Api {
        let api = self.api();
        api.publish(
            vec![json!({"name": "alpha", "state": "Running", "placement": "local", "roles": []})],
            BTreeMap::new(),
            Vec::new(),
        );
        api.publish_host(Vec::new(), None);
        api
    }
}

/// Serves `api` on the loopback listener and on one bound to every address, which takes bearers
/// as `daemon.address: 0.0.0.0` in a pod has it: their ports.
fn serve(api: Api) -> (u16, u16) {
    let api = Arc::new(api);
    let bind = |address: &str| {
        let listener = TcpListener::bind((address, 0)).expect("bind the listener");
        let port = listener.local_addr().expect("address").port();
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve(&listener, &served));
        port
    };
    (bind("127.0.0.1"), bind("0.0.0.0"))
}

/// One answer: its status and its body.
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

/// One request to the listener on `port`, through `127.0.0.1`, with `bearer` if any.
fn ask(port: u16, method: &str, target: &str, bearer: Option<&str>) -> Answer {
    let mut text =
        format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if let Some(bearer) = bearer {
        text.push_str(&format!("Authorization: Bearer {bearer}\r\n"));
    }
    text.push_str("\r\n");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream.write_all(text.as_bytes()).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    let text = String::from_utf8_lossy(&bytes).into_owned();
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

/// The label values of `conductor_build_info` on `/metrics` of the daemon on `port`, as the
/// one-row array `GET /v1/version` answers.
fn build_info(port: u16) -> Value {
    let metrics = ask(port, "GET", "/metrics", None);
    assert_eq!(metrics.status, 200, "{metrics:?}");
    let sample = metrics
        .body
        .lines()
        .find_map(|line| line.strip_prefix("conductor_build_info{"))
        .unwrap_or_else(|| panic!("no conductor_build_info sample: {}", metrics.body));
    let labels = sample.split_once('}').map_or(sample, |(labels, _)| labels);
    let label = |name: &str| {
        labels
            .split(',')
            .find_map(|pair| pair.strip_prefix(&format!("{name}=\"")))
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or_else(|| panic!("no label {name}: {sample}"))
            .to_owned()
    };
    json!([{"version": label("version"), "config": label("config")}])
}

/// The control token reads the version on either listener: `200` and a one-element array whose
/// row holds exactly the two fields, equal to the label values of `conductor_build_info` on
/// `/metrics` of the same daemon.
#[test]
fn the_control_token_reads_the_version_metrics_names() {
    let case = Case::new("control");
    let (loopback, everywhere) = serve(case.controlled());
    let expected = build_info(loopback);
    assert_eq!(
        expected[0]["version"],
        env!("CARGO_PKG_VERSION"),
        "{expected}"
    );
    assert_eq!(expected[0]["config"], "conductor.config/2", "{expected}");
    for port in [loopback, everywhere] {
        let read = ask(port, "GET", "/v1/version", Some(CONTROL));
        assert_eq!(read.status, 200, "{port}: {read:?}");
        let body = read.json();
        assert_eq!(body, expected, "{port}: {read:?}");
        let rows = body
            .as_array()
            .unwrap_or_else(|| panic!("no array: {body}"));
        assert_eq!(rows.len(), 1, "{body}");
        assert_eq!(
            rows[0].as_object().map(serde_json::Map::len),
            Some(2),
            "{body}"
        );
    }
}

/// Without a bearer the route answers as every other read under `/v1`: `401` on the listener
/// that takes bearers, the local caller's `200` on loopback; a bearer that is not the control
/// token is `401` on either, and any other method than `GET` is `405`.
#[test]
fn without_a_bearer_the_version_answers_as_every_other_read() {
    let case = Case::new("no-bearer");
    let (loopback, everywhere) = serve(case.controlled());
    let refused = ask(everywhere, "GET", "/v1/version", None);
    assert_eq!(refused.status, 401, "{refused:?}");
    let instances = ask(everywhere, "GET", "/v1/instances", None);
    assert_eq!(instances.status, refused.status, "{instances:?}");

    let local = ask(loopback, "GET", "/v1/version", None);
    assert_eq!(local.status, 200, "{local:?}");
    assert_eq!(local.json(), build_info(loopback), "{local:?}");

    for port in [loopback, everywhere] {
        let wrong = ask(port, "GET", "/v1/version", Some("not-the-control-token"));
        assert_eq!(wrong.status, 401, "{port}: {wrong:?}");
        let posted = ask(port, "POST", "/v1/version", Some(CONTROL));
        assert_eq!(posted.status, 405, "{port}: {posted:?}");
    }
}

/// The version does not wait for the reconciler: before its first publication `GET /v1/version`
/// answers `200` while `GET /v1/instances` answers `503`.
#[test]
fn before_the_first_publish_the_version_answers_and_the_instances_do_not() {
    let case = Case::new("unpublished");
    let daemon = case.starting();
    daemon.started();
    let (loopback, everywhere) = serve(daemon);
    for port in [loopback, everywhere] {
        let instances = ask(port, "GET", "/v1/instances", Some(CONTROL));
        assert_eq!(instances.status, 503, "{port}: {instances:?}");
        let read = ask(port, "GET", "/v1/version", Some(CONTROL));
        assert_eq!(read.status, 200, "{port}: {read:?}");
        assert_eq!(read.json(), build_info(loopback), "{port}: {read:?}");
    }
}
