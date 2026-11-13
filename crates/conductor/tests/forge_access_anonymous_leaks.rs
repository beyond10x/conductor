//! story:anonymous-pod-no-forge-secret: the pod of an instance with `forge_access: anonymous`
//! carries no forge credential under any entry name, and no session writes to the forge by a
//! spelling the guard can read.
//!
//! - The pod, against the fake API server of `tests/fake_kube/`: a second entry naming the
//!   secret of the daemon's forge token; the variables that point git, ssh or a forge's command
//!   line at a credential or a configuration file; the installer's git settings that carry no
//!   credential (a proxy, a CA file), which stay; and no leak between an anonymous instance and
//!   one with a `forge_token` of its own.
//! - The digest: the member is part of what replaces a pod.
//! - Validation: an anonymous mapping that names the secret of the daemon's forge token at
//!   another key, or a `secret_syncs` entry.
//! - The guard: spellings of a remote write the anonymous rule denies, and the one it cannot
//!   read (a program word the shell expands), which the guide names.

mod active;
mod fake_kube;
mod scratch;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use conductor_cli::config;
use conductor_cli::daemon::pods::{self, Pods};
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Manager;
use conductor_cli::guard::{self, Places, SessionList};
use conductor_model::dispatch::Verdict;
use conductor_model::instance::ReconcileReason;
use conductor_model::secret::SecretName;
use fake_kube::FakeKube;
use scratch::Scratch;
use serde_json::{Value, json};

/// The words every refusal of a write names.
const NAMED: &str = "forge_access: anonymous";

const NAMESPACE: &str = "conductor-instances";
const MODEL_TOKEN: &str = "model-token-value-0000";
const FORGE_TOKEN: &str = "forge-token-value-1111";
const BETA_TOKEN: &str = "beta-token-value-3333";
const CONTROL: &str = "control-token-value-2222";
const SETTING: &str = "example-setting-value-4444";

/// A git configuration that gives git a credential helper for GitHub.
const HELPER_CONFIG: &str = "[credential \"https://github.com\"]\n\thelper = !example-helper\n";

/// One daemon case: its directory and its fake API server.
struct Pod {
    dir: PathBuf,
    fake: FakeKube,
    _scratch: Scratch,
}

impl Pod {
    fn new(name: &str) -> Self {
        active::isolate();
        let scratch = Scratch::new(Path::new("faa-leaks").join(name));
        let dir = scratch.path().to_owned();
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        for (secret, value) in [
            ("alpha-model", MODEL_TOKEN),
            ("forge-token", FORGE_TOKEN),
            ("beta-forge", BETA_TOKEN),
            ("gitconfig", HELPER_CONFIG),
            ("setting", SETTING),
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

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

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

    /// One reconcile pass of the daemon over the file.
    fn pass(&self) {
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
        daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("a pass");
    }

    fn object(&self, resource: &str, name: &str) -> Value {
        self.fake
            .object(NAMESPACE, resource, name)
            .unwrap_or_else(|| panic!("no {resource} {name}"))
    }
}

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

/// The anonymous instance `alpha`.
fn anonymous_alpha() -> String {
    instance(
        "alpha",
        "{github: example-owner, include: [one]}",
        "\x20   forge_access: anonymous\n",
    )
}

fn secret_value(secret: &Value, key: &str) -> Option<String> {
    if let Some(text) = secret["stringData"][key].as_str() {
        return Some(text.to_owned());
    }
    let encoded = secret["data"][key].as_str()?;
    Some(String::from_utf8(pods::base64_decode(encoded).expect("base64")).expect("UTF-8"))
}

/// Every `(key, value)` the Secret holds.
fn held(secret: &Value) -> Vec<(String, String)> {
    ["data", "stringData"]
        .iter()
        .filter_map(|field| secret[*field].as_object())
        .flat_map(|map| map.keys().cloned())
        .filter_map(|key| secret_value(secret, &key).map(|value| (key, value)))
        .collect()
}

/// The container's env entries.
fn env_of(pod: &Value) -> Vec<Value> {
    pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("the container's env")
        .clone()
}

/// The names of the container's env entries.
fn names_of(pod: &Value) -> Vec<String> {
    env_of(pod)
        .iter()
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect()
}

/// The keys the pod's secret volume mounts.
fn mounted(pod: &Value) -> Vec<String> {
    pod["spec"]["volumes"]
        .as_array()
        .expect("the pod's volumes")
        .iter()
        .filter(|volume| volume["name"] == "secret")
        .flat_map(|volume| {
            volume["secret"]["items"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|item| item["key"].as_str().map(str::to_owned))
        .collect()
}

/// An installer that gives the daemon's GitHub token to a second variable as well (the guide's
/// "its Secret holds their secrets only where another entry names them") would make the anonymous
/// instance's pod carry the token through that entry, into the Secret, the container's env and
/// the mounted secret volume. It carries none of it. Beside it, an instance with a `forge_token`
/// of its own leaks nothing into the anonymous pod.
#[test]
fn a_second_entry_of_the_forge_tokens_secret_stays_out_of_the_anonymous_pod() {
    let case = Pod::new("alias");
    case.config(
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: GITHUB_PAT, secret: secret:forge-token}",
        ],
        &[
            anonymous_alpha(),
            instance(
                "beta",
                "{github: example-owner}",
                "\x20   forge_token: secret:beta-forge\n",
            ),
        ]
        .concat(),
    );
    case.pass();
    let secret = case.object("secrets", "conductor-instance-alpha");
    let held = held(&secret);
    // No leak between instances: beta's own token is not in alpha's Secret.
    assert!(
        !held.iter().any(|(_, value)| value == BETA_TOKEN),
        "alpha's Secret holds beta's forge token: {:?}",
        held.iter().map(|(key, _)| key).collect::<Vec<_>>()
    );
    // The daemon's forge token, by any key.
    let carrying: Vec<&String> = held
        .iter()
        .filter(|(_, value)| value == FORGE_TOKEN)
        .map(|(key, _)| key)
        .collect();
    let pod = case.object("pods", "conductor-instance-alpha");
    let referencing: Vec<String> = env_of(&pod)
        .iter()
        .filter(|entry| entry["valueFrom"]["secretKeyRef"]["key"] == "forge-token")
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect();
    let volume = mounted(&pod);
    assert!(
        carrying.is_empty() && referencing.is_empty() && !volume.iter().any(|k| k == "forge-token"),
        "the anonymous pod carries the daemon's forge token: its Secret holds it under \
         {carrying:?}, its container reads it as {referencing:?}, its volume mounts {volume:?}"
    );
}

/// git's configuration reaches the anonymous pod through `GIT_CONFIG_GLOBAL` or
/// `GIT_CONFIG_SYSTEM` unless the pod leaves them out: here the installer points git at a
/// configuration file the pod's secret volume mounts, which names a credential helper.
#[test]
fn git_config_global_and_system_stay_out_of_the_anonymous_pod() {
    let case = Pod::new("global");
    case.config(
        &[
            "{name: GIT_CONFIG_GLOBAL, value: /run/secrets/conductor/gitconfig}",
            "{name: GIT_CONFIG_SYSTEM, value: /run/secrets/conductor/gitconfig}",
            "{name: EXAMPLE_GITCONFIG, secret: secret:gitconfig}",
        ],
        &anonymous_alpha(),
    );
    case.pass();
    let pod = case.object("pods", "conductor-instance-alpha");
    let gits: Vec<String> = names_of(&pod)
        .into_iter()
        .filter(|name| name.starts_with("GIT_CONFIG_"))
        .collect();
    let secret = case.object("secrets", "conductor-instance-alpha");
    let helper = held(&secret)
        .into_iter()
        .any(|(_, value)| value.contains("helper"));
    assert!(
        gits.is_empty(),
        "the anonymous pod carries git's configuration {gits:?}; its Secret holds a helper: \
         {helper}"
    );
}

/// The variables that point git, ssh or a forge's command line at a credential or at a
/// configuration file are none of the anonymous pod's, while an instance without the member keeps
/// them; a secret-backed entry under a name the daemon cannot read as either stays in both.
#[test]
fn no_variable_that_points_git_ssh_or_a_forge_cli_at_a_credential_reaches_the_anonymous_pod() {
    let denied = [
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_SYSTEM",
        "GIT_SSH_COMMAND",
        "GIT_SSH",
        "SSH_AUTH_SOCK",
        "SSH_ASKPASS",
        "GH_CONFIG_DIR",
        "GLAB_CONFIG_DIR",
        "GIT_ASKPASS",
        "XDG_CONFIG_HOME",
    ];
    let entries: Vec<String> = denied
        .iter()
        .map(|name| {
            format!(
                "{{name: {name}, value: /etc/example/{}}}",
                name.to_lowercase()
            )
        })
        .chain([
            "{name: GH_TOKEN, secret: secret:forge-token}".to_owned(),
            "{name: EXAMPLE_SETTING, secret: secret:setting}".to_owned(),
            "{name: GIT_CONFIG_NOSYSTEM, value: \"1\"}".to_owned(),
        ])
        .collect();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let case = Pod::new("pointers");
    case.config(
        &entries,
        &[
            anonymous_alpha(),
            instance("beta", "{github: example-owner}", ""),
        ]
        .concat(),
    );
    case.pass();
    let alpha = names_of(&case.object("pods", "conductor-instance-alpha"));
    let carried: Vec<&&str> = denied
        .iter()
        .filter(|name| alpha.iter().any(|carried| carried == *name))
        .collect();
    assert!(
        carried.is_empty(),
        "the anonymous pod carries {carried:?}: {alpha:?}"
    );
    for kept in ["EXAMPLE_SETTING", "GIT_CONFIG_NOSYSTEM"] {
        assert!(
            alpha.iter().any(|name| name == kept),
            "alpha keeps {kept}: {alpha:?}"
        );
    }
    let secret = case.object("secrets", "conductor-instance-alpha");
    assert_eq!(secret_value(&secret, "setting").as_deref(), Some(SETTING));
    let beta = names_of(&case.object("pods", "conductor-instance-beta"));
    for name in denied.iter().chain(&["GH_TOKEN", "EXAMPLE_SETTING"]) {
        assert!(
            beta.iter().any(|carried| carried == name),
            "beta keeps {name}: {beta:?}"
        );
    }
}

/// The installer's git settings that carry no credential stay in the anonymous pod: a proxy and
/// a CA setting, at any level of `http.` and of `http.<url>.`, renumbered under a count that
/// matches; every other pair (a helper, a rewrite, an extra header, an ssh command, an include)
/// is left out.
#[test]
fn the_installers_proxy_and_ca_settings_stay_in_the_anonymous_pod() {
    let pairs = [
        ("credential.helper", "store"),
        ("http.proxy", "http://proxy.example.com:3128"),
        (
            "url.https://mirror.example.com/.insteadOf",
            "https://github.com/",
        ),
        ("http.sslCAInfo", "/etc/example/ca.pem"),
        (
            "http.https://github.com/.extraHeader",
            "Authorization: basic x",
        ),
        ("core.sshCommand", "ssh -i /etc/example/key"),
        (
            "http.https://github.com/.proxy",
            "http://proxy.example.com:3129",
        ),
        ("include.path", "/etc/example/gitconfig"),
        ("https.proxy", "http://proxy.example.com:3130"),
        ("http.sslCAPath", "/etc/example/certs"),
        ("core.askPass", "/etc/example/askpass"),
    ];
    let mut entries = vec![format!(
        "{{name: GIT_CONFIG_COUNT, value: \"{}\"}}",
        pairs.len()
    )];
    for (index, (key, value)) in pairs.iter().enumerate() {
        entries.push(format!("{{name: GIT_CONFIG_KEY_{index}, value: {key:?}}}"));
        entries.push(format!(
            "{{name: GIT_CONFIG_VALUE_{index}, value: {value:?}}}"
        ));
    }
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let case = Pod::new("proxy");
    case.config(&entries, &anonymous_alpha());
    case.pass();
    let pod = case.object("pods", "conductor-instance-alpha");
    let gits: Vec<(String, String)> = env_of(&pod)
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
    let kept = [
        ("http.proxy", "http://proxy.example.com:3128"),
        ("http.sslCAInfo", "/etc/example/ca.pem"),
        (
            "http.https://github.com/.proxy",
            "http://proxy.example.com:3129",
        ),
        ("https.proxy", "http://proxy.example.com:3130"),
        ("http.sslCAPath", "/etc/example/certs"),
    ];
    let mut wanted = vec![("GIT_CONFIG_COUNT".to_owned(), kept.len().to_string())];
    for (index, (key, value)) in kept.iter().enumerate() {
        wanted.push((format!("GIT_CONFIG_KEY_{index}"), (*key).to_owned()));
        wanted.push((format!("GIT_CONFIG_VALUE_{index}"), (*value).to_owned()));
    }
    assert_eq!(gits, wanted, "{pod}");
}

/// The member is part of the pod's digest, so setting or removing it through `PUT
/// /v1/instances/{name}` replaces the pod: no pod keeps a token once the member is set, and none
/// stays tokenless once it is removed.
#[test]
fn the_member_changes_the_pods_digest() {
    active::isolate();
    let home = Path::new("/srv/example");
    let file = |extra: &str| {
        format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
             \x20 secrets: {{source: files, dir: /var/lib/example/secrets}}\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: registry.example.com/conductor:1.0.0\n\
             \x20   owner: example-owner\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G}}\n\
             \x20   env:\n\
             \x20     - {{name: GH_TOKEN, secret: secret:forge-token}}\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: none\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-owner, include: [one]}}]\n\
             {extra}"
        )
    };
    let made = |extra: &str| {
        let parsed =
            config::parse(&file(extra), home).unwrap_or_else(|problems| panic!("{problems:?}"));
        let kubernetes = parsed.daemon.kubernetes.clone().expect("daemon.kubernetes");
        pods::manifests(&kubernetes, &parsed, &parsed.instances[0])
    };
    let with = made("\x20   forge_access: anonymous\n");
    let without = made("");
    assert_ne!(with.digest, without.digest);
    assert!(with.rendered.contains("forge_access"), "{}", with.rendered);
    assert!(!without.rendered.contains("forge_access"));
    assert!(!with.secrets.contains("forge-token"), "{:?}", with.secrets);
    assert!(
        without.secrets.contains("forge-token"),
        "{:?}",
        without.secrets
    );
}

/// The text of a file with the anonymous instance `alpha`, `extra` lines appended to it.
fn anonymous_file(extra: &str) -> String {
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
         \x20 secrets: {{source: files, dir: /var/lib/example/secrets}}\n\
         \x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example.com/conductor:1.0.0\n\
         \x20   owner: example-owner\n\
         \x20   api_port: 7310\n\
         \x20   volume: {{size: 20G}}\n\
         \x20   env:\n\
         \x20     - {{name: GH_TOKEN, secret: secret:forge-token}}\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-owner, include: [one]}}]\n\
         \x20   forge_access: anonymous\n\
         {extra}"
    )
}

/// An anonymous mapping that names the secret of the daemon's forge token at another key is
/// refused at that key: the rendered file cannot carry it into the pod's Secret.
#[test]
fn an_anonymous_mapping_cannot_name_the_daemons_forge_secret() {
    active::isolate();
    let text = anonymous_file(
        "\x20   git_identity: {name: \"secret:forge-token\", email: bot@example.com}\n",
    );
    let problems = config::parse(&text, Path::new("/srv/example"))
        .err()
        .unwrap_or_else(|| panic!("the file validates:\n{text}"));
    assert!(
        problems
            .iter()
            .any(|problem| problem.path == "instances[0].git_identity.name"),
        "{problems:?}"
    );
}

/// What a refusal of another instance's forge token beside the member says.
const OTHER_FORGE_TOKEN: &str = "forge_token";

/// An anonymous mapping that names another instance's `forge_token` secret at another key, or
/// `globals` that name it beside an anonymous instance, are refused at that key; a config that
/// passed by validation gets no pod: the start refuses it before any secret is resolved, nothing
/// created. The same key naming a secret of no forge token validates.
#[test]
fn another_instances_forge_token_is_refused_beside_the_member_and_starts_no_pod() {
    let case = Pod::new("other-forge-token");
    let beta = "\x20 - name: beta\n\
                \x20   placement: none\n\
                \x20   session_prefix: beta\n\
                \x20   sources: [{github: example-owner}]\n\
                \x20   forge_token: secret:beta-forge\n";
    let refused_at = |text: &str, path: &str| {
        let problems = config::parse(text, Path::new("/srv/example"))
            .err()
            .unwrap_or_else(|| panic!("the file validates:\n{text}"));
        assert!(
            problems.iter().any(|problem| problem.path == path
                && problem.message.contains(OTHER_FORGE_TOKEN)
                && problem.message.contains(NAMED)),
            "{path}: {problems:?}"
        );
    };
    refused_at(
        &anonymous_file(&format!("\x20   model_token: secret:beta-forge\n{beta}")),
        "instances[0].model_token",
    );
    refused_at(
        &anonymous_file(&format!(
            "\x20   git_identity: {{name: \"secret:beta-forge\", email: bot@example.com}}\n{beta}"
        )),
        "instances[0].git_identity.name",
    );
    refused_at(
        &anonymous_file(beta).replace(
            "instances:",
            "globals: {model_token: secret:beta-forge}\ninstances:",
        ),
        "globals.model_token",
    );
    let valid = config::parse(
        &anonymous_file(&format!("\x20   model_token: secret:alpha-model\n{beta}")),
        Path::new("/srv/example"),
    )
    .unwrap_or_else(|problems| panic!("{problems:?}"));

    let mut passed = valid.clone();
    passed.instances[0].model_token = Some(SecretName("beta-forge".to_owned()));
    let made = pods::manifests(
        passed
            .daemon
            .kubernetes
            .as_ref()
            .expect("daemon.kubernetes"),
        &passed,
        &passed.instances[0],
    );
    assert!(
        !made.secrets.contains("beta-forge"),
        "the anonymous pod's Secret holds beta's forge token: {:?}",
        made.secrets
    );
    let kubernetes = passed.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let asked = std::cell::RefCell::new(Vec::new());
    let error = Pods::new(case.fake.client())
        .start(
            &kubernetes,
            &passed,
            &passed.instances[0],
            "an-instance-token",
            &|name: &SecretName| {
                asked.borrow_mut().push(name.0.clone());
                None
            },
        )
        .expect_err("the start is refused");
    let error = format!("{error:#}");
    assert!(
        error.contains("beta-forge")
            && error.contains(NAMED)
            && error.contains("nothing is created"),
        "{error}"
    );
    assert!(asked.borrow().is_empty(), "resolved {:?}", asked.borrow());
    assert!(
        case.fake.requests().is_empty(),
        "{:?}",
        case.fake.requests()
    );
}

/// A `secret_syncs` entry beside `forge_access: anonymous` is refused at `secret_syncs`: a sync
/// writes the forge's secrets, which the instance never does; an empty list is no sync.
#[test]
fn secret_syncs_beside_the_member_is_refused_at_its_path() {
    active::isolate();
    let home = Path::new("/srv/example");
    let text = anonymous_file(
        "\x20   secret_syncs:\n\
         \x20     - name: ci-policy\n\
         \x20       source: {repository: one, path: ci/policy.json}\n\
         \x20       secrets:\n\
         \x20         - {owner: example-owner, name: CI_POLICY}\n\
         \x20       command: example-sync publish ci/policy.json\n",
    );
    let problems = config::parse(&text, home)
        .err()
        .unwrap_or_else(|| panic!("the file validates:\n{text}"));
    assert!(
        problems
            .iter()
            .any(|problem| problem.path == "instances[0].secret_syncs"
                && problem.message.contains(NAMED)),
        "{problems:?}"
    );
    let empty = anonymous_file("\x20   secret_syncs: []\n");
    config::parse(&empty, home).unwrap_or_else(|problems| panic!("{problems:?}"));
}

/// The guard's places of the anonymous instance `alpha` in `dir`, under the `gates` route.
fn places(dir: &Path) -> Places {
    let text = "version: conductor.config/2\n\
                daemon:\n\
                \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
                \x20 secrets: {source: files, dir: /var/lib/example/secrets}\n\
                \x20 kubernetes:\n\
                \x20   namespace: conductor-instances\n\
                \x20   image: registry.example.com/conductor:1.0.0\n\
                \x20   owner: example-owner\n\
                \x20   api_port: 7310\n\
                \x20   volume: {size: 20G}\n\
                instances:\n\
                \x20 - name: alpha\n\
                \x20   placement: none\n\
                \x20   session_prefix: alpha\n\
                \x20   sources: [{github: example-owner, include: [one]}]\n\
                \x20   forge_access: anonymous\n\
                \x20   forge: {route: gates}\n";
    let file = dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    let parsed = config::parse(text, &dir.join("home")).unwrap_or_else(|p| panic!("{p:?}"));
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

fn decide(places: &Places, cwd: &Path, command: &str) -> guard::Decision {
    let payload = json!({
        "session_id": "00000000-0000-4000-8000-0000000000b1",
        "cwd": text_of(cwd),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
    });
    guard::decide_in(&payload, places, &SessionList::default())
}

/// The case's directory with a checkout `root/one` and the records, each with a GitHub
/// `origin`; the guard's places over it.
fn guard_case(name: &str) -> (Scratch, PathBuf, Places) {
    active::isolate();
    let scratch = Scratch::new(Path::new("faa-leaks").join(name));
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
    let places = places(&dir);
    (scratch, dir, places)
}

/// Each `(cwd, command, verdict, reason)` of `commands` whose decision is not a denial naming
/// the member, in the checkout and in the records.
fn not_denied(places: &Places, dir: &Path, commands: &[&str]) -> Vec<String> {
    let mut missed = Vec::new();
    for cwd in [dir.join("root/one"), dir.join("records")] {
        for command in commands {
            let decision = decide(places, &cwd, command);
            if !(decision.verdict == Verdict::Deny && decision.reason.contains(NAMED)) {
                missed.push(format!(
                    "{} | {command} | {:?} | {}",
                    cwd.file_name().and_then(|name| name.to_str()).unwrap_or(""),
                    decision.verdict,
                    decision.reason
                ));
            }
        }
    }
    missed
}

/// Spellings of a write to git's remote or the forge the anonymous rule denies, naming the
/// member: after a launcher (`env`, `timeout`, `nohup`, `xargs`, `find -exec`), git's alias
/// given on the line, `git-push` by its own name, a git subcommand that pushes under another name
/// (`subtree push`, `lfs push`), and the bot route's API call with a write method.
#[test]
fn push_spellings_after_a_launcher_an_alias_or_another_name_are_denied() {
    let (_scratch, dir, places) = guard_case("guard-missed");
    let commands = [
        "env GIT_TRACE=1 git push origin main",
        "timeout 60 git push origin main",
        "nohup git push origin main",
        "echo main | xargs git push origin",
        "find . -maxdepth 0 -exec git push origin main \\;",
        "git -c alias.p=push p origin main",
        "git -c 'alias.p=!git push' p origin main",
        "/usr/lib/git-core/git-push origin main",
        "git-send-pack origin main",
        "git subtree push --prefix=docs origin main",
        "git lfs push origin main",
        "git-lfs push origin main",
        "env git lfs push origin main",
        "b10x-gates api -X POST repos/example-owner/one/issues -f title=x",
        "b10x-gates api --method POST --path repos/example-owner/one/issues --input body.json \
         --output out.json",
        "b10x-gates api --method=DELETE --path repos/example-owner/one/git/refs/heads/x \
         --output out.json",
        "timeout 60 b10x-gates bot push origin main",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(
        missed.is_empty(),
        "{} of {} decisions are no denial naming {NAMED}:\n{}",
        missed.len(),
        commands.len() * 2,
        missed.join("\n")
    );
}

/// Spellings of a forge write the anonymous rule denies, naming the member.
#[test]
fn push_spellings_the_anonymous_rule_holds() {
    let (_scratch, dir, places) = guard_case("guard-held");
    let commands = [
        "sh -c 'git push origin main'",
        "(git push origin main)",
        "echo $(git push origin main)",
        "command git push origin main",
        "exec git push origin main",
        "GIT_TRACE=1 git push origin main",
        "/usr/bin/git push origin main",
        "git --git-dir=.git --work-tree . -C . push origin main",
        "git --exec-path=/usr/lib/git-core push origin main",
        "git send-pack origin main",
        "b10x-gates bot push origin main",
        "gh api -X POST repos/example-owner/one/issues -f title=x",
        "gh api repos/example-owner/one/issues -f title=x",
        "env gh pr create -R example-owner/one --fill",
        "glab api -X POST projects/1/issues",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// The reads beside those spellings are no write: the rule names none of them.
#[test]
fn git_reads_beside_the_push_spellings_are_not_refused_as_writes() {
    let (_scratch, dir, places) = guard_case("guard-reads");
    for command in [
        "git fetch origin",
        "timeout 60 git fetch origin",
        "git -c alias.l=log l --oneline",
        "git subtree split --prefix=docs",
        "git lfs pull origin",
        "find . -maxdepth 0 -exec git status \\;",
        "echo push | xargs git log --grep",
        "b10x-gates api --method GET --path repos/example-owner/one --output out.json",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        assert!(
            !decision.reason.contains(NAMED),
            "{command}: {:?} {}",
            decision.verdict,
            decision.reason
        );
    }
}

/// The guide's section "Anonymous forge access", from its heading to the next.
fn guide_section() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../website/docs/guides/run-in-kubernetes.md");
    let text = fs::read_to_string(&path).expect("the Kubernetes guide");
    let start = text
        .find("## Anonymous forge access")
        .expect("the guide has the section");
    let rest = &text[start + 2..];
    let end = rest.find("\n## ").map_or(rest.len(), |end| end + 2);
    text[start..start + end].to_owned()
}

/// A push whose program word the shell expands (`$g push`) is not read from the line, so the
/// guard does not deny it: the guide says which spellings are refused and that for the rest the
/// pod holds no credential to write with.
#[test]
fn the_guide_says_a_program_the_shell_expands_is_not_read_and_the_pod_holds_no_credential() {
    let (_scratch, dir, places) = guard_case("guard-expanded");
    let decision = decide(&places, &dir.join("root/one"), "g=git; $g push origin main");
    assert!(
        !decision.reason.contains(NAMED),
        "the guard reads an expanded program word now; say so in the guide: {}",
        decision.reason
    );
    let section = guide_section();
    let one_line = section.split_whitespace().collect::<Vec<_>>().join(" ");
    for words in [
        "a program word the shell expands",
        "holds no credential",
        "`git subtree push`",
        "`git lfs push`",
        "`b10x-gates api`",
    ] {
        assert!(
            one_line.contains(words),
            "the section says {words:?}:\n{section}"
        );
    }
    assert!(
        !one_line.contains("Every write to the forge is refused before any call"),
        "the section still says every write is refused:\n{section}"
    );
}

fn text_of(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new(which("git"))
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_COUNT")
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
