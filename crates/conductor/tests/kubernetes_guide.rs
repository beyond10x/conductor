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
//! - the sidebar lists the guide beside the container guide.
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
const SUPERVISOR_RIGHTS: [(&str, &str); 10] = [
    ("create", "pods"),
    ("get", "pods"),
    ("list", "pods"),
    ("delete", "pods"),
    ("get", "pods/log"),
    ("create", "secrets"),
    ("get", "secrets"),
    ("list", "secrets"),
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
