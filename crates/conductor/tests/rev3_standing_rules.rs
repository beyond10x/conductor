//! A third review of an instance's standing rules (story:instance-rules-text): the one writer of
//! the text through `PutConfig` and the instance's settings write, the refusal's problem at every
//! level, a whole-text secret reference, and the daemon's write in place on a conflict and on a
//! Secret whose rendered file does not decode.
//!
//! Every case runs the library in this process over files under this target's temporary
//! directory ([`scratch::Scratch`]); the served case runs the instance API behind the daemon's
//! loopback listener as `tests/served/` does, and the pod cases run the reconciler against the
//! fake API server of `tests/fake_kube/`.

mod active;
mod fake_kube;
mod scratch;
mod served;
mod standing_rules;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use conductor_cli::config;
use conductor_cli::config::put::put_config;
use conductor_cli::daemon::pods::Pods;
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Manager;
use conductor_cli::dashboard::Sources;
use conductor_model::config::{InstanceName, Level, PutConfig, RoleName};
use conductor_model::instance::ReconcileReason;
use fake_kube::FakeKube;
use scratch::Scratch;
use serde_json::{Value, json};
use standing_rules::{HOSTILE, RULES};

/// The rules the control plane wrote for `alpha`, and for `beta`.
const ALPHA_RULES: &str = "Alpha's rule: no private name in a commit.\n";
const BETA_RULES: &str = "Beta's rule: every commit is signed.\n";

/// A config file of two instances, `alpha` and `beta`, each with the standing rules the control
/// plane wrote for it.
fn two_instances(dir: &Path) -> String {
    let instance = |name: &str, rules: &str| {
        format!(
            "\x20 - name: {name}\n\
             \x20   placement: systemd\n\
             \x20   session_prefix: {name}\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   standing_rules: {rules}\n",
            root = dir.join(name),
            trees = dir.join(format!("{name}-trees")),
            rules = serde_json::to_string(rules).expect("JSON"),
        )
    };
    format!(
        "version: conductor.config/2\n\
         instances:\n{}{}",
        instance("alpha", ALPHA_RULES),
        instance("beta", BETA_RULES)
    )
}

/// The standing rules of the instance named `name` in the config file `file`.
fn rules_of(file: &Path, home: &Path, name: &str) -> Option<String> {
    let text = fs::read_to_string(file).expect("read the config file");
    config::parse(&text, home)
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"))
        .instances
        .into_iter()
        .find(|instance| instance.name.0 == name)
        .and_then(|instance| instance.standing_rules)
}

/// A change by the operator, at `level` of `instance` (and `role`), of `path` to `value`.
fn change(
    level: Level,
    instance: Option<&str>,
    role: Option<&str>,
    path: &str,
    value: Option<&str>,
) -> PutConfig {
    PutConfig {
        actor: conductor_cli::actor::operator("operator"),
        level,
        instance: instance.map(|name| InstanceName(name.to_owned())),
        role: role.map(|role| RoleName(role.to_owned())),
        path: path.to_owned(),
        value: value.map(str::to_owned),
        reason: "a third review of the one writer".to_owned(),
        changed_at: conductor_model::primitives::Timestamp("2026-11-13T09:00:00Z".to_owned()),
    }
}

/// Claim 1, the one writer. `PUT /v1/config` refuses the key `standing_rules`, but not the key
/// `name` of the instance mapping that holds it. Three puts of `name` at the instance level, none
/// of which names `standing_rules`, swap the names of `alpha` and `beta`: the instance named
/// `alpha` then reads the rules the control plane wrote for `beta`, and no instance call wrote
/// them. Either a put of `name` of an instance with standing rules is refused, or the instance
/// named `alpha` keeps the text the control plane wrote for it.
#[test]
fn rev3_puts_of_the_instance_name_swap_the_standing_rules_of_two_instances() {
    active::isolate();
    let scratch = Scratch::new(Path::new("rev3-rules").join("rename"));
    let dir = scratch.path().to_owned();
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the home");
    let file = dir.join("conductor.yaml");
    fs::write(&file, two_instances(&dir)).expect("write the config file");
    assert_eq!(
        rules_of(&file, &home, "alpha").as_deref(),
        Some(ALPHA_RULES)
    );
    let url = format!("sqlite:{}", dir.join("daemon.sqlite").display());
    let mut store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the store opens");

    let mut taken = Vec::new();
    for (instance, to) in [("alpha", "gamma"), ("beta", "alpha"), ("gamma", "beta")] {
        let input = change(Level::Instance, Some(instance), None, "name", Some(to));
        match put_config(&file, &home, &mut store, &input).expect("no fault") {
            Ok(_) => taken.push(format!("{instance} -> {to}")),
            Err(_) => break,
        }
    }
    let read = rules_of(&file, &home, "alpha");
    assert_eq!(
        read.as_deref(),
        Some(ALPHA_RULES),
        "the puts {taken:?} of `name`, through `PUT /v1/config`, changed the standing rules the \
         instance named alpha reads to {read:?}, a text no instance call wrote for it"
    );
}

/// The refusal of a rename is narrow: an instance-level put of the `name` of an instance whose
/// mapping holds standing rules is refused at that `name`, says the instance calls rename it, and
/// quotes neither instance's rules, and nothing is written; the `name` of an instance without
/// standing rules is changed as before.
#[test]
fn a_put_of_the_name_is_refused_only_for_an_instance_with_standing_rules() {
    active::isolate();
    let scratch = Scratch::new(Path::new("rev3-rules").join("rename-narrow"));
    let dir = scratch.path().to_owned();
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the home");
    let file = dir.join("conductor.yaml");
    let beta = format!(
        "\x20   standing_rules: {}\n",
        serde_json::to_string(BETA_RULES).expect("JSON")
    );
    let text = two_instances(&dir).replace(&beta, "");
    assert_ne!(text, two_instances(&dir), "beta holds no rules here");
    fs::write(&file, &text).expect("write the config file");
    let url = format!("sqlite:{}", dir.join("daemon.sqlite").display());
    let mut store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the store opens");

    let input = change(Level::Instance, Some("alpha"), None, "name", Some("gamma"));
    let refusal = put_config(&file, &home, &mut store, &input)
        .expect("no fault")
        .expect_err("a rename of an instance with standing rules is refused");
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        text,
        "nothing written"
    );
    assert_eq!(
        refusal
            .problems
            .iter()
            .map(|problem| problem.path.as_str())
            .collect::<Vec<_>>(),
        ["instances[0].name"]
    );
    let said = refusal.to_string();
    assert!(said.contains("POST /v1/instances"), "{said}");
    for rules in [ALPHA_RULES, BETA_RULES] {
        assert!(
            !said.contains(rules.trim()),
            "the refusal quotes rules: {said}"
        );
    }

    let input = change(Level::Instance, Some("beta"), None, "name", Some("delta"));
    put_config(&file, &home, &mut store, &input)
        .expect("no fault")
        .expect("a rename of an instance without standing rules is taken");
    assert_eq!(rules_of(&file, &home, "delta"), None);
    assert_eq!(
        rules_of(&file, &home, "alpha").as_deref(),
        Some(ALPHA_RULES)
    );
}

/// Claim 1, the refusal's problem. The specification (`spec/domains/config.yaml`) says
/// `PutConfig` refuses the key `standing_rules` at every level "with a problem at the key's path
/// that names those two calls". A put at the role level of a role the instance does not write is
/// refused, and nothing is written, but its problem is the one of the missing role, at
/// `instances[0].roles`: it names neither the key's path nor `POST /v1/instances`.
#[test]
fn rev3_a_role_level_put_of_standing_rules_names_the_instance_calls() {
    active::isolate();
    let scratch = Scratch::new(Path::new("rev3-rules").join("role-level"));
    let dir = scratch.path().to_owned();
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the home");
    let file = dir.join("conductor.yaml");
    fs::write(&file, two_instances(&dir)).expect("write the config file");
    let text = fs::read_to_string(&file).expect("read");
    let url = format!("sqlite:{}", dir.join("daemon.sqlite").display());
    let mut store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the store opens");
    let input = change(
        Level::Role,
        Some("alpha"),
        Some("reviewer"),
        "standing_rules",
        Some("Zq9 offered rule"),
    );
    let refusal = put_config(&file, &home, &mut store, &input)
        .expect("no fault")
        .expect_err("refused");
    assert_eq!(fs::read_to_string(&file).expect("read"), text);
    let said = refusal.to_string();
    assert!(!said.contains("Zq9"), "{said}");
    assert!(
        said.contains("standing_rules") && said.contains("POST /v1/instances"),
        "the refusal at the role level names neither the key's path nor the instance calls: \
         {said}"
    );
}

/// Claim 4, closing a gap the suite leaves (green on this head): a whole text that is one secret
/// reference is refused around any Unicode white space Rust's `trim` takes (NBSP, ideographic
/// space, NEL, line separator), a BOM before it is no white space and is taken as text, and in
/// neither case does the secret scan of the file ([`conductor_cli::secret::referenced`]) list the
/// name, even over a file that was never validated.
#[test]
fn rev3_a_whole_text_secret_reference_is_refused_around_unicode_white_space_and_never_named() {
    active::isolate();
    for text in [
        "\u{00A0}secret:alpha-model\u{3000}",
        "\u{0085}secret:alpha-model\u{2028}\n",
        "\tsecret:alpha-model\r\n",
    ] {
        assert!(
            config::standing_rules_problem(text).is_some(),
            "{text:?} is taken"
        );
    }
    assert_eq!(
        config::standing_rules_problem("\u{FEFF}secret:alpha-model"),
        None
    );
    for rules in [
        "secret:alpha-model",
        "\u{FEFF}secret:alpha-model",
        " secret:alpha-model ",
    ] {
        let text = format!(
            "version: conductor.config/2\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   standing_rules: {}\n",
            serde_json::to_string(rules).expect("JSON")
        );
        let named = conductor_cli::secret::referenced(&text);
        assert!(
            named.get("alpha").is_none_or(|names| names.is_empty()),
            "{rules:?}: {named:?}"
        );
    }
}

/// The instance `example-org` behind the daemon's loopback listener, its config file a host's.
struct Served {
    scratch: Scratch,
}

impl Served {
    fn new(name: &str) -> Self {
        active::isolate();
        let case = Self {
            scratch: Scratch::new(Path::new("rev3-rules").join(name)),
        };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        fs::write(
            case.file(),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: eo\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
                 \x20   records: {records:?}\n\
                 \x20   state: {state:?}\n\
                 \x20   cache: {cache:?}\n\
                 \x20   standing_rules: {rules}\n",
                root = case.home().join("example-org"),
                trees = case.home().join("trees"),
                records = case.records(),
                state = case.state(),
                cache = case.dir().join("cache"),
                rules = serde_json::to_string(RULES).expect("JSON"),
            ),
        )
        .expect("write the config file");
        case
    }

    fn dir(&self) -> &Path {
        self.scratch.path()
    }

    fn home(&self) -> PathBuf {
        self.dir().join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    fn state(&self) -> PathBuf {
        self.dir().join("state")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    fn start(&self) -> u16 {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources.config_file = Some(self.file());
        sources.instance_name = Some("example-org".to_owned());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        served::serve(listener, sources, Some(self.state()));
        port
    }
}

/// One HTTP exchange on loopback: the status and the body.
fn exchange(port: u16, raw: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("set a read timeout");
    stream.write_all(raw).expect("send the request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    (status, body.to_owned())
}

/// The runtime's settings write of `body`, from the page's own origin with its token.
fn settings_write(port: u16, body: &Value) -> (u16, String) {
    let (status, page) = exchange(
        port,
        format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").as_bytes(),
    );
    assert_eq!(status, 200, "GET /: {page}");
    let marker = r#"<meta name="conductor-token" content=""#;
    let at = page.find(marker).expect("the page names its token");
    let rest = &page[at + marker.len()..];
    let token = rest[..rest.find('"').expect("the token's closing quote")].to_owned();
    let body = serde_json::to_vec(body).expect("a JSON body");
    let mut raw = format!(
        "PUT /v1/instances/example-org/settings HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Origin: http://127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         X-Conductor-Token: {token}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    raw.extend_from_slice(&body);
    exchange(port, &raw)
}

/// Claim 8, the one writer, under concurrency. `PUT /v1/instances/{name}`, the writer of the
/// text, reads, writes and commits the config file under its one lock (`.<name>.lock` beside it,
/// `config/put.rs`), so another run that read the file first cannot write its old text over the
/// change. The instance's settings write reads the whole file and writes it whole by a rename,
/// standing rules included, but takes no lock: here it writes the file while another writer holds
/// the lock. Interleaved with an instance call that changes the rules, it puts the old rules back,
/// a second writer of the text.
#[test]
fn rev3_the_settings_write_does_not_write_the_file_while_its_lock_is_held() {
    let case = Served::new("settings-lock");
    let port = case.start();
    let before = fs::read_to_string(case.file()).expect("read");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(case.home().join(".conductor.yaml.lock"))
        .expect("open the config file's lock");
    lock.lock().expect("hold the config file's lock");

    let (sent, answered) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let answer = settings_write(
            port,
            &json!({
                "changes": [{"path": "thresholds.silent_session", "value": "45m"}],
                "reason": "the operator's change of another key",
            }),
        );
        let _ = sent.send(answer);
    });
    let during = answered.recv_timeout(Duration::from_secs(5));
    let written_under_lock = fs::read_to_string(case.file()).expect("read") != before;
    lock.unlock().expect("release the lock");
    drop(lock);
    let _ = writer.join();
    assert!(
        !written_under_lock,
        "the settings write ({during:?}) wrote the config file, its standing rules included, \
         while another writer held the file's lock"
    );
}

const NAMESPACE: &str = "conductor-instances";
const IMAGE: &str = "registry.example.com/conductor:1.0.0";
const ALPHA_SECRET: &str =
    "/api/v1/namespaces/conductor-instances/secrets/conductor-instance-alpha";

/// A daemon that runs `alpha` as a pod against a fake API server.
struct Cluster {
    dir: PathBuf,
    fake: FakeKube,
    _scratch: Scratch,
}

impl Cluster {
    fn new(name: &str) -> Self {
        active::isolate();
        let scratch = Scratch::new(Path::new("rev3-rules").join(name));
        let dir = scratch.path().to_owned();
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        fs::write(dir.join("secrets/alpha-model"), "model-token-value-0000")
            .expect("write a secret");
        let fake = FakeKube::start(&dir);
        Self {
            dir,
            fake,
            _scratch: scratch,
        }
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn config(&self, rules: Option<&str>) {
        let rules = rules
            .map(|rules| {
                format!(
                    "\x20   standing_rules: {}\n",
                    serde_json::to_string(rules).expect("JSON")
                )
            })
            .unwrap_or_default();
        let text = format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: {store:?}\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: {IMAGE}\n\
             \x20   owner: example-org\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G, storage_class: fast-local}}\n\
             \x20   env: []\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: none\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   model_token: secret:alpha-model\n\
             {rules}",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            secrets = self.dir.join("secrets"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    /// The daemon after two passes, its pod `alpha` running; the time of its first pass.
    fn running(&self) -> (Daemon<Manager>, Instant) {
        let mut daemon = Daemon::open(
            self.file(),
            self.dir.join("home"),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Manager::Pods(Box::new(Pods::new(self.fake.client()))),
        )
        .expect("open the daemon");
        let start = Instant::now();
        daemon
            .pass(ReconcileReason::Cadence, start)
            .expect("a pass");
        let mut pod = self
            .fake
            .object(NAMESPACE, "pods", "conductor-instance-alpha")
            .expect("the pod");
        pod["status"] = json!({
            "phase": "Running",
            "podIP": "127.0.0.1",
            "containerStatuses": [
                {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
            ],
        });
        self.fake.put(NAMESPACE, "pods", pod);
        daemon
            .pass(ReconcileReason::Cadence, start + Duration::from_secs(2))
            .expect("a pass");
        (daemon, start)
    }

    fn count(&self, method: &str) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|request| request.method == method && request.path == ALPHA_SECRET)
            .count()
    }

    fn held_rules(&self) -> Option<String> {
        let secret = self
            .fake
            .object(NAMESPACE, "secrets", "conductor-instance-alpha")
            .expect("the Secret");
        let encoded = secret["data"]["conductor.yaml"].as_str()?;
        let text = String::from_utf8(conductor_cli::daemon::pods::base64_decode(encoded)?).ok()?;
        let file: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
        file["instances"][0]["standing_rules"]
            .as_str()
            .map(str::to_owned)
    }
}

fn detail(daemon: &Daemon<Manager>) -> String {
    daemon
        .rows()
        .into_iter()
        .find(|row| row["name"] == "alpha")
        .expect("the row of alpha")["detail"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

/// Claim 2, closing a gap the suite leaves (green on this head): a `409` on the write in place,
/// which the guide names, is no refusal by the Role: the `detail` says the write did not go
/// through, names no right and no holding back, and the next pass sends the `PUT` again; once it
/// goes through the Secret holds the text and the `detail` clears.
#[test]
fn rev3_a_conflict_on_the_write_in_place_is_tried_again_at_the_next_pass() {
    let case = Cluster::new("conflict");
    case.config(None);
    let (mut daemon, start) = case.running();
    case.fake.refuse("PUT", "secrets", 409, "Conflict");
    case.config(Some(RULES));
    let _ = daemon.pass(
        ReconcileReason::ConfigChange,
        start + Duration::from_secs(4),
    );
    let shown = detail(&daemon);
    for words in ["did not go through", "409", "next pass tries again"] {
        assert!(shown.contains(words), "the detail says {words:?}: {shown}");
    }
    for words in ["update", "does not try again"] {
        assert!(!shown.contains(words), "the detail says {words:?}: {shown}");
    }
    let puts = case.count("PUT");
    let _ = daemon.pass(ReconcileReason::Cadence, start + Duration::from_secs(6));
    assert_eq!(case.count("PUT"), puts + 1, "tried again");
    case.fake.permit("PUT", "secrets");
    let _ = daemon.pass(ReconcileReason::Cadence, start + Duration::from_secs(8));
    assert_eq!(case.held_rules().as_deref(), Some(RULES));
    assert_eq!(detail(&daemon), "");
}

/// Claim 2, closing a gap the suite leaves (green on this head): a Secret of this daemon whose
/// rendered file is not base64, or is missing, holds no rendering, so the daemon writes the
/// rendering into it in place and keeps every other key.
#[test]
fn rev3_a_secret_whose_rendered_file_does_not_decode_is_written_in_place() {
    for (what, broken) in [
        ("not base64", Some(json!("%%% not base64 %%%"))),
        ("missing", None),
    ] {
        let case = Cluster::new(&format!("undecoded-{}", what.replace(' ', "-")));
        case.config(Some(RULES));
        let (mut daemon, start) = case.running();
        let mut secret = case
            .fake
            .object(NAMESPACE, "secrets", "conductor-instance-alpha")
            .expect("the Secret");
        let token = secret["data"]["api_token"].clone();
        match broken {
            Some(value) => secret["data"]["conductor.yaml"] = value,
            None => {
                secret["data"]
                    .as_object_mut()
                    .expect("data")
                    .remove("conductor.yaml");
            }
        }
        case.fake.put(NAMESPACE, "secrets", secret);
        case.config(Some(HOSTILE));
        let _ = daemon.pass(
            ReconcileReason::ConfigChange,
            start + Duration::from_secs(4),
        );
        assert_eq!(case.held_rules().as_deref(), Some(HOSTILE), "{what}");
        let secret = case
            .fake
            .object(NAMESPACE, "secrets", "conductor-instance-alpha")
            .expect("the Secret");
        assert_eq!(secret["data"]["api_token"], token, "{what}");
        assert_eq!(detail(&daemon), "", "{what}");
    }
}
