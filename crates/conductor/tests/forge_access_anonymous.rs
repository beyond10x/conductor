//! story:forge-access-anonymous: an instance mapping that says `forge_access: anonymous` runs as
//! a pod that carries no credential of any forge, clones each repository its `github` sources
//! include over HTTPS with no credential, reads no pull request, issue, release or run of the
//! forge, and writes nothing to its forge.
//!
//! The cases, by the story's acceptance:
//! - the member, its one value and each refusal at its YAML path, read by the library's parser;
//! - the pod and the Secret the daemon creates, against the fake API server of
//!   `tests/fake_kube/`;
//! - the instance's `detail` and `GET /v1/instances/{name}`, from the daemon's rows and its API;
//! - the runtime's clone and its collect pass, the built binary run alone in a pod
//!   (`CONDUCTOR_API_LISTEN` set) with a cleared environment: `PATH` only the case's `bin/`, which
//!   holds a `gh` that logs every call and answers nothing, and a `git` that logs its words and
//!   whether a forge token reached it, then runs the real git. The forge is a directory of bare
//!   repositories which git reaches through a rewrite of `https://github.com/` given in its
//!   environment (`GIT_CONFIG_COUNT`), so no case reaches the network;
//! - the writes: the guard's decision on each forge write of a session, read by the library over
//!   the instance's places, and the records' commit, whose push is refused before git runs it.

mod active;
mod common;
mod fake_kube;
mod scratch;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use conductor_cli::config::{self, FORGE_CLI_TOKEN_VARIABLES};
use conductor_cli::daemon::api::{self, Api};
use conductor_cli::daemon::pods::{self, Pods};
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Manager;
use conductor_cli::guard::{self, Places, SessionList};
use conductor_cli::run::records::{self, Committed, Route};
use conductor_model::config::{ForgeAccess, ForgeRoute};
use conductor_model::dispatch::Verdict;
use conductor_model::instance::ReconcileReason;
use fake_kube::FakeKube;
use scratch::Scratch;
use serde_json::{Value, json};

/// The words every refusal of a write names.
const NAMED: &str = "forge_access: anonymous";

/// The one line the instance's `detail` carries.
const WRITES_NOTHING: &str = "writes nothing to its forge";

/// What a failed clone's line says.
const NOT_PUBLIC: &str = "is not public or not reachable";

/// git's last error line of a clone of a repository that is not there, after its status, as a
/// failed clone's line carries it: git says the repository may not exist or not be readable, and
/// its last line is the end of that sentence.
const GITS_LAST: &str = "`git clone` exited 128: and the repository exists.";

// ---------------------------------------------------------------------------------------------
// The member and its refusals
// ---------------------------------------------------------------------------------------------

/// A `conductor.config/2` file whose daemon runs pods when `pods`, with the one instance `alpha`
/// whose `sources` are `sources` and `extra` lines appended.
fn daemon_file(pods: bool, sources: &str, extra: &str) -> String {
    let kubernetes = if pods {
        "\x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example.com/conductor:1.0.0\n\
         \x20   owner: example-owner\n\
         \x20   api_port: 7310\n\
         \x20   volume: {size: 20G}\n"
    } else {
        ""
    };
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
         \x20 secrets: {{source: files, dir: /var/lib/example/secrets}}\n\
         {kubernetes}\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{sources}]\n\
         {extra}"
    )
}

/// The member parses to its one value, `config show` writes it as the file does, and a mapping
/// without it reads none, as before.
#[test]
fn the_member_reads_its_one_value_and_absent_reads_none() {
    active::isolate();
    let home = Path::new("/srv/example");
    let sources = "{github: example-owner, include: [one]}";
    let with = config::parse(
        &daemon_file(true, sources, "\x20   forge_access: anonymous\n"),
        home,
    )
    .unwrap_or_else(|problems| panic!("{problems:?}"));
    assert_eq!(with.instances[0].forge_access, Some(ForgeAccess::Anonymous));
    let without = config::parse(&daemon_file(true, "{github: example-owner}", ""), home)
        .unwrap_or_else(|problems| panic!("{problems:?}"));
    assert_eq!(without.instances[0].forge_access, None);
}

/// Each refusal of the member, at the key's or the offending member's YAML path: another value;
/// a `forge_token`; a source that is not `github`; a `github` source without `include`; a file
/// whose daemon runs no pods.
#[test]
fn each_refusal_names_its_path() {
    active::isolate();
    let home = Path::new("/srv/example");
    let anonymous = "\x20   forge_access: anonymous\n";
    let included = "{github: example-owner, include: [one]}";
    let cases: [(&str, String, &str, &str); 6] = [
        (
            "another value",
            daemon_file(true, included, "\x20   forge_access: public\n"),
            "instances[0].forge_access",
            "anonymous",
        ),
        (
            "a forge token",
            daemon_file(
                true,
                included,
                &format!("{anonymous}\x20   forge_token: secret:alpha-forge\n"),
            ),
            "instances[0].forge_token",
            NAMED,
        ),
        (
            "a gitlab source",
            daemon_file(
                true,
                "{gitlab: example-group, host: gitlab.example.com, include: [one]}",
                anonymous,
            ),
            "instances[0].sources[0]",
            NAMED,
        ),
        (
            "a local source",
            daemon_file(
                true,
                &format!("{included}, {{local: /srv/example}}"),
                anonymous,
            ),
            "instances[0].sources[1]",
            NAMED,
        ),
        (
            "a github source without include",
            daemon_file(true, "{github: example-owner}", anonymous),
            "instances[0].sources[0]",
            "include",
        ),
        (
            "a daemon that runs no pods",
            daemon_file(false, included, anonymous),
            "instances[0].forge_access",
            "daemon.kubernetes",
        ),
    ];
    for (what, text, path, says) in cases {
        let problems = config::parse(&text, home)
            .err()
            .unwrap_or_else(|| panic!("{what}: the file validates:\n{text}"));
        assert!(
            problems
                .iter()
                .any(|problem| problem.path == path && problem.message.contains(says)),
            "{what}: no problem at {path} that says {says:?}: {problems:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The pod, its Secret, its detail and the API
// ---------------------------------------------------------------------------------------------

/// The namespace of the daemon's cases.
const NAMESPACE: &str = "conductor-instances";

/// The values of the secrets the daemon's source holds.
const MODEL_TOKEN: &str = "model-token-value-0000";
const FORGE_TOKEN: &str = "forge-token-value-1111";
const CONTROL: &str = "control-token-value-2222";

/// One daemon case: its directory, its fake API server.
struct Pod {
    dir: PathBuf,
    fake: FakeKube,
    _scratch: Scratch,
}

impl Pod {
    fn new(name: &str) -> Self {
        active::isolate();
        let scratch = Scratch::new(Path::new("faa").join(name));
        let dir = scratch.path().to_owned();
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        for (secret, value) in [
            ("alpha-model", MODEL_TOKEN),
            ("forge-token", FORGE_TOKEN),
            ("gitlab-token", FORGE_TOKEN),
            ("helper-secret", FORGE_TOKEN),
            ("control", CONTROL),
        ] {
            fs::write(dir.join("secrets").join(secret), value).expect("write a secret");
        }
        let fake = FakeKube::start(&dir);
        Self {
            dir,
            fake,
            _scratch: scratch,
        }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// The config file: the daemon's `env` the entries `env`, and `instances`.
    fn config(&self, env: &[&str], instances: &str) {
        let env: String = env
            .iter()
            .map(|entry| format!("\n      - {entry}"))
            .collect();
        let text = format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: {store:?}\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 control_token: secret:control\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: registry.example.com/conductor:1.0.0\n\
             \x20   owner: example-owner\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G}}\n\
             \x20   env:{env}\n\
             instances:\n{instances}",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            secrets = self.dir.join("secrets"),
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

    fn object(&self, resource: &str, name: &str) -> Value {
        self.fake
            .object(NAMESPACE, resource, name)
            .unwrap_or_else(|| panic!("no {resource} {name}"))
    }

    /// The pod `conductor-instance-<name>` runs, its container ready.
    fn set_running(&self, name: &str) {
        let mut pod = self.object("pods", &format!("conductor-instance-{name}"));
        pod["status"] = json!({
            "phase": "Running",
            "podIP": "127.0.0.1",
            "containerStatuses": [
                {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
            ],
        });
        self.fake.put(NAMESPACE, "pods", pod);
    }
}

/// An instance mapping of the file: `name`, a model token, `sources`, and `extra` lines.
fn instance(name: &str, sources: &str, extra: &str) -> String {
    format!(
        "  - name: {name}\n\
         \x20   placement: none\n\
         \x20   session_prefix: {name}\n\
         \x20   sources: [{sources}]\n\
         \x20   model_token: secret:alpha-model\n\
         {extra}"
    )
}

/// The value of `key` of `secret`, from its base64 `data` or its `stringData`; none without one.
fn secret_value(secret: &Value, key: &str) -> Option<String> {
    if let Some(text) = secret["stringData"][key].as_str() {
        return Some(text.to_owned());
    }
    let encoded = secret["data"][key].as_str()?;
    Some(String::from_utf8(pods::base64_decode(encoded).expect("base64")).expect("UTF-8"))
}

/// The names of the variables of the pod's one container.
fn variables(pod: &Value) -> Vec<String> {
    pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("the container's env")
        .iter()
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect()
}

/// Whether `name` is a variable of git's configuration from the environment.
fn gits_config(name: &str) -> bool {
    name == "GIT_CONFIG_COUNT"
        || name.starts_with("GIT_CONFIG_KEY_")
        || name.starts_with("GIT_CONFIG_VALUE_")
}

/// The pod of an anonymous instance carries no forge token variable and no entry of git's
/// configuration, so no credential helper: not the daemon's `gh` or `glab` entries of
/// `daemon.kubernetes.env`, not the helper the daemon writes, not the installer's git
/// configuration; its Secret holds none of their secrets. Beside it, an instance without the
/// member keeps what it kept before. Read from the pod and the Secret the daemon created.
#[test]
fn the_pod_of_an_anonymous_instance_carries_no_forge_credential() {
    let installer = [
        "{name: GIT_CONFIG_COUNT, value: \"1\"}",
        "{name: GIT_CONFIG_KEY_0, value: credential.helper}",
        "{name: GIT_CONFIG_VALUE_0, secret: secret:helper-secret}",
    ];
    for (what, git) in [
        ("the daemon's helper", &[][..]),
        ("the installer's", &installer[..]),
    ] {
        let case = Pod::new(if git.is_empty() {
            "pod-helper"
        } else {
            "pod-installer"
        });
        let mut env = vec![
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GITHUB_TOKEN, secret: secret:forge-token}",
            "{name: GITLAB_TOKEN, secret: secret:gitlab-token}",
            "{name: GIT_AUTHOR_NAME, value: conductor}",
        ];
        env.extend(git);
        case.config(
            &env,
            &[
                instance(
                    "alpha",
                    "{github: example-owner, include: [one]}",
                    "\x20   forge_access: anonymous\n",
                ),
                instance("beta", "{github: example-owner}", ""),
            ]
            .concat(),
        );
        let mut daemon = case.daemon();
        daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("a pass");

        let alpha = case.object("pods", "conductor-instance-alpha");
        let carried = variables(&alpha);
        for variable in FORGE_CLI_TOKEN_VARIABLES
            .iter()
            .flat_map(|(_, variables)| variables.iter())
        {
            assert!(
                !carried.iter().any(|name| name == variable),
                "{what}: alpha carries {variable}: {carried:?}"
            );
        }
        assert!(
            !carried.iter().any(|name| gits_config(name)),
            "{what}: alpha carries git's configuration: {carried:?}"
        );
        assert!(
            !alpha.to_string().contains("credential"),
            "{what}: alpha names a credential: {alpha}"
        );
        assert!(
            carried.iter().any(|name| name == "GIT_AUTHOR_NAME"),
            "{what}: alpha keeps every other entry: {carried:?}"
        );
        let secret = case.object("secrets", "conductor-instance-alpha");
        for key in ["forge-token", "gitlab-token", "helper-secret"] {
            assert_eq!(
                secret_value(&secret, key),
                None,
                "{what}: alpha's Secret holds {key}"
            );
        }
        let held: Vec<String> = ["data", "stringData"]
            .iter()
            .filter_map(|field| secret[*field].as_object())
            .flat_map(|map| map.keys().cloned())
            .collect();
        for key in &held {
            assert_ne!(
                secret_value(&secret, key).as_deref(),
                Some(FORGE_TOKEN),
                "{what}: alpha's Secret holds a forge token under {key}"
            );
        }
        let rendered = secret_value(&secret, "conductor.yaml").expect("the rendered file");
        let file: serde_yaml::Value = serde_yaml::from_str(&rendered).expect("YAML");
        assert_eq!(
            file["instances"][0]["forge_access"].as_str(),
            Some("anonymous"),
            "{rendered}"
        );

        // Beside it, the instance without the member, as before.
        let beta = case.object("pods", "conductor-instance-beta");
        let kept = variables(&beta);
        for variable in ["GH_TOKEN", "GITHUB_TOKEN", "GIT_CONFIG_COUNT"] {
            assert!(
                kept.iter().any(|name| name == variable),
                "{what}: beta keeps {variable}: {kept:?}"
            );
        }
        let secret = case.object("secrets", "conductor-instance-beta");
        assert_eq!(
            secret_value(&secret, "forge-token").as_deref(),
            Some(FORGE_TOKEN)
        );
        let rendered = secret_value(&secret, "conductor.yaml").expect("the rendered file");
        assert!(!rendered.contains("forge_access"), "{rendered}");
    }
}

/// `GET /v1/instances/{name}` with the control token over the API `port`; the JSON row.
fn get(port: u16, name: &str) -> Value {
    let (status, body) = call(port, "GET", &format!("/v1/instances/{name}"), None);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap_or_else(|_| panic!("a JSON row: {body}"))
}

/// One request with the control token to the daemon's API on `port`: its status and body.
fn call(port: u16, method: &str, target: &str, body: Option<&Value>) -> (u16, String) {
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    write!(
        stream,
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
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

/// The instance's `detail` carries one line that says the instance writes nothing to its forge,
/// whether its pod starts or runs, after any other line; one without the member carries none.
/// `GET /v1/instances/{name}` answers the member, and none where the mapping holds none. A `PUT`
/// of a mapping the member refuses is refused with `ConfigInvalid` at the path.
#[test]
fn the_detail_and_the_api_answer_the_member() {
    let case = Pod::new("api");
    case.config(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &[
            instance(
                "alpha",
                "{github: example-owner, include: [one]}",
                "\x20   forge_access: anonymous\n",
            ),
            instance("beta", "{github: example-owner}", ""),
        ]
        .concat(),
    );
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    let detail = |daemon: &Daemon<Manager>, name: &str| {
        daemon
            .rows()
            .into_iter()
            .find(|row| row["name"] == name)
            .expect("the row")["detail"]
            .clone()
    };
    let lines = |detail: &Value| -> Vec<String> {
        detail
            .as_str()
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains(WRITES_NOTHING))
            .map(str::to_owned)
            .collect()
    };
    let starting = detail(&daemon, "alpha");
    assert_eq!(lines(&starting).len(), 1, "starting: {starting}");
    assert!(lines(&detail(&daemon, "beta")).is_empty());
    case.set_running("alpha");
    daemon
        .pass(ReconcileReason::Cadence, start + Duration::from_secs(2))
        .expect("a pass");
    let running = detail(&daemon, "alpha");
    assert_eq!(lines(&running).len(), 1, "running: {running}");
    assert_eq!(
        running
            .as_str()
            .unwrap_or_default()
            .lines()
            .last()
            .unwrap_or_default(),
        lines(&running)[0],
        "the line stands after any other"
    );

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

    let alpha = get(port, "alpha");
    assert_eq!(alpha["forge_access"], "anonymous", "{alpha}");
    assert_eq!(
        lines(&alpha["detail"]).len(),
        1,
        "the row's detail: {alpha}"
    );
    let beta = get(port, "beta");
    assert!(beta.get("forge_access").is_none(), "{beta}");

    let refused = json!({
        "mapping": {
            "name": "alpha",
            "placement": "none",
            "session_prefix": "alpha",
            "sources": [{"github": "example-owner", "include": ["one"]}],
            "model_token": "secret:alpha-model",
            "forge_access": "anonymous",
            "forge_token": "secret:forge-token",
        },
        "reason": "a forge token beside anonymous access",
    });
    let (status, body) = call(port, "PUT", "/v1/instances/alpha", Some(&refused));
    assert_eq!(status, 422, "{body}");
    assert!(body.contains("ConfigInvalid"), "{body}");
    assert!(body.contains("instances[0].forge_token"), "{body}");
}

// ---------------------------------------------------------------------------------------------
// The runtime: its clone and its collect pass
// ---------------------------------------------------------------------------------------------

/// The clock of every snapshot.
const NOW: &str = "2026-10-07T12:00:00Z";

/// The variable that tells the runtime it runs alone in a pod.
const LISTEN: &str = "CONDUCTOR_API_LISTEN";

/// A token a stray variable of the runtime's environment holds, built from pieces: the commit
/// hook's scanner refuses a literal that looks like a token.
fn stray() -> String {
    ["stray", "-", "value", "-", "4Kd9", "Wq2"].concat()
}

/// One runtime case: `home/`, the checkouts root `root/`, `trees/`, `records/`, `state/`,
/// `cache/`, `bin/`, `forge/` for the bare repositories and an empty `work/`.
struct Runtime {
    dir: PathBuf,
    _scratch: Scratch,
}

impl Runtime {
    fn new(name: &str) -> Self {
        active::isolate();
        let scratch = Scratch::new(Path::new("faa").join(name));
        let dir = fs::canonicalize(scratch.path()).expect("the case's directory");
        for sub in ["home", "root", "trees", "work", "bin", "forge"] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::create_dir_all(dir.join("records/.engineering")).expect("create the records");
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the records' workspace file");
        let case = Self {
            dir,
            _scratch: scratch,
        };
        case.programs();
        case
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    fn script(&self, name: &str, script: &str) {
        let file = self.bin().join(name);
        fs::write(&file, script).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// `gh`, which logs its words and answers nothing; `git`, which logs its words and whether a
    /// forge token reached it, then runs the real one; fakes of `aep`, `claude` and `ess`; and
    /// the real `tar`.
    fn programs(&self) {
        self.script(
            "gh",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\necho 'no answer' >&2\nexit 1\n",
                text(&self.log("gh"))
            ),
        );
        let tokens: String = FORGE_CLI_TOKEN_VARIABLES
            .iter()
            .flat_map(|(_, variables)| variables.iter())
            .map(|variable| format!("${{{variable}:+{variable}}}"))
            .collect();
        self.script(
            "git",
            &format!(
                "#!/bin/sh\nprintf '%s [token:%s]\\n' \"$*\" \"{tokens}\" >> '{}'\nexec '{}' \
                 \"$@\"\n",
                text(&self.log("git")),
                text(&which("git"))
            ),
        );
        self.script(
            "aep",
            "#!/bin/sh\ncase \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf \
             '%s' '{\"artifacts\": []}' ;;\nesac\n",
        );
        self.script("claude", "#!/bin/sh\nprintf '%s' '[]'\n");
        self.script(
            "ess",
            "#!/bin/sh\ncase \"$1\" in\n  specify) exit 0 ;;\n  *) echo '1 scenario(s) (0 \
             authored), 0 refusal(s), written to x' ;;\nesac\n",
        );
        std::os::unix::fs::symlink(which("tar"), self.bin().join("tar")).expect("link tar");
    }

    /// The bare repository `example-owner/<name>` on the forge, with one commit on `main`.
    fn bare(&self, name: &str) {
        let bare = self
            .dir
            .join("forge/github/example-owner")
            .join(format!("{name}.git"));
        fs::create_dir_all(&bare).expect("create the bare repository");
        git(
            &bare,
            &["init", "--quiet", "--bare", "--initial-branch=main"],
        );
        let work = self.dir.join("pushing");
        let _ = fs::remove_dir_all(&work);
        git(
            &self.dir,
            &[
                "clone",
                "--quiet",
                &format!("file://{}", text(&bare)),
                &text(&work),
            ],
        );
        fs::write(work.join("start.txt"), "start").expect("write a file");
        git(&work, &["add", "--all"]);
        git(&work, &["commit", "--quiet", "-m", "Start"]);
        git(&work, &["push", "--quiet", "origin", "HEAD:main"]);
        fs::remove_dir_all(&work).expect("remove the pushing clone");
    }

    /// The config file of the instance `alpha`, whose sources include `one` and `two` of
    /// `example-owner`, `extra` lines appended.
    fn config(&self, extra: &str) -> PathBuf {
        self.config_including("[one, two]", extra)
    }

    /// The config file of the instance `alpha`, whose source includes `include` of
    /// `example-owner`, `extra` lines appended.
    fn config_including(&self, include: &str, extra: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: alpha\n    sources: [{{github: \
             example-owner, include: {include}}}]\n    checkouts:\n      root: {}\n      \
             trees: {}\n    records: {}\n    state: {}\n    cache: {}\n{extra}",
            quoted(&self.root()),
            quoted(&self.dir.join("trees")),
            quoted(&self.dir.join("records")),
            quoted(&self.dir.join("state")),
            quoted(&self.dir.join("cache")),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// The environment of every run: `HOME`, `PATH` only `bin/`, the config file, a git identity,
    /// git's rewrite of GitHub's HTTPS origin to the bare repositories, the pod's variable, and a
    /// stray forge token.
    fn environment(&self, config: &Path, command: &mut Command) {
        command
            .env_clear()
            .env("HOME", self.dir.join("home"))
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!(
                    "url.file://{}/github/.insteadOf",
                    text(&self.dir.join("forge"))
                ),
            )
            .env("GIT_CONFIG_VALUE_0", "https://github.com/")
            .env(LISTEN, "127.0.0.1:0")
            .env("GH_TOKEN", stray());
    }

    /// One collect pass, `snapshot start-snapshot` at [`NOW`].
    fn pass(&self, config: &Path) -> Output {
        let mut command = common::conductor(self.dir.join("home"));
        self.environment(config, &mut command);
        command
            .current_dir(self.dir.join("work"))
            .args([
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// The rows of the view `view` of the snapshot `snapshot`.
    fn rows(&self, config: &Path, view: &str, snapshot: &str) -> Vec<Value> {
        let mut command = common::conductor(self.dir.join("home"));
        self.environment(config, &mut command);
        let output = command
            .current_dir(self.dir.join("work"))
            .args(["snapshot", view, "--format", "jsonl"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(output.status.success(), "{view}: {}", describe(&output));
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one JSON object a line"))
            .filter(|row| row["snapshot_id"].as_str() == Some(snapshot))
            .collect()
    }

    /// The runtime's start, `instance serve` with its loop off, until it says its seeding ended;
    /// what it printed.
    fn start(&self, config: &Path) -> String {
        let mut command = common::conductor(self.dir.join("home"));
        self.environment(config, &mut command);
        command
            .current_dir(self.dir.join("work"))
            .args(["instance", "serve"])
            .env(conductor_cli::instance::TOKEN_VARIABLE, "instance-token")
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut serve = Serve::spawn(command);
        serve.until(|printed| {
            printed.contains("seeded its instance") || printed.contains("the seeding failed")
        })
    }

    fn lines(&self, program: &str) -> Vec<String> {
        fs::read_to_string(self.log(program))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }
}

/// `instance serve`, killed when dropped, with what it printed.
struct Serve {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Serve {
    fn spawn(mut command: Command) -> Self {
        let mut child = command.spawn().expect("instance serve starts");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            std::thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        Self { child, printed }
    }

    fn until(&mut self, holds: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let printed = self
                .printed
                .lock()
                .map(|printed| printed.clone())
                .unwrap_or_default();
            if holds(&printed) {
                return printed;
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                panic!("instance serve exited {status}: {printed}");
            }
            assert!(Instant::now() < deadline, "never held: {printed}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The git lines of a clone.
fn clones(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|line| line.starts_with("clone ") || line.contains(" clone "))
        .collect()
}

/// At its start the runtime of an anonymous instance clones each included repository over HTTPS
/// with no credential: no credential helper and no forge token reach git, though one stray
/// variable holds one. A clone that fails says the repository is not public or not reachable,
/// with git's last error line. Nothing asks the forge's command line for a repository.
#[test]
fn the_start_clones_each_included_repository_with_no_credential() {
    let case = Runtime::new("start");
    case.bare("one");
    let config = case.config("    forge_access: anonymous\n");
    let printed = case.start(&config);
    assert!(
        case.root().join("one/.git").is_dir(),
        "one is cloned: {printed}"
    );
    assert!(!case.root().join("two").exists(), "{printed}");
    let failed: Vec<&str> = printed
        .lines()
        .filter(|line| line.contains(": two: "))
        .collect();
    assert_eq!(failed.len(), 1, "{printed}");
    assert!(failed[0].contains(NOT_PUBLIC), "{}", failed[0]);
    assert!(
        failed[0].contains("https://github.com/example-owner/two.git"),
        "{}",
        failed[0]
    );
    assert!(
        failed[0].contains(GITS_LAST),
        "git's last line: {}",
        failed[0]
    );
    let lines = case.lines("git");
    let cloned = clones(&lines);
    // The start brings the checkouts and its seeding pass brings them again, so `two`, whose
    // clone failed, is tried once more; `one` is cloned once.
    for (name, times) in [("one", 1..=1), ("two", 1..=2)] {
        let url = format!("https://github.com/example-owner/{name}.git ");
        let tried = cloned.iter().filter(|line| line.contains(&url)).count();
        assert!(
            times.contains(&tried),
            "{name} cloned {tried} times: {lines:?}"
        );
    }
    assert!(cloned.len() <= 3, "{lines:?}");
    for line in cloned {
        assert!(line.contains("credential.helper="), "{line}");
        assert!(line.ends_with("[token:]"), "a token reached git: {line}");
        assert!(line.contains("https://github.com/example-owner/"), "{line}");
    }
    // The start's own report asks no forge command line anything, not its version or its login
    // (story:anonymous-pod-no-forge-secret): the instance needs none.
    assert_eq!(
        case.lines("gh"),
        Vec::<String>::new(),
        "gh is run: {printed}"
    );
}

/// The words the watch's one line of an anonymous instance says, as the collectors' line does.
const WATCH_SKIPPED: &str = "conductor watch: forge_access: anonymous: this pass reads no pull \
                             request, issue, release or workflow run of the forge";

/// The watch of an anonymous instance makes no call of a forge's command line: one pass over a
/// store whose newest snapshot holds an Active repository reads no `main` CI and no pull
/// request, runs no `gh`, and says so in one line (story:anonymous-pod-no-forge-secret).
#[test]
fn the_watch_of_an_anonymous_instance_runs_no_gh_and_says_so_once() {
    let case = Runtime::new("watch");
    case.bare("one");
    let config = case.config_including("[one]", "    forge_access: anonymous\n");
    // A snapshot that starts now, so the commit of `one` makes it Active.
    let now = time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("a whole second")
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC 3339");
    let mut command = common::conductor(case.dir.join("home"));
    case.environment(&config, &mut command);
    let output = command
        .current_dir(case.dir.join("work"))
        .args([
            "snapshot",
            "start-snapshot",
            "--started-at",
            &now,
            "--disk-free-bytes",
            "100000000000",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let snapshot = String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let rows = case.rows(&config, "repositories", &snapshot);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(case.lines("gh"), Vec::<String>::new());

    let mut command = common::conductor(case.dir.join("home"));
    case.environment(&config, &mut command);
    command
        .current_dir(case.dir.join("work"))
        .args(["watch", "run", "--follow", "--every", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut watch = Serve::spawn(command);
    // The first pass reads main's CI; the passes after it, within `ci_every`, do not.
    watch.until(|printed| printed.contains(WATCH_SKIPPED) || printed.contains("not read"));
    // Two more passes of one second each, which read no CI.
    std::thread::sleep(Duration::from_secs(3));
    let printed = watch.until(|_| true);
    drop(watch);
    assert_eq!(case.lines("gh"), Vec::<String>::new(), "{printed}");
    assert!(!printed.contains("not read"), "{printed}");
    assert_eq!(
        printed.matches(WATCH_SKIPPED).count(),
        1,
        "one line in the CI pass: {printed}"
    );
}

/// A collect pass of an anonymous instance completes: it clones what has no checkout, records
/// each repository with a checkout, records the one that fails as one member failure that says
/// it is not public or not reachable, runs no `gh`, and says once that it reads no pull
/// request, issue, release or run of the forge.
#[test]
fn a_pass_reads_no_pull_request_or_issue_and_says_so_once() {
    let case = Runtime::new("pass");
    case.bare("one");
    let config = case.config("    forge_access: anonymous\n");
    let output = case.pass(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let snapshot = String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let said: Vec<&str> = stderr.lines().filter(|line| line.contains(NAMED)).collect();
    assert_eq!(said.len(), 1, "{stderr}");
    for words in ["pull request", "issue"] {
        assert!(said[0].contains(words), "{}", said[0]);
    }
    assert_eq!(case.lines("gh"), Vec::<String>::new(), "gh is not run");
    let rows = case.rows(&config, "repositories", &snapshot);
    let names: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["repository"].as_str())
        .collect();
    assert_eq!(names, ["one"], "{rows:?}");
    assert_eq!(rows[0]["local_checkout"], true);
    assert_eq!(rows[0]["open_issues"], 0);
    let failures = case.rows(&config, "member-failures", &snapshot);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0]["member"], "two");
    let line = failures[0]["error_line"].as_str().unwrap_or_default();
    assert!(line.contains(NOT_PUBLIC), "{line}");
    assert!(line.contains(GITS_LAST), "git's last line: {line}");
    for line in clones(&case.lines("git")) {
        assert!(line.contains("credential.helper="), "{line}");
        assert!(line.ends_with("[token:]"), "a token reached git: {line}");
    }
}

/// The anonymous clone runs with no `insteadOf` rewrite and no extra header of the system's or
/// the user's git configuration (story:anonymous-pod-no-forge-secret): here both name a rewrite
/// of the owner's origin to a place that holds nothing, which would win over the environment's
/// rewrite to the forge as the longer match, and an extra header. `git` logs the configuration
/// it reads for each clone; the clone of `one` succeeds and reads neither.
#[test]
fn the_anonymous_clone_reads_no_rewrite_and_no_extra_header_of_gits_configuration() {
    let case = Runtime::new("rewrite");
    case.bare("one");
    let nowhere = format!("file://{}/", text(&case.dir.join("nowhere")));
    let rewrite = |header: &str| {
        format!(
            "[url \"{nowhere}\"]\n\tinsteadOf = https://github.com/example-owner/\n[http]\n\t\
             extraHeader = X-Example: {header}\n"
        )
    };
    fs::write(case.dir.join("home/.gitconfig"), rewrite("from-global")).expect("write it");
    let system = case.dir.join("gitconfig-system");
    fs::write(&system, rewrite("from-system")).expect("write it");
    case.script(
        "git",
        &format!(
            "#!/bin/sh\nset -f\ndump() {{\n  pre=''\n  while [ \"$1\" = -c ]; do pre=\"$pre -c \
             $2\"; shift 2; done\n  printf 'clone reads:\\n' >> '{dump}'\n  '{git}' $pre config \
             --show-origin --get-regexp 'insteadof|extraheader' >> '{dump}' 2>&1\n}}\ncase \" $* \
             \" in *' clone '*) dump \"$@\" ;; esac\nexec '{git}' \"$@\"\n",
            dump = text(&case.log("gitconfig")),
            git = text(&which("git")),
        ),
    );
    let config = case.config_including("[one]", "    forge_access: anonymous\n");
    let mut command = common::conductor(case.dir.join("home"));
    case.environment(&config, &mut command);
    let output = command
        .env("GIT_CONFIG_SYSTEM", &system)
        .current_dir(case.dir.join("work"))
        .args([
            "snapshot",
            "start-snapshot",
            "--started-at",
            NOW,
            "--disk-free-bytes",
            "100000000000",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    let read = fs::read_to_string(case.log("gitconfig")).unwrap_or_default();
    assert!(
        read.contains("clone reads:"),
        "no clone ran: {}",
        describe(&output)
    );
    for words in ["nowhere", "X-Example"] {
        assert!(!read.contains(words), "the clone reads {words:?}:\n{read}");
    }
    assert!(
        case.root().join("one/.git").is_dir(),
        "one is cloned: {}\n{read}",
        describe(&output)
    );
}

/// An `include` entry validation accepts and no repository name can be (`two three`) is one
/// member failure of an anonymous instance's pass, which records every other repository.
#[test]
fn an_include_entry_that_names_no_repository_is_one_member_failure() {
    let case = Runtime::new("include-name");
    case.bare("one");
    let config = case.config_including("[one, \"two three\"]", "    forge_access: anonymous\n");
    let output = case.pass(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let snapshot = String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let rows = case.rows(&config, "repositories", &snapshot);
    let names: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["repository"].as_str())
        .collect();
    assert_eq!(names, ["one"], "{rows:?}\n{}", describe(&output));
    let failures = case.rows(&config, "member-failures", &snapshot);
    let named: Vec<&Value> = failures
        .iter()
        .filter(|failure| failure["member"] == "two three")
        .collect();
    assert_eq!(named.len(), 1, "{failures:?}\n{}", describe(&output));
    let line = named[0]["error_line"].as_str().unwrap_or_default();
    assert!(line.contains("is not a repository name"), "{line}");
}

/// Without the member, a pod that carries no token of its forge tries no clone, as before: each
/// repository is a failure that says so.
#[test]
fn without_the_member_a_pod_without_a_token_tries_no_clone() {
    let case = Runtime::new("absent");
    case.bare("one");
    // Without the member the owner's list is read from the forge, as before.
    case.script(
        "gh",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1 $2\" in\n  'repo list') printf \
             '%s\\n' '[{{\"name\":\"one\",\"visibility\":\"PUBLIC\",\"isArchived\":false,\
             \"hasIssuesEnabled\":false}},{{\"name\":\"two\",\"visibility\":\"PUBLIC\",\
             \"isArchived\":false,\"hasIssuesEnabled\":false}}]' ;;\n  *) printf '%s\\n' '[]' \
             ;;\nesac\n",
            text(&case.log("gh"))
        ),
    );
    let config = case.config("");
    let mut command = common::conductor(case.dir.join("home"));
    case.environment(&config, &mut command);
    command
        .env_remove("GH_TOKEN")
        .current_dir(case.dir.join("work"))
        .args(["instance", "serve"])
        .env(conductor_cli::instance::TOKEN_VARIABLE, "instance-token")
        .env(conductor_cli::instance::LOOP_VARIABLE, "off")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut serve = Serve::spawn(command);
    let printed = serve.until(|printed| {
        printed.contains("seeded its instance") || printed.contains("the seeding failed")
    });
    assert!(!case.root().join("one").exists(), "{printed}");
    assert!(clones(&case.lines("git")).is_empty(), "{printed}");
    let refused: Vec<&str> = printed
        .lines()
        .filter(|line| line.contains("the pod carries no token of github.com (GH_TOKEN)"))
        .collect();
    assert_eq!(refused.len(), 2, "{printed}");
    assert!(!printed.contains(NOT_PUBLIC), "{printed}");
}

// ---------------------------------------------------------------------------------------------
// The writes
// ---------------------------------------------------------------------------------------------

/// The guard's places of the anonymous instance `alpha` under `route`, its checkouts root
/// `root/` and records `records/` in `dir`, as the hook builds them in its pod.
fn places(dir: &Path, route: &str, anonymous: bool) -> Places {
    let access = if anonymous {
        "\x20   forge_access: anonymous\n"
    } else {
        ""
    };
    let text = daemon_file(
        true,
        "{github: example-owner, include: [one]}",
        &format!("{access}\x20   forge: {{route: {route}}}\n"),
    );
    let file = dir.join("conductor.yaml");
    fs::write(&file, &text).expect("write the config file");
    let parsed = config::parse(&text, &dir.join("home")).unwrap_or_else(|p| panic!("{p:?}"));
    let mut instance = parsed.instances[0].clone();
    instance.checkouts.root = text_of(&dir.join("root"));
    instance.checkouts.trees = text_of(&dir.join("trees"));
    instance.records = text_of(&dir.join("records"));
    instance.state = text_of(&dir.join("state"));
    instance.cache = text_of(&dir.join("cache"));
    let active = config::Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
        daemon_port: config::DAEMON_PORT,
    };
    Places::active(&dir.join("home"), &active).with_config_file(&file)
}

/// The guard's decision on `command` run in `cwd`.
fn decide(places: &Places, cwd: &Path, command: &str) -> guard::Decision {
    let payload = json!({
        "session_id": "00000000-0000-4000-8000-0000000000a1",
        "cwd": text_of(cwd),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
    });
    guard::decide_in(&payload, places, &SessionList::default())
}

/// Every forge write of a session of an anonymous instance, a controller's in its checkout or
/// conductor's in its records, under either route, is denied before it runs, naming
/// `forge_access: anonymous`: a push (plain or through the bot route), a pull request, a comment,
/// a release through the bot route, a GitLab write. The same pull request under `direct` without
/// the member is allowed, so the denial is the member's.
#[test]
fn every_forge_write_of_a_session_is_denied_naming_the_member() {
    active::isolate();
    let scratch = Scratch::new(Path::new("faa").join("guard"));
    let dir = fs::canonicalize(scratch.path()).expect("the case's directory");
    for sub in ["home", "root/one", "trees", "records", "state", "cache"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    for checkout in [dir.join("root/one"), dir.join("records")] {
        git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
        git(
            &checkout,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/example-owner/one.git",
            ],
        );
    }
    let writes = [
        "git push origin main",
        "git -c push.default=current push",
        "b10x-gates bot -- push origin main",
        "gh pr create -R example-owner/one --fill",
        "gh pr comment 7 -R example-owner/one --body done",
        "b10x-gates gh --repository example-owner/one -- release create v1.0.0",
        "glab mr note 7 -R example-group/one -m done",
    ];
    for route in ["gates", "direct"] {
        let anonymous = places(&dir, route, true);
        for cwd in [dir.join("root/one"), dir.join("records")] {
            for command in writes {
                let decision = decide(&anonymous, &cwd, command);
                assert_eq!(
                    decision.verdict,
                    Verdict::Deny,
                    "{route}, {}: {command}: {}",
                    cwd.display(),
                    decision.reason
                );
                assert!(
                    decision.reason.contains(NAMED),
                    "{route}: {command}: {}",
                    decision.reason
                );
            }
            let read = decide(&anonymous, &cwd, "git log -1 --oneline");
            assert_eq!(read.verdict, Verdict::Allow, "{}", read.reason);
        }
    }
    let direct = places(&dir, "direct", false);
    let allowed = decide(
        &direct,
        &dir.join("root/one"),
        "gh pr create -R example-owner/one --fill",
    );
    assert_eq!(allowed.verdict, Verdict::Allow, "{}", allowed.reason);
}

/// The variables the recording `git` of [`every_remote_git_of_an_anonymous_snapshot_is_hardened`]
/// writes, each as `NAME=value`, `unset` where it is not set.
const RECORDED: [&str; 9] = [
    "GIT_NO_LAZY_FETCH",
    "GIT_SSH_COMMAND",
    "NETRC",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_NOSYSTEM",
    "GIT_TEMPLATE_DIR",
    "GIT_ASKPASS",
    "SSH_ASKPASS",
];

/// One run of the recording `git`: its words, the [`RECORDED`] variables, whether a forge token
/// reached it, and the directory it ran in (`-C` counted).
struct GitRun {
    words: Vec<String>,
    variables: Vec<String>,
    token: bool,
    dir: String,
}

impl GitRun {
    fn read(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.strip_prefix("args:")?.split('\t').skip(1).collect();
        let (words, rest) = fields.split_at(fields.iter().position(|field| *field == "env:")?);
        let variables: Vec<String> = rest[1..=RECORDED.len()]
            .iter()
            .map(|field| (*field).to_owned())
            .collect();
        let token = !rest[RECORDED.len() + 1].strip_prefix("token=")?.is_empty();
        let mut dir = rest[RECORDED.len() + 2].strip_prefix("pwd=")?.to_owned();
        if let Some(at) = words.iter().position(|word| *word == "-C") {
            dir = (*words.get(at + 1)?).to_owned();
        }
        Some(Self {
            words: words.iter().map(|word| (*word).to_owned()).collect(),
            variables,
            token,
            dir,
        })
    }

    /// Its subcommand and the word after it: the first word that is no option of git's own.
    fn subcommand(&self) -> (Option<&str>, Option<&str>) {
        let mut at = 0;
        while let Some(word) = self.words.get(at) {
            if ["-c", "-C", "--git-dir", "--work-tree", "--namespace"].contains(&word.as_str()) {
                at += 2;
            } else if word.starts_with('-') {
                at += 1;
            } else {
                return (Some(word), self.words.get(at + 1).map(String::as_str));
            }
        }
        (None, None)
    }

    /// Whether it talks to a remote: a clone, fetch, pull, ls-remote or push, `remote update` or
    /// `show`, or `submodule update`.
    fn talks_to_a_remote(&self) -> bool {
        matches!(
            self.subcommand(),
            (
                Some("clone" | "fetch" | "pull" | "ls-remote" | "push" | "fetch-pack"),
                _
            ) | (Some("remote"), Some("update" | "show"))
                | (Some("submodule"), Some("update"))
        )
    }

    /// What of the hardened route it lacks.
    fn lacks(&self) -> Vec<String> {
        let mut lacking: Vec<String> = [
            "credential.helper=",
            "http.extraHeader=",
            "core.askPass=",
            "fetch.recurseSubmodules=false",
            "submodule.recurse=false",
        ]
        .into_iter()
        .filter(|word| !self.words.iter().any(|held| held == word))
        .map(str::to_owned)
        .collect();
        for wanted in [
            "NETRC=/dev/null",
            "GIT_CONFIG_GLOBAL=/dev/null",
            "GIT_CONFIG_SYSTEM=/dev/null",
            "GIT_CONFIG_NOSYSTEM=1",
            "GIT_TEMPLATE_DIR=",
            "GIT_ASKPASS=",
            "SSH_ASKPASS=unset",
        ] {
            if !self.variables.iter().any(|held| held == wanted) {
                lacking.push(wanted.to_owned());
            }
        }
        if self.token {
            lacking.push("no forge token".to_owned());
        }
        lacking
    }
}

/// Three whole snapshots of an anonymous instance, the first cloning `one`, the others fetching
/// it, with a recording `git` on `PATH` and an `aep` whose workspace names `one`, so that the
/// blockers collector reads it too: every git command runs with `GIT_NO_LAZY_FETCH=1`, so none
/// fetches what a partial clone lacks by itself; every one that talks to a remote runs with the
/// hardened route's words (no submodule fetched either) and environment and no forge token; and
/// every fetch runs after the checkout's own configuration was read for the keys a fetch would
/// send, in that checkout. The snapshots fetch `one` at least once. The home's git configuration
/// names a `core.sshCommand`, the checkout's own names another before the second snapshot, and
/// the caller sets `GIT_SSH_COMMAND` for the third: every git command, local ones included, and
/// the `aep` the blockers collector starts get `GIT_SSH_COMMAND` as plain `ssh` with
/// [`conductor_cli::collect::SSH_OPTIONS`], which git reads before `core.sshCommand`.
#[test]
fn every_remote_git_of_an_anonymous_snapshot_is_hardened() {
    let case = Runtime::new("hardened");
    case.bare("one");
    fs::write(
        case.dir.join("home/.gitconfig"),
        "[core]\n\tsshCommand = ssh -i /srv/example/home-key\n",
    )
    .expect("write the home's git configuration");
    let tokens: String = FORGE_CLI_TOKEN_VARIABLES
        .iter()
        .flat_map(|(_, variables)| variables.iter())
        .map(|variable| format!("${{{variable}:+{variable}}}"))
        .collect();
    let variables: String = RECORDED
        .iter()
        .map(|name| format!("printf '\\t{name}=%s' \"${{{name}-unset}}\"\n"))
        .collect();
    case.script(
        "git",
        &format!(
            "#!/bin/sh\n{{\nprintf 'args:'\nfor word in \"$@\"; do printf '\\t%s' \"$word\"; \
             done\nprintf '\\tenv:'\n{variables}printf '\\ttoken=%s' \"{tokens}\"\nprintf \
             '\\tpwd=%s\\n' \"$(pwd)\"\n}} >> '{}'\nexec '{}' \"$@\"\n",
            text(&case.log("git")),
            text(&which("git"))
        ),
    );
    case.script(
        "aep",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"${{GIT_SSH_COMMAND-unset}}\" >> '{}'\ncase \"$3\" in\n  \
             members) printf '%s' '{{\"members\": [{{\"name\": \"one\", \"source\": \"one\", \
             \"tree\": {}}}]}}' ;;\n  *) printf '%s' '{{\"artifacts\": []}}' ;;\nesac\n",
            text(&case.log("aep")),
            quoted(&case.root().join("one"))
        ),
    );
    let config = case.config_including("[one]", "    forge_access: anonymous\n");
    let ssh = format!("ssh {}", conductor_cli::collect::SSH_OPTIONS);
    // The clone with the home's `core.sshCommand`; a fetch with the checkout's own too; a fetch
    // with the caller's `GIT_SSH_COMMAND` too.
    for pass in 0..3 {
        if pass == 1 {
            git(
                &case.root().join("one"),
                &["config", "core.sshCommand", "ssh -i /srv/example/own-key"],
            );
        }
        let mut command = common::conductor(case.dir.join("home"));
        case.environment(&config, &mut command);
        if pass == 2 {
            command.env("GIT_SSH_COMMAND", "ssh -i /srv/example/caller-key");
        }
        let output = command
            .current_dir(case.dir.join("work"))
            .args([
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert_eq!(
            output.status.code(),
            Some(0),
            "pass {pass}: {}",
            describe(&output)
        );
    }
    let runs: Vec<GitRun> = case
        .lines("git")
        .iter()
        .filter_map(|line| GitRun::read(line))
        .collect();
    assert!(!runs.is_empty(), "git was not recorded");
    let mut wrong = Vec::new();
    let mut fetches = 0;
    for (at, run) in runs.iter().enumerate() {
        // No git command of the instance fetches what a partial clone lacks by itself.
        if !run
            .variables
            .iter()
            .any(|held| held == "GIT_NO_LAZY_FETCH=1")
        {
            wrong.push(format!("{:?} lacks GIT_NO_LAZY_FETCH=1", run.words));
        }
        // No `core.sshCommand` of the home's configuration or of the checkout's own, and no
        // `GIT_SSH_COMMAND` of the caller's, reaches any git command: ssh as it is, never asking.
        let held = run
            .variables
            .iter()
            .find_map(|held| held.strip_prefix("GIT_SSH_COMMAND="))
            .unwrap_or_default();
        if held != ssh {
            wrong.push(format!("{:?} has GIT_SSH_COMMAND={held}", run.words));
        }
        if !run.talks_to_a_remote() {
            continue;
        }
        let lacking = run.lacks();
        if !lacking.is_empty() {
            wrong.push(format!("{:?} lacks {lacking:?}", run.words));
        }
        if run.subcommand().0 == Some("fetch") {
            fetches += 1;
            let read = runs[..at].iter().rev().any(|earlier| {
                earlier.dir == run.dir
                    && earlier.subcommand().0 == Some("config")
                    && earlier.words.iter().any(|word| word == "--includes")
                    && earlier.words.iter().any(|word| word == "--get-regexp")
            });
            if !read {
                wrong.push(format!(
                    "{:?} in {} without a read of the checkout's own configuration",
                    run.words, run.dir
                ));
            }
        }
    }
    assert!(
        fetches >= 1,
        "no fetch was recorded: {:?}",
        case.lines("git")
    );
    let aep = case.lines("aep");
    assert!(!aep.is_empty(), "aep was not recorded");
    for held in &aep {
        if *held != ssh {
            wrong.push(format!("aep has GIT_SSH_COMMAND={held}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The records of an anonymous instance are committed and not pushed: the push is refused
/// before git runs it, in one line that names `forge_access: anonymous`, and `origin` receives
/// nothing.
#[test]
fn the_records_push_is_refused_before_git_runs_it() {
    active::isolate();
    let scratch = Scratch::new(Path::new("faa").join("records"));
    let dir = fs::canonicalize(scratch.path()).expect("the case's directory");
    let records = dir.join("records");
    let origin = dir.join("origin.git");
    fs::create_dir_all(records.join("decisions")).expect("create the records");
    fs::create_dir_all(&origin).expect("create the origin");
    git(
        &origin,
        &["init", "--quiet", "--bare", "--initial-branch=main"],
    );
    git(&records, &["init", "--quiet", "--initial-branch=main"]);
    fs::write(records.join("STATUS.md"), "# Status\n").expect("write the status");
    git(&records, &["add", "--all"]);
    git(&records, &["commit", "--quiet", "-m", "records"]);
    git(
        &records,
        &[
            "remote",
            "add",
            "origin",
            &format!("file://{}", text(&origin)),
        ],
    );
    // The commit runs in this process, which carries no git identity of its own.
    git(&records, &["config", "user.name", "Fixture"]);
    git(
        &records,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&records, &["config", "commit.gpgsign", "false"]);
    fs::write(records.join("decisions/2026-10.jsonl"), "{}\n").expect("write a decision");
    let route = Route::new(
        ForgeRoute::Direct,
        Path::new("true"),
        None,
        Some(&dir.join("state")),
    )
    .with_forge_access(Some(ForgeAccess::Anonymous));
    let committed = records::commit_records(&records, &route, "2026-10-07");
    match &committed {
        Committed::Refused {
            reason,
            line,
            commit,
        } => {
            assert_eq!(*reason, "push", "{committed:?}");
            assert!(line.contains(NAMED), "{line}");
            assert!(commit.is_some(), "{committed:?}");
        }
        other => panic!("not refused: {other:?}"),
    }
    let output = Command::new(which("git"))
        .args(["rev-parse", "--verify", "--quiet", "refs/heads/main"])
        .current_dir(&origin)
        .output()
        .expect("git runs");
    assert!(!output.status.success(), "origin received main");
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn text(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

fn text_of(path: &Path) -> String {
    text(path)
}

/// `path` as a YAML scalar: a JSON string.
fn quoted(path: &Path) -> String {
    Value::from(text(path)).to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The program `name` on this process's `PATH`.
fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

/// `git <args>` in `dir`, with no configuration of the user's, which must succeed.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new(which("git"))
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_COUNT")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}
