//! The guide `website/docs/guides/run-in-kubernetes.md` and the manifests it quotes,
//! `tests/fixtures/k8s/manifests.yaml`, which the end-to-end run on a cluster applies:
//!
//! - the guide quotes the file whole, as one block, so the two cannot drift;
//! - the ConfigMap's `daemon.yaml` is a daemon-config file `conductor daemon serve
//!   --daemon-config` accepts;
//! - two namespaces: the daemon's workload, its Secret and its service account in one; the
//!   instance pods' namespace, `daemon.kubernetes.namespace`, another, which holds the Role, the
//!   RoleBinding and the network policy and nothing else, since the daemon's rights there reach
//!   every Secret and service account of it;
//! - the Role grants exactly the rights the supervisor uses ([`SUPERVISOR_RIGHTS`]), each one
//!   `conductor doctor` asks for ([`conductor_cli::doctor::POD_RIGHTS`]), bound to the daemon's
//!   service account of the other namespace;
//! - both namespaces enforce the `restricted` pod security level and the daemon's pod meets it,
//!   the daemon not its first process;
//! - the network policy selects the instance pods by the daemon's label set
//!   ([`conductor_cli::daemon::pods::labels`]) and admits the daemon's pods, by label and
//!   namespace, on `daemon.kubernetes.api_port`;
//! - the sidebar lists the guide beside the container guide;
//! - the work call's subsection says what the call starts and how a review link comes back, and
//!   its request and answers decode as the runtime's own.
//!
//! No case runs `kubectl`, a cluster or the `conductor` binary.

mod active;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::daemon::pods;
use serde_yaml::Value;

const GUIDE: &str = "website/docs/guides/run-in-kubernetes.md";
const MANIFESTS: &str = "crates/conductor/tests/fixtures/k8s/manifests.yaml";

/// The image the manifests name, which a reader replaces.
const IMAGE: &str = "registry.example.com/conductor:<tag>";

/// The rights the pod supervisor uses in the instance namespace (`daemon/pods.rs`): no path reads,
/// lists or deletes a volume claim.
const SUPERVISOR_RIGHTS: [(&str, &str); 11] = [
    ("create", "pods"),
    ("get", "pods"),
    ("list", "pods"),
    ("delete", "pods"),
    ("get", "pods/log"),
    ("create", "secrets"),
    ("get", "secrets"),
    ("list", "secrets"),
    ("update", "secrets"),
    ("delete", "secrets"),
    ("create", "persistentvolumeclaims"),
];

/// The kinds that live in the instance namespace beside what the daemon makes there.
const IN_THE_INSTANCE_NAMESPACE: [&str; 3] = ["Role", "RoleBinding", "NetworkPolicy"];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every document of the manifests.
fn documents() -> Vec<Value> {
    let text = read(MANIFESTS);
    text.split("\n---\n")
        .map(|document| serde_yaml::from_str(document).expect("a YAML document"))
        .collect()
}

/// Every document of `kind`.
fn all(kind: &str) -> Vec<Value> {
    documents()
        .into_iter()
        .filter(|document| document["kind"].as_str() == Some(kind))
        .collect()
}

/// The one document of `kind`.
fn one(kind: &str) -> Value {
    let found = all(kind);
    assert_eq!(found.len(), 1, "one {kind} in {MANIFESTS}");
    found.into_iter().next().expect("one")
}

fn text<'v>(value: &'v Value, what: &str) -> &'v str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{what} is no string: {value:?}"))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_sequence()
        .map(|items| {
            items
                .iter()
                .map(|item| text(item, "an item").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn rights(of: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    of.iter()
        .map(|(verb, resource)| ((*verb).to_owned(), (*resource).to_owned()))
        .collect()
}

/// The ConfigMap's daemon-config file, parsed as `daemon serve --daemon-config` parses it.
fn daemon_config() -> conductor_model::config::DaemonConfig {
    active::isolate();
    let map = one("ConfigMap");
    let file = text(&map["data"]["daemon.yaml"], "data.daemon.yaml");
    conductor_cli::config::parse_daemon_config(file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the ConfigMap's daemon.yaml: {problems:?}"))
}

/// The daemon's namespace, its service account's, and the instance pods', which the ConfigMap's
/// `daemon.kubernetes.namespace` names.
fn namespaces() -> (String, String) {
    let account = one("ServiceAccount");
    let daemon = text(
        &account["metadata"]["namespace"],
        "the service account's namespace",
    );
    let kubernetes = daemon_config()
        .daemon
        .kubernetes
        .expect("daemon.kubernetes");
    (daemon.to_owned(), kubernetes.namespace.0)
}

#[test]
fn the_guide_quotes_the_manifests_whole() {
    let guide = read(GUIDE);
    let manifests = read(MANIFESTS);
    assert!(
        guide.contains(&format!("```yaml\n{manifests}```\n")),
        "{GUIDE} quotes {MANIFESTS} as one block, byte for byte"
    );
}

#[test]
fn the_configmap_holds_a_daemon_config_file_for_the_statefulset() {
    let config = daemon_config();
    let kubernetes = config.daemon.kubernetes.expect("daemon.kubernetes");
    assert_eq!(kubernetes.image, IMAGE);
    assert!(
        config.daemon.control_token.is_some(),
        "daemon.control_token"
    );
    let set = one("StatefulSet");
    let container = &set["spec"]["template"]["spec"]["containers"][0];
    assert_eq!(text(&container["image"], "image"), IMAGE);
    let command = strings(&container["command"]);
    assert_eq!(
        command,
        [
            "conductor",
            "daemon",
            "serve",
            "--daemon-config",
            "/config/daemon.yaml"
        ]
    );
    let port = config.daemon.port;
    assert_eq!(
        container["ports"][0]["containerPort"].as_u64(),
        u64::try_from(port).ok()
    );
}

#[test]
fn the_instance_namespace_is_not_the_daemons_and_holds_only_its_rights_and_policy() {
    let (daemon, instances) = namespaces();
    assert_ne!(
        daemon, instances,
        "the instance pods have a namespace of their own"
    );
    let names: Vec<String> = all("Namespace")
        .iter()
        .map(|namespace| text(&namespace["metadata"]["name"], "a name").to_owned())
        .collect();
    assert_eq!(names, [daemon.clone(), instances.clone()]);
    for document in documents() {
        let kind = text(&document["kind"], "kind");
        if kind == "Namespace" {
            continue;
        }
        let name = text(&document["metadata"]["name"], "metadata.name");
        let at = text(&document["metadata"]["namespace"], "metadata.namespace");
        let wanted = if IN_THE_INSTANCE_NAMESPACE.contains(&kind) {
            &instances
        } else {
            &daemon
        };
        assert_eq!(at, wanted, "{kind} {name}");
    }
}

#[test]
fn the_role_grants_the_rights_the_supervisor_uses_to_the_daemons_service_account() {
    let role = one("Role");
    let mut granted = BTreeSet::new();
    for rule in role["rules"].as_sequence().expect("rules") {
        assert_eq!(strings(&rule["apiGroups"]), [""], "the core API only");
        for resource in strings(&rule["resources"]) {
            for verb in strings(&rule["verbs"]) {
                assert!(
                    granted.insert((verb.clone(), resource.clone())),
                    "{verb} {resource} granted twice"
                );
            }
        }
    }
    assert_eq!(granted, rights(&SUPERVISOR_RIGHTS));
    let asked = rights(&conductor_cli::doctor::POD_RIGHTS);
    assert!(
        granted.is_subset(&asked),
        "doctor asks for each right the Role grants: {:?}",
        granted.difference(&asked).collect::<Vec<_>>()
    );

    let (daemon, _) = namespaces();
    let binding = one("RoleBinding");
    let account = one("ServiceAccount");
    assert_eq!(binding["roleRef"]["name"], role["metadata"]["name"]);
    assert_eq!(
        binding["subjects"][0]["kind"].as_str(),
        Some("ServiceAccount")
    );
    assert_eq!(binding["subjects"][0]["name"], account["metadata"]["name"]);
    assert_eq!(
        binding["subjects"][0]["namespace"].as_str(),
        Some(daemon.as_str())
    );
    let set = one("StatefulSet");
    assert_eq!(
        set["spec"]["template"]["spec"]["serviceAccountName"],
        account["metadata"]["name"]
    );
}

#[test]
fn both_namespaces_enforce_restricted_and_the_daemon_pod_meets_it() {
    for namespace in all("Namespace") {
        assert_eq!(
            namespace["metadata"]["labels"]["pod-security.kubernetes.io/enforce"].as_str(),
            Some("restricted"),
            "{namespace:?}"
        );
    }
    let set = one("StatefulSet");
    let spec = &set["spec"]["template"]["spec"];
    assert_eq!(
        spec["securityContext"]["runAsNonRoot"].as_bool(),
        Some(true)
    );
    assert_eq!(
        spec["securityContext"]["seccompProfile"]["type"].as_str(),
        Some("RuntimeDefault")
    );
    // The daemon is not the pod's first process, which would ignore `SIGTERM` until the grace
    // period kills it: the pause process is, as `docker run --init` gives a container one.
    assert_eq!(spec["shareProcessNamespace"].as_bool(), Some(true));
    for container in spec["containers"].as_sequence().expect("containers") {
        let context = &container["securityContext"];
        assert_eq!(context["allowPrivilegeEscalation"].as_bool(), Some(false));
        assert_eq!(strings(&context["capabilities"]["drop"]), ["ALL"]);
    }
}

#[test]
fn the_network_policy_admits_the_daemon_to_the_instance_pods_only() {
    let config = daemon_config();
    let kubernetes = config.daemon.kubernetes.expect("daemon.kubernetes");
    let (daemon, _) = namespaces();
    let policy = one("NetworkPolicy");
    let selected: serde_json::Value =
        serde_json::to_value(&policy["spec"]["podSelector"]["matchLabels"]).expect("JSON");
    let mut expected = pods::labels(&kubernetes.owner.0, "any");
    expected
        .as_object_mut()
        .expect("labels")
        .remove(pods::INSTANCE_LABEL);
    assert_eq!(selected, expected, "the label set but the instance's name");
    assert_eq!(strings(&policy["spec"]["policyTypes"]), ["Ingress"]);
    let ingress = policy["spec"]["ingress"].as_sequence().expect("ingress");
    assert_eq!(ingress.len(), 1);
    let from = ingress[0]["from"].as_sequence().expect("from");
    assert_eq!(
        from.len(),
        1,
        "one peer: the daemon's pods of its namespace"
    );
    let set = one("StatefulSet");
    assert_eq!(
        from[0]["podSelector"]["matchLabels"], set["spec"]["template"]["metadata"]["labels"],
        "the daemon's pods"
    );
    assert_eq!(
        from[0]["namespaceSelector"]["matchLabels"]["kubernetes.io/metadata.name"].as_str(),
        Some(daemon.as_str()),
        "of the daemon's namespace"
    );
    assert_eq!(
        ingress[0]["ports"][0]["port"].as_u64(),
        u64::try_from(kubernetes.api_port).ok()
    );
}

/// The text of the guide's section `title`, from its heading to the next one.
fn section(title: &str) -> String {
    let guide = read(GUIDE);
    let heading = format!("\n## {title}\n");
    let start = guide
        .find(&heading)
        .unwrap_or_else(|| panic!("{GUIDE} has a section {title:?}"));
    let rest = &guide[start + heading.len()..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())].to_owned()
}

/// The guide's section "Forge credentials" says both setups (story:instance-forge-token), each
/// with one example that the daemon reads as the section says: the `daemon.kubernetes.env` entry
/// of its YAML block, added to the ConfigMap's `daemon.yaml`, gives `GH_TOKEN` to an instance
/// that names no `forge_token`; the mapping of its `PUT` names one, and its pod reads `GH_TOKEN`
/// from that key of its own Secret instead.
#[test]
fn the_forge_credentials_section_holds_both_setups_as_the_daemon_reads_them() {
    active::isolate();
    let text = section("Forge credentials");
    for words in [
        "one token per daemon",
        "one token per instance",
        "forge_token",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
    let block = text
        .split_once("```yaml\n")
        .and_then(|(_, rest)| rest.split_once("```"))
        .map(|(block, _)| block)
        .expect("the section's YAML block");
    let fragment: Value = serde_yaml::from_str(block).expect("the YAML block");
    let env = fragment["daemon"]["kubernetes"]["env"].clone();
    assert!(
        env.is_sequence(),
        "the block's daemon.kubernetes.env: {block}"
    );
    let mapping = text
        .split_once("-d '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(body, _)| body)
        .expect("the section's request body");
    let body: serde_json::Value = serde_json::from_str(mapping).expect("the body's JSON");
    let instance = body["mapping"].clone();
    let forge = instance["forge_token"]
        .as_str()
        .and_then(|value| value.strip_prefix("secret:"))
        .expect("the mapping's forge_token")
        .to_owned();

    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    daemon["daemon"]["kubernetes"]["env"] = env;
    let mut without = instance.clone();
    without
        .as_object_mut()
        .expect("the mapping")
        .remove("forge_token");
    let mut file = daemon.clone();
    file["instances"] = serde_yaml::to_value([instance, without_name(without)]).expect("YAML");
    let file = serde_yaml::to_string(&file).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let token = |index: usize| -> Vec<serde_json::Value> {
        let made = pods::manifests(&kubernetes, &config, &config.instances[index]);
        made.pod["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
            .iter()
            .filter(|entry| entry["name"] == "GH_TOKEN")
            .map(|entry| entry["valueFrom"]["secretKeyRef"]["key"].clone())
            .collect()
    };
    assert_eq!(token(0), [serde_json::json!(forge)], "{file}");
    let daemon_wide = config.daemon.kubernetes.as_ref().and_then(|kubernetes| {
        kubernetes
            .env
            .iter()
            .find(|entry| entry.name == "GH_TOKEN")
            .and_then(|entry| entry.secret.clone())
    });
    let daemon_wide = daemon_wide.expect("the block's GH_TOKEN secret").0;
    assert_ne!(daemon_wide, forge, "{file}");
    assert_eq!(token(1), [serde_json::json!(daemon_wide)], "{file}");
}

/// The guide names an instance's `git_identity` (story:instance-git-identity) in the table of
/// the config's keys, with its rule, and in the list of what a pod carries: the identity's four
/// variables, the member over a `daemon.kubernetes.env` entry over the fixed default, and every
/// commit in the pod carrying it, a session's and the records'. The list no longer says the
/// fixed identity is the only one.
#[test]
fn the_guide_names_an_instances_git_identity_in_the_keys_and_in_what_a_pod_carries() {
    let table = section("The daemon-config file");
    let row = table
        .lines()
        .find(|line| line.starts_with("| `git_identity") || line.contains("`git_identity`"))
        .unwrap_or_else(|| panic!("the table names git_identity:\n{table}"));
    for words in ["name", "email", "200 bytes", "`<`", "`>`", "`@`", "422"] {
        assert!(row.contains(words), "the row says {words:?}: {row}");
    }

    let pod = section("What the daemon creates for each instance");
    let item = pod
        .split("\n- ")
        .find(|item| item.contains("GIT_AUTHOR_NAME"))
        .unwrap_or_else(|| panic!("the pod's list names the git identity:\n{pod}"));
    for words in [
        "`git_identity`",
        "GIT_AUTHOR_NAME",
        "GIT_AUTHOR_EMAIL",
        "GIT_COMMITTER_NAME",
        "GIT_COMMITTER_EMAIL",
        "`daemon.kubernetes.env`",
        "`conductor`",
        "`conductor@localhost`",
        "a session's commits",
        "the records",
    ] {
        assert!(item.contains(words), "the item says {words:?}:\n{item}");
    }
    assert!(
        !item.contains("the git identity of the records' commits,"),
        "the item still names one identity for the records only:\n{item}"
    );
}

/// The guide names an instance's `model_token_kind` in the table of the config's keys, with its
/// two kinds and its default, and has a section "The model token" that says which kind takes
/// which route to the harness (`apiKeyHelper` and `conductor token reveal --model-token` for an
/// API key; `CLAUDE_CODE_OAUTH_TOKEN` in the harness's environment, read once at each session's
/// start, for a subscription token), that nothing refreshes a subscription token, so an expired
/// or changed one needs a new value and `POST /v1/instances/{name}/restart`, what a running pod
/// does when the value changes, and what a control plane compares to see a pod on an older
/// value: `modified_at` of `GET /v1/instances/{name}/secrets` and the pod's start.
#[test]
fn the_guide_names_the_model_token_kind_its_two_routes_and_what_a_new_value_needs() {
    let table = section("The daemon-config file");
    let row = table
        .lines()
        .find(|line| line.contains("`model_token_kind`"))
        .unwrap_or_else(|| panic!("the table names model_token_kind:\n{table}"));
    for words in ["`api_key`", "`subscription_token`", "`model_token`", "422"] {
        assert!(row.contains(words), "the row says {words:?}: {row}");
    }

    let text = section("The model token");
    for words in [
        "`model_token_kind`",
        "`api_key`",
        "`subscription_token`",
        "`apiKeyHelper`",
        "`conductor token reveal --model-token <instance>`",
        "`CLAUDE_CODE_OAUTH_TOKEN`",
        "at each session's start",
        "nothing refreshes",
        "expired",
        "`POST /v1/instances/{name}/restart`",
        "`modified_at`",
        "`GET /v1/instances/{name}/secrets`",
        "a placement that opens a box",
        "/proc",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
}

/// The guide names an instance's `standing_rules` (story:instance-rules-text) in the table of the
/// config's keys, with its bound, the NUL and the refusal, and has a section "Standing rules" that
/// says which sessions read them and which do not, the frame they stand in, the copy a role with
/// its own profile starts from, that a change of them alone replaces no pod and how the new text
/// reaches the pod, the right that takes, that a session started after the change reads the new
/// text, and that a session that already runs keeps the text it started with. It names the
/// secret scan's two refusals of the text, that an empty text is no rule, that the instance's own
/// settings write does not change the rules, that a session on a host where the daemon starts
/// instances as units runs as the daemon's user, what a refused write of the Secret shows and
/// when the doctor needs the right.
#[test]
fn the_guide_names_the_standing_rules_who_reads_them_and_what_a_change_reaches() {
    let table = section("The daemon-config file");
    let row = table
        .lines()
        .find(|line| line.contains("`standing_rules`"))
        .unwrap_or_else(|| panic!("the table names standing_rules:\n{table}"));
    for words in [
        "16384 bytes",
        "NUL",
        "422",
        "[standing rules](#standing-rules)",
    ] {
        assert!(row.contains(words), "the row says {words:?}: {row}");
    }

    let text = section("Standing rules");
    for words in [
        "`standing_rules`",
        "every session",
        "controller",
        "director",
        "maintainer",
        "host roles",
        "backticks",
        "`profile`",
        "not changed",
        "replaces no pod",
        "in place",
        "`update`",
        "at each session's start",
        "keeps the text it started with",
        "`GET /v1/instances/{name}`",
        "looks like a token",
        "writes a password",
        "only white space",
        "settings write",
        "the daemon's user",
        "`detail`",
        "`conductor doctor`",
        "`POST /v1/instances`",
        "`PUT /v1/instances/{name}`",
        "`PUT /v1/config`",
        "`conductor config put`",
        "no secret reference",
        "not by itself",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
}

/// The guide names an instance's `forge_access` (story:forge-access-anonymous) in the table of the
/// config's keys, and has a section "Anonymous forge access" that says what the member does (no
/// credential in the pod, the clone with none, the line of a failed clone, the reads not made,
/// the writes refused and the `detail`), what such an instance cannot do, that whoever sends the
/// mapping answers for the repositories being public, each refusal and the API's answer.
#[test]
fn the_guide_names_anonymous_forge_access_what_it_does_and_cannot_do() {
    let table = section("The daemon-config file");
    let row = table
        .lines()
        .find(|line| line.starts_with("| `forge_access`"))
        .unwrap_or_else(|| panic!("the table names forge_access:\n{table}"));
    for words in [
        "`anonymous`",
        "422",
        "[anonymous forge access](#anonymous-forge-access)",
    ] {
        assert!(row.contains(words), "the row says {words:?}: {row}");
    }

    let text = section("Anonymous forge access");
    for words in [
        "`forge_access: anonymous`",
        "no credential",
        "`daemon.kubernetes.env`",
        "`GIT_CONFIG_COUNT`",
        "credential helper",
        "over HTTPS",
        "not public or not reachable",
        "git's last error line",
        "`include`",
        "pull request, issue, release or workflow run",
        "one line a pass",
        "refused before any call",
        "push",
        "comment",
        "`detail`",
        "writes nothing to its forge",
        "cannot do",
        "read a private repository",
        "Whoever sends the mapping answers for the repositories being public",
        "`instances[<i>].forge_token`",
        "`instances[<i>].sources[<j>]`",
        "`instances[<i>].forge_access`",
        "daemon that runs no pods",
        "`GET /v1/instances/{name}`",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
}

/// Each place of the guide that says how far the Role's rights reach says it of the writes too:
/// `create`, `delete` and `update` on Secrets can write or replace any Secret of that namespace,
/// an instance's API token and its rendered config included (story:instance-rules-text), beside
/// what `get`, `list` and `create` on pods reach.
#[test]
fn every_place_that_says_how_far_the_rights_reach_names_the_writes_on_secrets() {
    let guide = read(GUIDE);
    for start in [
        "The daemon needs no right in its own namespace.",
        "These rights reach further than the daemon's own objects.",
    ] {
        let at = guide
            .find(start)
            .unwrap_or_else(|| panic!("the guide says {start:?}"));
        let paragraph = guide[at..].split("\n\n").next().unwrap_or_default();
        for words in [
            "`create`, `delete` and `update` on Secrets",
            "write or replace any Secret",
            "API token",
            "rendered config",
        ] {
            assert!(
                paragraph.contains(words),
                "the paragraph says {words:?}:\n{paragraph}"
            );
        }
    }
}

/// The section "Forge credentials" says what a pod's git reaches with the token
/// (story:pod-git-reaches-forge): git over HTTPS to the forge's origin through a credential
/// helper, no other host, `GIT_CONFIG_COUNT` in `daemon.kubernetes.env` leaving git's
/// configuration to the installer, and SSH remotes not covered; and the pod its YAML block gives
/// an instance with a `github` source has the helper for `https://github.com`.
#[test]
fn the_forge_credentials_section_says_what_a_pods_git_reaches_with_the_token() {
    active::isolate();
    let text = section("Forge credentials");
    for words in [
        "credential helper",
        "https://github.com",
        "https://<host>",
        "no other host",
        "carries none of the daemon's forge token variables",
        "GIT_CONFIG_COUNT",
        "SSH",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
    assert!(
        !text.contains("does not read these variables"),
        "the section still says git reads no token:\n{text}"
    );
    let block = text
        .split_once("```yaml\n")
        .and_then(|(_, rest)| rest.split_once("```"))
        .map(|(block, _)| block)
        .expect("the section's YAML block");
    let fragment: Value = serde_yaml::from_str(block).expect("the YAML block");
    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    daemon["daemon"]["kubernetes"]["env"] = fragment["daemon"]["kubernetes"]["env"].clone();
    daemon["instances"] = serde_yaml::from_str(
        "[{name: alpha, placement: none, session_prefix: alpha, sources: [{github: example-org}]}]",
    )
    .expect("an instance");
    let file = serde_yaml::to_string(&daemon).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let made = pods::manifests(&kubernetes, &config, &config.instances[0]);
    let env = made.pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("env")
        .clone();
    let value = |name: &str| {
        env.iter()
            .find(|entry| entry["name"] == name)
            .and_then(|entry| entry["value"].as_str())
            .map(str::to_owned)
    };
    assert_eq!(value("GIT_CONFIG_COUNT").as_deref(), Some("1"), "{env:?}");
    assert_eq!(
        value("GIT_CONFIG_KEY_0").as_deref(),
        Some("credential.https://github.com.helper"),
        "{env:?}"
    );
}

/// The section "Forge credentials" says who gets the daemon's forge token (decisions F3-D1 to
/// F3-D5, F3-D7): it lists every variable of a forge's command line the daemon treats as its
/// forge token once, says that the daemon's GitLab token belongs to the host of `GITLAB_HOST`,
/// that `host` and `host:443` are different hosts, that an instance on another GitLab host names
/// a `forge_token` of its own, and that no instance names the secret of a daemon's forge token.
/// The self-hosted setup of its second YAML block, added to the ConfigMap's `daemon.yaml`, gives
/// an instance on that host the token and git's helper for it, and an instance on `gitlab.com`
/// neither; and the restart table names the pods that change once.
#[test]
fn the_forge_credentials_section_gives_the_daemons_gitlab_token_to_its_host_only() {
    active::isolate();
    let text = section("Forge credentials");
    for words in [
        "`GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`",
        "`GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN`, `OAUTH_TOKEN`",
        "GITLAB_HOST",
        "`host:443`",
        "a `forge_token` of its own",
        "cannot name the secret of a daemon's forge token",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
    let block = text
        .split("```yaml\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("```").map(|(block, _)| block))
        .find(|block| block.contains("GITLAB_HOST"))
        .expect("the section's YAML block of the self-hosted setup");
    let fragment: Value = serde_yaml::from_str(block).expect("the YAML block");
    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    daemon["daemon"]["kubernetes"]["env"] = fragment["daemon"]["kubernetes"]["env"].clone();
    let host = config_host(&fragment);
    daemon["instances"] = serde_yaml::from_str(&format!(
        "[{{name: alpha, placement: none, session_prefix: alpha, \
         sources: [{{gitlab: example-org, host: {host}}}]}}, \
         {{name: beta, placement: none, session_prefix: beta, sources: [{{gitlab: example-org}}]}}]"
    ))
    .expect("two instances");
    let file = serde_yaml::to_string(&daemon).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let env = |index: usize| -> Vec<serde_json::Value> {
        let made = pods::manifests(&kubernetes, &config, &config.instances[index]);
        made.pod["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
            .clone()
    };
    let named = |env: &[serde_json::Value], name: &str| {
        env.iter()
            .find(|entry| entry["name"] == name)
            .cloned()
            .unwrap_or_default()
    };
    let alpha = env(0);
    assert!(
        named(&alpha, "GITLAB_TOKEN")["valueFrom"].is_object(),
        "{alpha:?}"
    );
    assert_eq!(
        named(&alpha, "GIT_CONFIG_KEY_0")["value"],
        serde_json::json!(format!("credential.https://{host}.helper")),
        "{alpha:?}"
    );
    let beta = env(1);
    assert!(named(&beta, "GITLAB_TOKEN").is_null(), "{beta:?}");
    assert!(named(&beta, "GIT_CONFIG_COUNT").is_null(), "{beta:?}");

    let restarts = section("What survives a restart of the daemon");
    for words in ["GITLAB_HOST", "GITHUB_TOKEN"] {
        assert!(
            restarts.contains(words),
            "the restart table says {words:?}:\n{restarts}"
        );
    }
}

/// The section "Forge credentials" says which forge's daemon token a pod gets (decisions F4-D1 to
/// F4-D3): the daemon's entries of a forge only where the instance has a source on it, both where
/// its sources are all `local`, `CI_JOB_TOKEN` among `glab`'s variables, and a GitLab host written
/// in `GL_HOST`, `GITLAB_URI` or `GITLAB_URL` refused; it no longer says an instance keeps what
/// `env` gives every pod. The setup of its first YAML block, added to the ConfigMap's
/// `daemon.yaml`, gives an instance on GitLab no `GH_TOKEN` and one with `local` sources only the
/// `GH_TOKEN`; the restart table names the pods that lose a token of another forge.
#[test]
fn the_forge_credentials_section_gives_each_forges_daemon_token_to_its_instances_only() {
    active::isolate();
    let text = section("Forge credentials");
    for words in [
        "`GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN`, `OAUTH_TOKEN`, `CI_JOB_TOKEN`",
        "only where it has a source on that forge",
        "sources are all `local`",
        "`GL_HOST`, `GITLAB_URI` or `GITLAB_URL`",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
    for stale in ["on its forge and host", "every instance pod;"] {
        assert!(
            !text.contains(stale),
            "the section still says {stale:?}:\n{text}"
        );
    }
    let block = text
        .split_once("```yaml\n")
        .and_then(|(_, rest)| rest.split_once("```"))
        .map(|(block, _)| block)
        .expect("the section's YAML block");
    let fragment: Value = serde_yaml::from_str(block).expect("the YAML block");
    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    daemon["daemon"]["kubernetes"]["env"] = fragment["daemon"]["kubernetes"]["env"].clone();
    daemon["instances"] = serde_yaml::from_str(
        "[{name: alpha, placement: none, session_prefix: alpha, sources: [{gitlab: example-org}]}, \
         {name: beta, placement: none, session_prefix: beta, \
         sources: [{local: /var/lib/conductor/data/checkouts}]}]",
    )
    .expect("two instances");
    let file = serde_yaml::to_string(&daemon).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    let kubernetes = config.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let carries = |index: usize| {
        let made = pods::manifests(&kubernetes, &config, &config.instances[index]);
        made.pod["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env")
            .iter()
            .any(|entry| entry["name"] == "GH_TOKEN")
    };
    assert!(
        !carries(0),
        "an instance on GitLab carries GH_TOKEN: {file}"
    );
    assert!(carries(1), "an instance with local sources only: {file}");

    let restarts = section("What survives a restart of the daemon");
    for words in ["no source on", "CI_JOB_TOKEN", "GL_HOST"] {
        assert!(
            restarts.contains(words),
            "the restart table says {words:?}:\n{restarts}"
        );
    }
}

/// The value of the `GITLAB_HOST` entry of a guide's YAML block.
fn config_host(fragment: &Value) -> String {
    fragment["daemon"]["kubernetes"]["env"]
        .as_sequence()
        .into_iter()
        .flatten()
        .find(|entry| entry["name"].as_str() == Some("GITLAB_HOST"))
        .and_then(|entry| entry["value"].as_str())
        .expect("the block's GITLAB_HOST value")
        .to_owned()
}

/// A YAML string as text.
fn text_of(value: &Value) -> &str {
    text(value, "a YAML string")
}

/// The mapping `mapping` (JSON) renamed `beta`, its session prefix `beta`: a second instance.
fn without_name(mut mapping: serde_json::Value) -> serde_json::Value {
    mapping["name"] = serde_json::json!("beta");
    mapping["session_prefix"] = serde_json::json!("beta");
    mapping
}

#[test]
fn the_sidebar_lists_the_guide_beside_the_container_guide() {
    let sidebar = read("website/sidebars.ts");
    assert!(
        sidebar.contains("'guides/run-in-a-container',\n        'guides/run-in-kubernetes',"),
        "website/sidebars.ts"
    );
}

/// The section "The calls" says where a checkout must be and what a pod clones
/// (story:pod-source-checkouts): only what `include` names, at the collector's path with the
/// HTTPS `origin` its table gives; how a failed clone is read, through the instance's health;
/// which variables a self-hosted GitLab needs or the instance's `forge_token`; and which rights of
/// a token conductor's calls use, with GitLab's names for them as read, not tested. The mapping
/// of its example validates in the ConfigMap's daemon file, and its `gitlab` source is the one
/// forge the collectors read, with the host and the `include` the table's checkouts come from.
#[test]
fn the_calls_section_says_where_a_checkout_must_be_and_what_a_pod_clones() {
    active::isolate();
    let text = section("The calls");
    for words in [
        "include",
        "and nothing\nelse",
        "/var/lib/conductor/data/checkouts",
        "member_failures",
        "$D/v1/instances/beta/health",
        "`detail` does not change",
        "`GITLAB_TOKEN` as a `secret`",
        "`GITLAB_HOST` as a `value`",
        "forge_token",
        "only read",
        "`read_api`",
        "`api`",
        "`write_repository`",
        "not tested against\nevery forge version",
        "names no checkout",
        "trusted for the harness",
        "also one an earlier `include` had the pod clone, is not trusted",
        "start trusts every checkout under it, so an operator's own checkout stays usable",
        "whose content may run in the pod with the pod's tokens",
        "the only limit of what a pod clones",
        "at most 20 minutes for clones",
        "takes its first collect pass at its start",
        "every `cadence.cycle` (2 hours by default)",
        "and at every start of the pod",
        "`POST /v1/instances/{name}/restart` tries at once",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
    let body = text
        .split("-d '")
        .skip(1)
        .filter_map(|rest| rest.split_once('\'').map(|(body, _)| body))
        .find(|body| body.contains("\"include\""))
        .expect("the section's request body with an include");
    let body: serde_json::Value = serde_json::from_str(body).expect("the body's JSON");
    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    daemon["instances"] = serde_yaml::to_value([body["mapping"].clone()]).expect("YAML");
    let file = serde_yaml::to_string(&daemon).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    let forges = conductor_cli::collect::gitlab::Forge::all(&config.instances[0]);
    assert_eq!(forges.len(), 1, "{forges:?}");
    let forge = &forges[0];
    assert_eq!(forge.host, "gitlab.example.com");
    let include = forge.include.clone().expect("the source's include");
    assert_eq!(include, ["alpha", "sub/gamma"]);
    for path in &include {
        let wanted = conductor_cli::collect::checkouts::Wanted::gitlab(
            &forge.host,
            &forge.group,
            path,
            path,
            None,
            false,
        );
        let row = format!("| `<checkouts>/{}` | `{}` |", wanted.path, wanted.url);
        assert!(
            text.contains(&row),
            "the section's table holds {row:?}:\n{text}"
        );
    }
}

/// The text of the guide's subsection `title`, from its `###` heading to the next heading.
fn subsection(title: &str) -> String {
    let guide = read(GUIDE);
    let heading = format!("\n### {title}\n");
    let start = guide
        .find(&heading)
        .unwrap_or_else(|| panic!("{GUIDE} has a subsection {title:?}"));
    let rest = &guide[start + heading.len()..];
    let end = [rest.find("\n## "), rest.find("\n### ")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// The answer line under the `console` block's `curl` line that ends with `url`.
fn answer_to(text: &str, url: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim_end().ends_with(url))
        .unwrap_or_else(|| panic!("a curl line ending {url:?}:\n{text}"));
    lines
        .get(at + 1)
        .filter(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("an answer under {url:?}:\n{text}"))
        .to_string()
}

/// `answer` decoded as the work call's row (`conductor.dispatch.WorkState`), and written again
/// as the runtime writes it, byte for byte the line the guide quotes.
fn work_row(answer: &str) -> conductor_model::dispatch::WorkState {
    use conductor_model::server::{json, wire};
    let value = json::parse(answer).unwrap_or_else(|error| panic!("{answer}: {error:?}"));
    let row = wire::decode_conductor_dispatch_work_state(&value, "")
        .unwrap_or_else(|error| panic!("the guide's row is no WorkState: {error:?}\n{answer}"));
    let mut written = String::new();
    wire::encode_conductor_dispatch_work_state(&row, &mut written);
    assert_eq!(written, answer, "the runtime writes the row so");
    row
}

/// The subsection "The work call" (story:control-plane-work-handler): what a `201` starts and a
/// `200` does not, what the session reads, the default charter, the row's `start` with its three
/// values and the cycle that tries again at the instance's `cadence.cycle`, `max_working` and its
/// one-process lock, the rules of the brief, `start_ref` and `issue`, the stop after
/// [`conductor_cli::dispatch::FAILED_STARTS_MOST`] failed starts, the `review <url>` report and
/// the row it leaves, which is not finished. Its request decodes as a `WorkRequest` the call
/// takes, and its two answers decode as rows the runtime writes so.
#[test]
fn the_work_call_section_says_what_a_201_starts_and_how_the_review_link_comes_back() {
    use conductor_cli::dashboard::work::{BRIEF_MOST, MAX_BODY};
    use conductor_cli::dispatch::{
        BRIEF_BACKTICKS_MOST, FAILED_STARTS_MOST, refused_brief, refused_member, valid_link,
        valid_reference,
    };
    use conductor_model::dispatch::DispatchState;
    use conductor_model::server::{json, wire};

    active::isolate();
    let text = subsection("The work call");
    let calls = section("The calls");
    assert!(
        calls.contains("the five writes below"),
        "the calls name the work call among the writes:\n{calls}"
    );
    for words in [
        "`POST /v1/instances/{name}/work`",
        "`GET /v1/instances/{name}/work/{reference}`",
        "`201`",
        "starts nothing",
        "`Running` or `Paused` controller",
        "does not wait for the session",
        "`## Dispatch`",
        "the brief, verbatim, in a fenced block",
        "`<records>/charters/<repository>.md`",
        "missing or empty",
        "never overwritten",
        "`merge: human`",
        "never merge",
        "Any edit makes it the operator's charter",
        "only for a dispatch",
        "`controllers.max_working`",
        "`conductor spawn` run by hand",
        "not counted",
        "every `cadence.cycle` (2 hours by default)",
        "left to the operator",
        "a NUL",
        "C0 or C1",
        "U+2028",
        "U+2029",
        "bidirectional control",
        "backtick",
        "written in the controller's first prompt",
        "`[REPORT <repo> <dispatch-id>] review <url>`",
        "Only a `review` report",
        "is not finished",
        "`done`",
    ] {
        assert!(
            text.contains(words),
            "the subsection says {words:?}:\n{text}"
        );
    }
    assert!(
        text.contains(&format!("{BRIEF_MOST} bytes"))
            && text.contains(&format!("more than {BRIEF_BACKTICKS_MOST} backticks"))
            && MAX_BODY == 1024 * 1024
            && text.contains("1 MiB"),
        "the brief's and the body's bounds:\n{text}"
    );
    assert_eq!(FAILED_STARTS_MOST, 3);
    assert!(text.contains("three starts"), "{text}");

    for words in [
        "merge: human",
        "review <url>",
        "never merge",
        "done",
        "failed",
        "blocked",
    ] {
        assert!(
            conductor_cli::spawn::DEFAULT_CHARTER.contains(words),
            "the default charter says {words:?}"
        );
    }
    for name in [
        "ControllerLimit",
        "AlreadyRunning",
        "StartFailed",
        "LaunchFailed",
    ] {
        let row = text
            .lines()
            .find(|line| line.contains(&format!("`{name}`")))
            .unwrap_or_else(|| panic!("the subsection names {name}:\n{text}"));
        let value = json::parse(&format!("\"{name}\"")).expect("a JSON string");
        assert!(
            wire::decode_conductor_dispatch_work_start_refusal(&value, "").is_ok(),
            "{name} is a WorkStartRefusal: {row}"
        );
    }

    assert!(refused_brief("a\0b").is_some());
    assert!(refused_brief(&"`".repeat(BRIEF_BACKTICKS_MOST)).is_none());
    assert!(refused_brief(&"`".repeat(BRIEF_BACKTICKS_MOST + 1)).is_some());
    for refused in ['\u{1}', '\u{85}', '\u{2028}', '\u{2029}', '\u{202e}', '`'] {
        let word = format!("main{refused}x");
        assert!(refused_member(Some(&word), None).is_some(), "{refused:?}");
        assert!(refused_member(None, Some(&word)).is_some(), "{refused:?}");
    }

    let map = one("ConfigMap");
    let mut daemon: Value =
        serde_yaml::from_str(text_of(&map["data"]["daemon.yaml"])).expect("daemon.yaml");
    let add = calls
        .split("-d '")
        .skip(1)
        .filter_map(|rest| rest.split_once('\'').map(|(body, _)| body))
        .find(|body| body.contains("\"mapping\""))
        .expect("the request body of an add");
    let add: serde_json::Value = serde_json::from_str(add).expect("the body's JSON");
    daemon["instances"] = serde_yaml::to_value([add["mapping"].clone()]).expect("YAML");
    let file = serde_yaml::to_string(&daemon).expect("the file");
    let config = conductor_cli::config::parse(&file, Path::new("/var/lib/conductor"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{file}"));
    assert_eq!(
        conductor_cli::config::seconds(&config.instances[0].cadence.cycle),
        Some(2 * 3600),
        "the cycle's period the subsection names"
    );

    let body = text
        .split_once("-d '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(body, _)| body)
        .expect("the subsection's request body");
    let value = json::parse(body).expect("the body's JSON");
    let request = wire::decode_conductor_dispatch_work_request(&value, "")
        .unwrap_or_else(|error| panic!("the body is no WorkRequest: {error:?}"));
    assert!(valid_reference(&request.reference), "{body}");
    assert!(!request.brief.is_empty() && request.brief.len() <= BRIEF_MOST);
    assert!(refused_brief(&request.brief).is_none(), "{body}");
    assert!(
        refused_member(request.start_ref.as_deref(), request.issue.as_deref()).is_none(),
        "{body}"
    );

    let sent = work_row(&answer_to(&text, "$D/v1/instances/alpha/work"));
    assert_eq!(sent.state, DispatchState::Sent);
    assert_eq!(sent.reference, request.reference);
    assert_eq!(sent.repository.0, request.repository.0);
    assert!(sent.links.is_empty() && sent.start.is_none() && sent.last_report.is_none());

    let read = work_row(&answer_to(
        &text,
        &format!("$D/v1/instances/alpha/work/{}", request.reference),
    ));
    assert_eq!(
        read.state,
        DispatchState::Started,
        "a review report on a Sent row starts it, and ends nothing"
    );
    assert!(read.started_at.is_some() && read.ended_at.is_none());
    assert_eq!(read.dispatch_id.0, sent.dispatch_id.0);
    let report = read.last_report.clone().expect("last_report");
    let link = report
        .strip_prefix(&format!(
            "[REPORT {} {}] review ",
            read.repository.0, read.dispatch_id.0
        ))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("last_report is a review report: {report}"));
    assert!(valid_link(link), "{link}");
    assert_eq!(read.links, [link.to_owned()]);
}

/// The guide's list of what a pod's environment holds names `GLAB_SEND_TELEMETRY`, which the
/// daemon writes into every instance pod where `daemon.kubernetes.env` does not name it.
#[test]
fn the_pod_section_says_its_glab_sends_no_usage_data() {
    active::isolate();
    let text = section("What the daemon creates for each instance");
    for words in [
        "`GLAB_SEND_TELEMETRY` `false`",
        "so no `glab` the pod runs sends usage",
    ] {
        assert!(text.contains(words), "the section says {words:?}:\n{text}");
    }
}
