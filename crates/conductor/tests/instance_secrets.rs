//! `GET /v1/instances/{instance}/secrets` (`story:instance-secret-state`,
//! `conductor.secret.InstanceSecrets`): a consumer control plane reads, for each key of an
//! instance's mapping that names a secret, the secret's name, whether the daemon's secrets source
//! holds a value under it as a start would find it, and for the `files` source when its file last
//! changed; never a value. The daemon answers it from its own config file and its own source, at
//! each call, so a file added to the mounted directory reads `set: true` at the next call.
//!
//! The route's cases run the daemon's API in this process, built from the case's config file as
//! `daemon serve` builds it, on a listener bound to every address as on a pod: the file runs its
//! instances as pods, which `daemon serve` does only inside a cluster, since `forge_token` is a
//! pod's key. One case runs `conductor daemon serve` on a file without pods, to read every line it
//! writes. No answer and no line carries a secret's value.

mod active;
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use conductor_cli::daemon::api::{self, Api};
use conductor_cli::secret;
use conductor_model::config::SecretsSource;
use conductor_model::secret::Source;
use serde_json::{Value, json};

/// The value of the forge secret, built from pieces: no answer and no log line may carry it.
fn forge_value() -> String {
    ["example-org", "forge", "value", "5d0e9b"].join("-")
}

/// The value of the model secret, built from pieces.
fn model_value() -> String {
    ["example-org", "model", "value", "8a2f4c"].join("-")
}

/// The control token: the value of the secret `control` in the case's source.
fn control_value() -> String {
    ["control", "token", "of", "the", "case"].join("-")
}

/// The daemon's own token, which no case presents.
const TOKEN: &str = "4444444444444444444444444444444444444444444444444444444444444444";

/// When the forge secret's file was last written, as the case sets it.
const FORGE_WRITTEN: u64 = 1_700_000_000;

/// The same time as the answer writes it, RFC 3339.
const FORGE_WRITTEN_TEXT: &str = "2023-11-14T22:13:20Z";

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
            .join("instance-secrets")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        let case = Self { dir };
        fs::write(case.file(), case.text()).expect("write the config file");
        case.put("control", &control_value());
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn secrets(&self) -> PathBuf {
        self.dir.join("secrets")
    }

    fn store(&self) -> String {
        format!("sqlite:{}", self.dir.join("daemon.sqlite").display())
    }

    /// The `daemon` section: on every address, with the control token `control` of its `files`
    /// source, and `more`.
    fn daemon(&self, port: u16, more: &str) -> String {
        format!(
            "daemon: {{port: {port}, address: \"0.0.0.0\", store: {store:?}, \
             control_token: \"secret:control\", secrets: {{source: files, dir: {secrets:?}}}{more}}}\n",
            store = self.store(),
            secrets = self.secrets(),
        )
    }

    /// The config file of the route's cases: the daemon running its instances as pods, and the
    /// instance `alpha` naming `alpha-model` as its `model_token` and `alpha-forge` as its
    /// `forge_token`.
    fn text(&self) -> String {
        format!(
            "version: conductor.config/2\n{daemon}\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: none\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   model_token: secret:alpha-model\n\
             \x20   forge_token: secret:alpha-forge\n",
            daemon = self.daemon(
                7300,
                ", kubernetes: {namespace: conductor-instances, \
                 image: \"registry.example.com/conductor:1.0.0\", owner: example-org, \
                 api_port: 7310, volume: {size: 20G}}"
            ),
        )
    }

    /// The config file `daemon serve` runs on outside a cluster: no pods, and the instance
    /// `alpha` on the local placement naming `alpha-model` as its `model_token`.
    fn local_text(&self, port: u16) -> String {
        let root = self.dir.join("alpha");
        format!(
            "version: conductor.config/2\n{daemon}\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: local\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             \x20   model_token: secret:alpha-model\n",
            daemon = self.daemon(port, ""),
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        )
    }

    /// Writes `value` as the secret `name` of the `files` source.
    fn put(&self, name: &str, value: &str) {
        fs::write(self.secrets().join(name), value).expect("write a secret's file");
    }

    /// Dates the file at `path` at [`FORGE_WRITTEN`].
    fn date(&self, path: &Path) {
        fs::File::options()
            .write(true)
            .open(path)
            .and_then(|file| {
                file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(FORGE_WRITTEN))
            })
            .expect("date a secret's file");
    }

    fn validate(&self) {
        let output = common::conductor(self.home())
            .arg("--config")
            .arg(self.file())
            .args(["config", "validate"])
            .output()
            .expect("run config validate");
        assert_eq!(
            output.status.code(),
            Some(0),
            "config validate: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// The daemon's API on the case's file, as `daemon serve` builds it from the file's `daemon`
    /// section.
    fn api(&self) -> Api {
        let text = fs::read_to_string(self.file()).expect("read the config file");
        let config = conductor_cli::config::parse(&text, &self.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}"));
        Api::new(TOKEN.to_owned(), "operator".to_owned(), None)
            .with_config(self.file(), self.home(), self.store())
            .with_secrets(&config.daemon)
            .with_control_token(
                config.daemon.secrets.clone(),
                config.daemon.control_token.clone(),
            )
            .with_pods(config.daemon.kubernetes.is_some())
    }
}

/// Serves `api` on a listener bound to `address`: `0.0.0.0` takes bearers, as on a pod,
/// `127.0.0.1` is the loopback listener. Its port.
fn serve(api: Api, address: &str) -> u16 {
    let listener = TcpListener::bind((address, 0)).expect("bind the listener");
    let port = listener.local_addr().expect("address").port();
    let api = Arc::new(api);
    thread::spawn(move || api::serve(&listener, &api));
    port
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

/// `GET target` on `port` through `127.0.0.1`, with `bearer` if any; none when nothing answers.
fn try_get(port: u16, target: &str, bearer: Option<&str>) -> Option<Answer> {
    let mut text =
        format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if let Some(bearer) = bearer {
        text.push_str(&format!("Authorization: Bearer {bearer}\r\n"));
    }
    text.push_str("\r\n");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .ok()?;
    stream.write_all(text.as_bytes()).ok()?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head.split(' ').nth(1)?.parse().ok()?;
    Some(Answer {
        status,
        body: body.to_owned(),
    })
}

fn get(port: u16, target: &str, bearer: Option<&str>) -> Answer {
    try_get(port, target, bearer).unwrap_or_else(|| panic!("no answer to GET {target}"))
}

/// The rows of `answer` by key.
fn by_key(answer: &Answer) -> std::collections::BTreeMap<String, Value> {
    answer
        .json()
        .as_array()
        .unwrap_or_else(|| panic!("no list: {answer:?}"))
        .iter()
        .map(|row| {
            (
                row["key"].as_str().unwrap_or_default().to_owned(),
                row.clone(),
            )
        })
        .collect()
}

/// With the control token: one row per key of `alpha`'s mapping that names a secret, its
/// instance, key, secret and `set`; `set: false` and no `modified_at` for a file that is not
/// there; `set: true` and the file's modification time for one that is. A file added after the
/// daemon started reads `set: true` at the next call, an empty one `set: false`. A name of the
/// daemon's own written into the file afterwards is no row. No answer carries a value.
#[test]
fn a_control_plane_reads_whether_each_named_secret_is_set_and_never_its_value() {
    let case = Case::new("rows");
    case.validate();
    case.put("alpha-forge", &format!("{}\n", forge_value()));
    case.date(&case.secrets().join("alpha-forge"));
    let port = serve(case.api(), "0.0.0.0");
    let control = control_value();
    let mut bodies = String::new();

    let first = get(port, "/v1/instances/alpha/secrets", Some(&control));
    bodies.push_str(&first.body);
    assert_eq!(first.status, 200, "{first:?}");
    let rows = by_key(&first);
    assert_eq!(
        rows.keys().map(String::as_str).collect::<Vec<_>>(),
        ["forge_token", "model_token"],
        "{first:?}"
    );
    assert_eq!(
        rows["model_token"],
        json!({"instance": "alpha", "key": "model_token", "secret": "alpha-model", "set": false}),
        "{first:?}"
    );
    assert_eq!(
        rows["forge_token"],
        json!({
            "instance": "alpha", "key": "forge_token", "secret": "alpha-forge", "set": true,
            "modified_at": FORGE_WRITTEN_TEXT,
        }),
        "{first:?}"
    );

    // Added to the mounted directory after the start: read at the next call, no restart.
    case.put("alpha-model", &model_value());
    let added = get(port, "/v1/instances/alpha/secrets", Some(&control));
    bodies.push_str(&added.body);
    assert_eq!(added.status, 200, "{added:?}");
    let model = &by_key(&added)["model_token"];
    assert_eq!(model["set"], true, "{added:?}");
    assert!(
        model["modified_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{added:?}"
    );

    // An empty file, or one holding only a line end, holds no value: a start would refuse.
    for empty in ["", "\n"] {
        case.put("alpha-model", empty);
        let emptied = get(port, "/v1/instances/alpha/secrets", Some(&control));
        bodies.push_str(&emptied.body);
        assert_eq!(emptied.status, 200, "{emptied:?}");
        assert_eq!(
            by_key(&emptied)["model_token"],
            json!({"instance": "alpha", "key": "model_token", "secret": "alpha-model", "set": false}),
            "{empty:?}: {emptied:?}"
        );
    }

    // A file written by hand after the start whose `model_token` names the control token's
    // secret, which validation refuses: the name is no row.
    fs::write(
        case.file(),
        case.text().replace(
            "model_token: secret:alpha-model",
            "model_token: secret:control",
        ),
    )
    .expect("write the config file");
    let withheld = get(port, "/v1/instances/alpha/secrets", Some(&control));
    bodies.push_str(&withheld.body);
    assert_eq!(withheld.status, 200, "{withheld:?}");
    assert!(
        !withheld.body.contains("\"control\""),
        "a secret of the daemon's own is listed: {withheld:?}"
    );
    assert_eq!(
        by_key(&withheld)
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["forge_token"],
        "{withheld:?}"
    );

    for value in [forge_value(), model_value(), control] {
        assert!(
            !bodies.contains(&value),
            "an answer carries a value: {bodies}"
        );
    }
}

/// An instance the config file does not hold is `404` naming
/// `conductor.instance.UnknownInstance`; without a bearer, a listener that takes bearers answers
/// `401` and the loopback listener reads as for `GET /v1/instances/{name}`; and `GET
/// /v1/secrets` still refuses the control token.
#[test]
fn an_unknown_instance_is_404_no_bearer_401_and_the_secrets_list_refuses_the_control_token() {
    let case = Case::new("refusals");
    let port = serve(case.api(), "0.0.0.0");
    let control = control_value();

    let unknown = get(port, "/v1/instances/nobody/secrets", Some(&control));
    assert_eq!(unknown.status, 404, "{unknown:?}");
    assert_eq!(
        unknown.json(),
        json!({"error": "conductor.instance.UnknownInstance", "name": "nobody"}),
        "{unknown:?}"
    );

    let bare = get(port, "/v1/instances/alpha/secrets", None);
    assert_eq!(bare.status, 401, "{bare:?}");

    let listed = get(port, "/v1/secrets", Some(&control));
    assert_eq!(listed.status, 403, "{listed:?}");
    assert_eq!(
        listed.json()["error"],
        "conductor.operator.ActorNotHuman",
        "{listed:?}"
    );

    let loopback = serve(case.api(), "127.0.0.1");
    let local = get(loopback, "/v1/instances/alpha/secrets", None);
    assert_eq!(local.status, 200, "{local:?}");
    assert_eq!(by_key(&local).len(), 2, "{local:?}");
}

/// A running `daemon serve`, stopped when dropped.
struct Daemon {
    child: Child,
    readers: Vec<JoinHandle<Vec<u8>>>,
    state: PathBuf,
}

impl Daemon {
    /// Stops the daemon: everything it wrote to its standard output and error, and every file
    /// of its state directory.
    fn stop(mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut said = String::new();
        for reader in self.readers.drain(..) {
            let bytes = reader.join().expect("read the daemon's output");
            said.push_str(&String::from_utf8_lossy(&bytes));
        }
        said.push_str(&files_text(&self.state));
        said
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Every file under `dir`, as text, one after another.
fn files_text(dir: &Path) -> String {
    let mut text = String::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return text;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            text.push_str(&files_text(&path));
        } else if let Ok(bytes) = fs::read(&path) {
            text.push_str(&String::from_utf8_lossy(&bytes));
        }
    }
    text
}

/// `conductor daemon serve` on `case`'s file, until it answers `GET /v1/version` on `port`.
fn daemon_serve(case: &Case, port: u16) -> Daemon {
    // An empty PATH: no systemctl or systemd-run, so the daemon's pass reaches no service
    // manager of the host.
    let empty = case.dir.join("no-programs");
    fs::create_dir_all(&empty).expect("create an empty PATH directory");
    let mut child = common::conductor(case.home())
        .arg("--config")
        .arg(case.file())
        .env("PATH", &empty)
        .args(["daemon", "serve", "--state-dir"])
        .arg(case.dir.join("daemon"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run daemon serve");
    let drain = |mut from: Box<dyn std::io::Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = from.read_to_end(&mut bytes);
            bytes
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("stdout")));
    let err = drain(Box::new(child.stderr.take().expect("stderr")));
    let daemon = Daemon {
        child,
        readers: vec![out, err],
        state: case.dir.join("daemon"),
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if try_get(port, "/v1/version", Some(&control_value()))
            .is_some_and(|answer| answer.status == 200)
        {
            return daemon;
        }
        assert!(
            Instant::now() < deadline,
            "the daemon did not answer: {}",
            daemon.stop()
        );
        thread::sleep(Duration::from_millis(100));
    }
}

/// `conductor daemon serve` answers the route with the model secret set, and no line of its
/// output or file of its state directory carries the secret's value or the control token.
#[test]
fn no_line_of_the_daemons_output_carries_a_value() {
    let case = Case::new("daemon-log");
    let port = TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port");
    fs::write(case.file(), case.local_text(port)).expect("write the config file");
    case.validate();
    case.put("alpha-model", &format!("{}\n", model_value()));
    let daemon = daemon_serve(&case, port);
    let control = control_value();

    let answer = get(port, "/v1/instances/alpha/secrets", Some(&control));
    assert_eq!(answer.status, 200, "{answer:?}");
    assert_eq!(
        answer.json(),
        json!([{
            "instance": "alpha", "key": "model_token", "secret": "alpha-model", "set": true,
            "modified_at": by_key(&answer)["model_token"]["modified_at"],
        }]),
        "{answer:?}"
    );
    assert!(!answer.body.contains(&model_value()), "{answer:?}");

    let said = daemon.stop();
    assert!(
        said.contains("secrets: files"),
        "the daemon named no secrets source: {said}"
    );
    assert!(
        !said.contains(&model_value()),
        "the daemon's output carries the value: {said}"
    );
    assert!(
        !said.contains(&control),
        "the daemon's output carries the control token"
    );
}

/// The `files` source of `dir`.
fn files(dir: &Path) -> SecretsSource {
    SecretsSource {
        source: Source::Files,
        namespace: None,
        url: None,
        dir: Some(dir.display().to_string()),
    }
}

/// A mounted Kubernetes Secret's key is a link into its `..data` directory: the row reads the
/// time of the file it links to, not of the link.
#[test]
fn a_linked_key_reads_the_time_of_the_file_it_links_to() {
    let case = Case::new("linked");
    let data = case.secrets().join("..data");
    fs::create_dir_all(&data).expect("create ..data");
    fs::write(data.join("alpha-forge"), forge_value()).expect("write the linked file");
    case.date(&data.join("alpha-forge"));
    std::os::unix::fs::symlink("..data/alpha-forge", case.secrets().join("alpha-forge"))
        .expect("link the key");
    let rows = secret::instance_view(
        &files(&case.secrets()),
        &case.text(),
        "alpha",
        &BTreeSet::new(),
    )
    .unwrap_or_else(|_| panic!("the files source did not answer"))
    .expect("alpha is an instance of the file");
    let json = serde_json::to_value(&rows).expect("rows as JSON");
    assert_eq!(
        json,
        json!([
            {"instance": "alpha", "key": "model_token", "secret": "alpha-model", "set": false},
            {"instance": "alpha", "key": "forge_token", "secret": "alpha-forge", "set": true,
             "modified_at": FORGE_WRITTEN_TEXT},
        ])
    );
    assert!(!json.to_string().contains(&forge_value()), "{json}");
}

/// A name the daemon keeps to itself, or the secret of one of its forge-token entries, is no
/// row, whichever key names it.
#[test]
fn a_withheld_name_is_no_row() {
    let case = Case::new("withheld");
    let text = format!(
        "{}    roles: [{{role: controller, model_token: secret:daemon-forge}}]\n",
        case.text()
    );
    let withheld: BTreeSet<String> = ["alpha-forge", "daemon-forge"].map(str::to_owned).into();
    let rows = secret::instance_view(&files(&case.secrets()), &text, "alpha", &withheld)
        .unwrap_or_else(|_| panic!("the files source did not answer"))
        .expect("alpha is an instance of the file");
    let secrets: Vec<&str> = rows.iter().map(|row| row.secret.as_str()).collect();
    assert_eq!(secrets, ["alpha-model"]);
    assert_eq!(
        secret::instance_view(&files(&case.secrets()), &text, "nobody", &withheld)
            .unwrap_or_else(|_| panic!("the files source did not answer")),
        None
    );
}

/// A source that does not answer is its declared error, `SourceUnreachable` naming the source,
/// not rows that read `set: false`.
#[test]
fn a_source_that_does_not_answer_is_source_unreachable() {
    let case = Case::new("unreachable");
    let remote = SecretsSource {
        source: Source::Remote,
        namespace: None,
        url: Some("https://secrets.example".to_owned()),
        dir: None,
    };
    match secret::instance_view(&remote, &case.text(), "alpha", &BTreeSet::new()) {
        Err(secret::Unanswered::Unreachable(error)) => assert_eq!(error.source, Source::Remote),
        Err(secret::Unanswered::Unusable(unusable)) => panic!("unusable: {unusable}"),
        Ok(rows) => panic!("rows from a source that does not answer: {rows:?}"),
    }
}
