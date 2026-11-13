//! The config reference page: every key of a config file with its type, default and rule.
//!
//! The keys, their types, the rules and the type descriptions come from the specification,
//! `spec/domains/config.yaml`: its types are walked from `conductor.config.Config`, its invariants
//! become rules, and the comment above each type is its description. The defaults come from the
//! library the `conductor` binary runs: [`SAMPLE`], a `conductor.config/2` file that sets every
//! key, is read with one key left out at a time through `conductor_cli::config::parse`, and the
//! value `conductor_cli::config::document` shows for it (what `conductor config show` prints; for
//! the `daemon` and `globals` sections, `daemon_document` and `globals_document`) is its default.
//! An instance key is left out of `globals` as well, so its default is the built-in one, not the
//! global level's. A key whose absence the library refuses is required.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail, ensure};
use serde_yaml::Value;

use crate::{SOURCE, cell, front_matter, prose};

/// Where the config reference lands, relative to the repository root.
pub const PAGE: &str = "website/docs/reference/config.md";
/// The specification of the config file, relative to the repository root.
const SPEC: &str = "spec/domains/config.yaml";
/// Every domain of the specification, for the types the config refers to in other domains.
const DOMAINS: &str = "spec/domains";
/// The type of the whole file.
const ROOT: &str = "conductor.config.Config";
/// The type of one instance.
const INSTANCE: &str = "conductor.config.Instance";
/// The sections of the file that hold keys of their own, each with its type and its title.
const SECTIONS: [(&str, &str, &str); 2] = [
    ("daemon", "conductor.config.Daemon", "Daemon"),
    ("globals", "conductor.config.Globals", "Globals"),
];
/// The name of the instance of [`SAMPLE`], written `<name>` where a default holds it.
const NAME: &str = "sample";
/// The home directory [`SAMPLE`] is read against, written `~` where a default holds it.
const HOME: &str = "/sample-home";

/// The keys `<list>[].<field>` the library derives rather than reads (`story:instance-session-names`):
/// a file may write only the derived value, so [`SAMPLE`] does not set them, their default is
/// written `derived`, and they are left out of the default of their list. An instance's roles and
/// the global roles alike.
const DERIVED: [(&str, &str); 1] = [("roles", "session_name")];

/// The sections whose every key the library derives (`conductor`, the conductor that serves the
/// instance; `remote`, the remote substrate daemon read from the instance's `placement`,
/// `story:placement-substrate-remote`): [`SAMPLE`] does not set them, and their keys' default is
/// written `derived`.
const DERIVED_SECTIONS: [&str; 2] = ["conductor", "remote"];

/// The instance keys `(key, dependent)` where the library refuses `dependent` without `key`
/// (`story:auto-resume`: `usage.rotate` only beside `usage.probe`; `model_token_kind` only beside
/// `model_token`). Whether `key` is optional is read with `dependent` left out too, else the
/// refusal would make `key` read `required`.
const DEPENDENT: [(&str, &str); 2] = [
    ("usage.probe", "usage.rotate"),
    ("model_token", "model_token_kind"),
];

/// The keys `(<list>[], [<a>, <b>])` of which each element of the list writes exactly one
/// (`conductor.config.KubernetesEnv`'s `value` and `secret`): leaving one of them out of every
/// element leaves an element with neither, so their default is written as the pair.
const ONE_OF: [(&str, [&str; 2]); 1] = [("daemon.kubernetes.env[]", ["value", "secret"])];

/// The section whose keys make an instance run as a pod (`conductor.config.Kubernetes`, decision
/// D8): [`SAMPLE`] sets it, and an instance key's default is read without it, so it is the default
/// of an instance of a daemon that runs no pod.
const PODS: [&str; 2] = ["daemon", "kubernetes"];

/// The instance keys only a daemon that runs its instances as pods reads (`forge_token`, and only
/// for an instance whose sources name one forge, story:instance-forge-token; `git_identity`,
/// story:instance-git-identity; `forge_access`, only for an instance whose sources are all
/// `github` with `include`, story:forge-access-anonymous): [`SAMPLE`], whose instance has sources
/// on both forges and whose instance keys are read without [`PODS`], does not set them, their
/// default is written `none`, and each key of one of them is written `required`, as the library
/// refuses the key without it.
const POD_ONLY: [&str; 3] = ["forge_token", "git_identity", "forge_access"];

/// A config file that sets every key of the specification but [`DERIVED`] and [`POD_ONLY`] to a
/// value that differs from its default, so that leaving one out shows the default the library
/// fills. Generation fails when a key of the specification is missing here.
const SAMPLE: &str = "\
version: conductor.config/2
daemon:
  port: 7400
  loopback: false
  tls:
    listen: 0.0.0.0:7443
    cert: /etc/conductor/tls.crt
    key: /etc/conductor/tls.key
    identity: https://identity.example.com
  cadence:
    reconcile: 30s
  store: postgres://conductor@db.example.com/conductor
  secrets:
    source: remote
    namespace: example
    url: https://secrets.example.com
    dir: /run/secrets/example
  dashboard: /conductor
  operators:
    - name: example-operator
      role: owner
      grants: [answer-decisions, start-instances]
  upgrade_timeout: 5m
  timeout: 10s
  substrate_slots: 4
  unattended: true
  address: 0.0.0.0
  kubernetes:
    namespace: example-instances
    image: registry.example.com/conductor:1.0.0
    owner: example-org
    api_port: 7310
    volume:
      size: 20G
      storage_class: example-class
    resources:
      requests:
        cpu: 500m
        memory: 1Gi
      limits:
        cpu: '2'
        memory: 4Gi
    env:
      - name: GH_TOKEN
        secret: secret:example-forge-token
      - name: GIT_AUTHOR_NAME
        value: conductor
  control_token: secret:example-control-token
globals:
  roles:
    - role: supervisor
      kind: claude-code
      harness: codex
      model: sonnet
      agent: conductor-supervisor
      settings: ~/example-org/supervisor-settings.json
      profile: ~/example-org/conductor-supervisor.md
      read_only_tools: [mcp__example__read]
      forge_token_env: [EXAMPLE_FORGE_TOKEN]
      grant:
        tools: [show, search]
        repositories: [alpha]
        paths: [src]
        max_output_bytes: 32768
        ttl: 8h
      limits:
        memory_max: 8G
        cpu_weight: 50
        tasks_max: 512
      writable: [checkout, ~/example-org/shared]
      network: open
    - role: lead
      harness: claude
      model: opus
      token_owner: example-operator
  cadence:
    cycle: 3h
    daily: '07:00'
    watch: 120s
    ci: 1200s
  thresholds:
    context_handover: 250k
    disk_path: /srv
    disk_low: 95G
    disk_clear: 125G
    disk_admit: 30G
    build_slot: 75G
    build_size: 14G
    gate_stop: 14G
    gate_resume: 19G
    watchdog_every: 7s
    silent_session: 50m
    cpu_reserve: 4
    build_slots: 3
    repo_cache: off
    repo_cache_size: 6G
    lean_debuginfo: false
    short_lane_slots: 3
    short_lane_max: 6m
    stalled_session: 2h
    build_queued: 25m
    tree_stale: 4d
  limits:
    memory_max: 64G
    cpu_weight: 300
    tasks_max: 8192
  tuning:
    keys:
      - {path: thresholds.context_handover, min: 100000, max: 400000}
default: sample
instances:
  - name: sample
    placement: example-box
    session_prefix: example
    sources:
      - github: example-org
        include: [example-repository]
      - local: ~/alpha
        exclude: [archive]
      - gitlab: example-group
        host: gitlab.example.com
        exclude: [legacy]
    checkouts:
      root: /var/lib/conductor/data/example-org
      trees: /var/lib/conductor/data/trees/example-org
    records: /var/lib/conductor/data/records
    state: /var/lib/conductor/data/state
    cache: /var/lib/conductor/data/cache
    roles:
      - role: controller
        kind: claude-code
        harness: codex
        model: sonnet
        agent: repo-controller
        settings: ~/example-org/controller-settings.json
        profile: ~/example-org/repo-controller.md
        read_only_tools: [mcp__example__read]
        forge_token_env: [EXAMPLE_FORGE_TOKEN]
        grant:
          tools: [show, search]
          repositories: [alpha]
          paths: [src]
          max_output_bytes: 32768
          ttl: 8h
        limits:
          memory_max: 8G
          cpu_weight: 50
          tasks_max: 512
        writable: [checkout, ~/example-org/shared]
        network: open
        token_owner: example-owner
    limits:
      memory_max: 32G
      cpu_weight: 200
      tasks_max: 4096
    controllers:
      max_working: 2
      max_subagents: 2
      max_open_reviews: 2
    repositories:
      - {match: alpha, activity: inactive, no_releases: published on merge}
    cadence:
      cycle: 1h
      daily: '09:00'
      watch: 60s
      ci: 600s
    thresholds:
      context_handover: 200k
      disk_path: /data
      disk_low: 105G
      disk_clear: 115G
      disk_admit: 25G
      build_slot: 70G
      build_size: 12G
      gate_stop: 16G
      gate_resume: 18G
      watchdog_every: 5s
      silent_session: 45m
      cpu_reserve: 3
      build_slots: 2
      repo_cache: off
      repo_cache_size: 4G
      lean_debuginfo: false
      short_lane_slots: 2
      short_lane_max: 5m
      stalled_session: 1h
      build_queued: 20m
      tree_stale: 3d
    retention:
      snapshots: 3
    reports:
      - {channel: example-channel, as: example-bot, when: [daily]}
    authority:
      class_c: operator
      class_o: conductor
    usage:
      probe: usage-probe --json
      watcher: usage-rotator.service
      rotate: usage-rotate --best
    operator: the operator
    catalog:
      repository: registry
      path: catalog/entries
      names: hex
    secret_syncs:
      - name: ci-policy
        source: {repository: policies, branch: release, path: ci/policy.json}
        secrets:
          - {owner: example-org, repository: ci, name: CI_POLICY}
        command: example-sync publish ci/policy.json
        dry_run: example-sync publish ci/policy.json --dry-run
    gates_policy: ~/example-org/gates/policy.json
    model_token: secret:example-org-model
    model_token_kind: subscription_token
    standing_rules: No private name in a commit.
    forge:
      route: direct
";

/// One type of the specification: its declaration and the comment above it.
struct Type {
    decl: Value,
    comment: String,
}

/// The types of every domain, by qualified name, and the config domain's own in its order.
struct Spec {
    types: BTreeMap<String, Type>,
    order: Vec<String>,
    summary: String,
}

/// The comment lines directly above the line `- name: <name>` of `text`, joined.
fn comment_above(text: &str, name: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = lines
        .iter()
        .position(|line| line.trim() == format!("- name: {name}"))
    else {
        return String::new();
    };
    let mut comment: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .take_while(|line| line.trim_start().starts_with('#'))
        .map(|line| line.trim_start().trim_start_matches('#').trim())
        .collect();
    comment.reverse();
    comment
        .split(|line| line.is_empty())
        .map(|paragraph| paragraph.join(" "))
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `text` without the plan references it carries, such as ` (story:config-file)`, and without
/// its open questions: each `UNMAPPED:` note, to the end of its paragraph, is for whoever settles
/// the specification, not for a reader of the reference.
fn without_plan_references(text: &str) -> Result<String> {
    let mut out = text
        .split("\n\n")
        .map(|paragraph| {
            paragraph
                .find("UNMAPPED:")
                .map_or(paragraph, |at| paragraph[..at].trim_end())
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    while let Some(at) = out.find("(story:") {
        let end = out[at..]
            .find(')')
            .map(|end| at + end + 1)
            .context("an unclosed story reference")?;
        let start = if out[..at].ends_with(' ') { at - 1 } else { at };
        out.replace_range(start..end, "");
    }
    while let Some(at) = out.find("(`story:") {
        let end = out[at..]
            .find("`)")
            .map(|end| at + end + 2)
            .context("an unclosed story reference")?;
        let start = if out[..at].ends_with(' ') { at - 1 } else { at };
        out.replace_range(start..end, "");
    }
    // A story named after another reference in one parenthesis: `(`a`, story:b)`.
    while let Some(at) = out.find(", story:") {
        let id = &out[at + ", story:".len()..];
        let length = id
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(id.len());
        let end = at + ", story:".len() + length;
        ensure!(
            out[end..].starts_with(')'),
            "a story reference the generator cannot take out: {out}"
        );
        out.replace_range(at..end, "");
    }
    ensure!(
        !out.contains("story:"),
        "a story reference the generator cannot take out: {out}"
    );
    Ok(out)
}

fn read_spec(root: &Path) -> Result<Spec> {
    let mut types = BTreeMap::new();
    let mut order = Vec::new();
    let mut summary = String::new();
    let mut files: Vec<_> = fs::read_dir(root.join(DOMAINS))
        .with_context(|| format!("reading {DOMAINS}"))?
        .collect::<Result<_, _>>()?;
    files.sort_by_key(fs::DirEntry::path);
    for file in files {
        let path = file.path();
        if path.extension().is_none_or(|ext| ext != "yaml") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let domain: Value =
            serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let own = path.ends_with(SPEC);
        if own {
            summary = domain["summary"]
                .as_str()
                .context("the config domain has no summary")?
                .trim()
                .to_owned();
        }
        for decl in domain["types"].as_sequence().into_iter().flatten() {
            let name = decl["name"]
                .as_str()
                .context("a type has no name")?
                .to_owned();
            let comment = without_plan_references(&comment_above(&text, &name))?;
            if own {
                order.push(name.clone());
            }
            types.insert(
                name,
                Type {
                    decl: decl.clone(),
                    comment,
                },
            );
        }
    }
    ensure!(types.contains_key(ROOT), "{SPEC} declares no {ROOT}");
    Ok(Spec {
        types,
        order,
        summary: without_plan_references(&summary)?,
    })
}

/// A type expression, `List<X>`, `Optional<X>` or a name.
enum Shape<'a> {
    List(&'a str),
    Optional(&'a str),
    Plain(&'a str),
}

fn shape(expr: &str) -> Shape<'_> {
    if let Some(inner) = expr.strip_prefix("List<").and_then(|r| r.strip_suffix('>')) {
        Shape::List(inner)
    } else if let Some(inner) = expr
        .strip_prefix("Optional<")
        .and_then(|r| r.strip_suffix('>'))
    {
        Shape::Optional(inner)
    } else {
        Shape::Plain(expr)
    }
}

fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

impl Spec {
    fn kind(&self, name: &str) -> Option<&str> {
        self.types.get(name).and_then(|t| t.decl["kind"].as_str())
    }

    fn variants(&self, name: &str) -> Vec<String> {
        self.types[name].decl["variants"]
            .as_sequence()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_lowercase)
            .collect()
    }

    /// How the page writes the type expression `expr`.
    fn shown(&self, expr: &str) -> String {
        match shape(expr) {
            Shape::List(inner) => format!("list of {}", self.shown(inner)),
            Shape::Optional(inner) => self.shown(inner),
            Shape::Plain(name) => match self.kind(name) {
                Some("enum") => self
                    .variants(name)
                    .iter()
                    .map(|v| format!("`{v}`"))
                    .collect::<Vec<_>>()
                    .join(" | "),
                Some(_) => format!("[{}](#{})", short(name), short(name).to_lowercase()),
                None => name.to_lowercase(),
            },
        }
    }

    /// Every type `name` refers to, itself included, depth first.
    fn reachable(&self, name: &str, found: &mut Vec<String>) {
        let base = match shape(name) {
            Shape::List(inner) | Shape::Optional(inner) | Shape::Plain(inner) => inner,
        };
        let Some(t) = self.types.get(base) else {
            return;
        };
        if found.iter().any(|f| f == base) {
            return;
        }
        found.push(base.to_owned());
        for field in t.decl["fields"].as_sequence().into_iter().flatten() {
            if let Some(ty) = field["type"].as_str() {
                self.reachable(ty, found);
            }
        }
        if let Some(variants) = t.decl["variants"].as_mapping() {
            for target in variants.values().filter_map(Value::as_str) {
                self.reachable(target, found);
            }
        }
        if let Some(of) = t.decl["of"].as_str() {
            self.reachable(of, found);
        }
    }
}

/// An invariant as the page writes it.
fn invariant(value: &Value) -> Result<String> {
    Ok(match value {
        Value::String(text) => format!("`{text}`"),
        Value::Mapping(map) if map.len() == 1 => {
            let (key, body) = map.iter().next().context("an empty invariant")?;
            match key.as_str() {
                // Without `by`, the items themselves are distinct.
                Some("distinct") => format!(
                    "`{}` distinct across `{}`",
                    body["by"]
                        .as_str()
                        .or_else(|| body["as"].as_str())
                        .context("distinct without by or as")?,
                    body["in"].as_str().context("distinct without in")?
                ),
                Some("exists") => format!(
                    "some `{}` in `{}` has `{}`",
                    body["as"].as_str().context("exists without as")?,
                    body["in"].as_str().context("exists without in")?,
                    body["that"].as_str().context("exists without that")?
                ),
                // The condition as the specification writes it, on one line.
                Some("forall") => format!(
                    "every `{}` in `{}` has `{}`",
                    body["as"].as_str().context("forall without as")?,
                    body["in"].as_str().context("forall without in")?,
                    match &body["that"] {
                        Value::String(text) => text.clone(),
                        Value::Null => bail!("forall without that"),
                        that => serde_json::to_string(that)?,
                    }
                ),
                Some("any") => body
                    .as_sequence()
                    .context("any without a list")?
                    .iter()
                    .map(invariant)
                    .collect::<Result<Vec<_>>>()?
                    .join(" or "),
                other => bail!("an invariant form the page cannot write: {other:?}"),
            }
        }
        other => bail!("an invariant form the page cannot write: {other:?}"),
    })
}

/// Whether the rendered invariant `rule` names the field `field`.
fn names(rule: &str, field: &str) -> bool {
    rule.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| word == field)
}

/// One step of a key's path inside an instance: a key, or a key whose value is a list whose
/// every element is stepped into.
#[derive(Clone)]
enum Step {
    Key(String),
    Each(String),
}

/// One row of a key table.
struct Row {
    key: String,
    shown: String,
    default: String,
    rule: String,
}

/// A row's default as the probe found it.
enum Found {
    Required,
    Value(Value),
    Absent,
}

/// `value` with every key at `path` removed; how many were.
fn remove(value: &mut Value, path: &[Step]) -> usize {
    match path {
        [] => 0,
        [Step::Key(key)] => usize::from(
            value
                .as_mapping_mut()
                .and_then(|map| map.remove(key.as_str()))
                .is_some(),
        ),
        [Step::Key(key), rest @ ..] => value.get_mut(key.as_str()).map_or(0, |v| remove(v, rest)),
        [Step::Each(key), rest @ ..] => value
            .get_mut(key.as_str())
            .and_then(Value::as_sequence_mut)
            .map_or(0, |items| {
                items.iter_mut().map(|item| remove(item, rest)).sum()
            }),
    }
}

/// The value at `path` in `value`, from the first list element that holds it.
fn lookup<'v>(value: &'v Value, path: &[Step]) -> Option<&'v Value> {
    match path {
        [] => Some(value),
        [Step::Key(key), rest @ ..] => value.get(key.as_str()).and_then(|v| lookup(v, rest)),
        [Step::Each(key), rest @ ..] => value
            .get(key.as_str())?
            .as_sequence()?
            .iter()
            .find_map(|item| lookup(item, rest)),
    }
}

/// Reads [`SAMPLE`] with the instance key at `path` left out (or the file key, when `top`: a key
/// of the file, or of one of its [`SECTIONS`]). An instance key is left out of `globals` too, so
/// its default is the built-in one and not the value the sample sets at the global level.
fn probe(sample: &Value, path: &[Step], top: bool) -> Result<Found> {
    let mut file = sample.clone();
    let removed = if top {
        remove(&mut file, path)
    } else {
        let removed = remove(&mut file["instances"][0], path);
        remove(&mut file["globals"], path);
        remove(&mut file, &PODS.map(|key| Step::Key(key.to_owned())));
        for (key, dependent) in DEPENDENT {
            if written(path) == key {
                let dependent: Vec<Step> = dependent
                    .split('.')
                    .map(|key| Step::Key(key.to_owned()))
                    .collect();
                remove(&mut file["instances"][0], &dependent);
            }
        }
        removed
    };
    ensure!(removed > 0, "the sample file sets no {}", written(path));
    let text = serde_yaml::to_string(&file)?;
    let Ok(config) = conductor_cli::config::parse(&text, Path::new(HOME)) else {
        return Ok(Found::Required);
    };
    let mut found = if top {
        let mut sections = serde_yaml::Mapping::new();
        sections.insert(
            Value::from("daemon"),
            conductor_cli::config::daemon_document(&config.daemon),
        );
        sections.insert(
            Value::from("globals"),
            conductor_cli::config::globals_document(&config.globals),
        );
        lookup(&Value::Mapping(sections), path)
            .filter(|value| !value.is_null())
            .cloned()
    } else {
        let instance = config
            .instances
            .first()
            .context("the sample file read back without its instance")?;
        let document = conductor_cli::config::document(instance);
        lookup(&document["instances"][0], path)
            .filter(|value| !value.is_null())
            .cloned()
    };
    if let (Some(Step::Key(key)), Some(Value::Sequence(items))) = (path.last(), found.as_mut()) {
        for (list, field) in DERIVED {
            if key == list {
                for item in items.iter_mut() {
                    if let Value::Mapping(map) = item {
                        map.remove(field);
                    }
                }
            }
        }
    }
    Ok(found.map_or(Found::Absent, Found::Value))
}

/// Whether `prefix` and `key` are a [`DERIVED`] key, in an instance or in a section.
fn derived(prefix: &[Step], key: &str) -> bool {
    DERIVED.iter().any(|(list, field)| {
        matches!(prefix.last(), Some(Step::Each(each)) if each == list) && key == *field
    }) || matches!(prefix, [Step::Key(section)] if DERIVED_SECTIONS.contains(&section.as_str()))
        || (prefix.is_empty() && DERIVED_SECTIONS.contains(&key))
}

fn written(path: &[Step]) -> String {
    path.iter()
        .map(|step| match step {
            Step::Key(key) => key.clone(),
            Step::Each(key) => format!("{key}[]"),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// `value` written in YAML flow style, with the sample's instance name as `<name>`.
fn flow(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(text) => {
            let text = text
                .split('/')
                .map(|segment| if segment == NAME { "<name>" } else { segment })
                .collect::<Vec<_>>()
                .join("/");
            match text.strip_prefix(HOME) {
                Some(rest) => format!("~{rest}"),
                None => text,
            }
        }
        Value::Sequence(items) => format!(
            "[{}]",
            items.iter().map(flow).collect::<Vec<_>>().join(", ")
        ),
        Value::Mapping(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", flow(k), flow(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim_end()
            .to_owned(),
    }
}

/// The default cell for `found`, a key of type `expr`.
fn default_cell(found: &Found, expr: &str) -> String {
    match found {
        Found::Required => "required".to_owned(),
        Found::Value(value) => format!("`{}`", flow(value)),
        Found::Absent if matches!(shape(expr), Shape::List(_)) => "`[]`".to_owned(),
        Found::Absent => "none".to_owned(),
    }
}

/// The rule an `alphabet` sets: its letters and digits in words, the rest as written.
fn alphabet_rule(alphabet: &str) -> String {
    let letters = ('A'..='Z').chain('a'..='z').all(|c| alphabet.contains(c));
    let digits = ('0'..='9').all(|c| alphabet.contains(c));
    let rest: String = alphabet
        .chars()
        .filter(|c| !(letters && c.is_ascii_alphabetic()) && !(digits && c.is_ascii_digit()))
        .collect();
    let mut parts = Vec::new();
    if letters {
        parts.push("letters".to_owned());
    }
    if digits {
        parts.push("digits".to_owned());
    }
    if !rest.is_empty() {
        parts.push(format!("`{rest}`"));
    }
    format!("{} only", parts.join(", "))
}

/// The rule cell for a field:the invariants of its struct that name it, and its type's alphabet.
fn rule_cell(spec: &Spec, owner: &Type, field: &str, expr: &str) -> Result<String> {
    let mut rules = Vec::new();
    for value in owner.decl["invariants"].as_sequence().into_iter().flatten() {
        let rule = invariant(value)?;
        if names(&rule, field) {
            rules.push(rule);
        }
    }
    let base = match shape(expr) {
        Shape::List(inner) | Shape::Optional(inner) | Shape::Plain(inner) => inner,
    };
    if let Some(alphabet) = spec
        .types
        .get(base)
        .and_then(|t| t.decl["alphabet"].as_str())
    {
        rules.push(alphabet_rule(alphabet));
    }
    Ok(rules.join("; "))
}

/// The rows of the fields of struct `name`, keys prefixed with `prefix` (written) and `path`;
/// under the file's root when `top`, else under the instance.
fn rows(
    spec: &Spec,
    sample: &Value,
    name: &str,
    prefix: &[Step],
    top: bool,
    out: &mut Vec<Row>,
) -> Result<()> {
    let owner = spec
        .types
        .get(name)
        .with_context(|| format!("{SPEC} declares no {name}"))?;
    for field in owner.decl["fields"].as_sequence().into_iter().flatten() {
        let key = field["name"].as_str().context("a field has no name")?;
        let expr = field["type"]
            .as_str()
            .with_context(|| format!("{name}.{key} has no type"))?;
        let mut path = prefix.to_vec();
        path.push(Step::Key(key.to_owned()));
        let (base, list) = match shape(expr) {
            Shape::List(inner) => (inner, true),
            Shape::Optional(inner) | Shape::Plain(inner) => (inner, false),
        };
        let kind = spec.kind(base);
        let required_struct = kind == Some("struct") && matches!(shape(expr), Shape::Plain(_));
        if !required_struct {
            let pair = ONE_OF
                .iter()
                .find(|(list, pair)| written(prefix) == *list && pair.contains(&key));
            let default = if derived(prefix, key) {
                "derived".to_owned()
            } else if !top && prefix.is_empty() && POD_ONLY.contains(&key) {
                "none".to_owned()
            } else if !top
                && matches!(prefix, [Step::Key(section)] if POD_ONLY.contains(&section.as_str()))
            {
                "required".to_owned()
            } else if let Some((_, [a, b])) = pair {
                format!("one of `{a}`, `{b}`")
            } else {
                default_cell(&probe(sample, &path, top)?, expr)
            };
            out.push(Row {
                key: written(&path),
                shown: spec.shown(expr),
                default,
                rule: rule_cell(spec, owner, key, expr)?,
            });
        }
        let mut inner = prefix.to_vec();
        inner.push(if list {
            Step::Each(key.to_owned())
        } else {
            Step::Key(key.to_owned())
        });
        match kind {
            Some("struct") => rows(spec, sample, base, &inner, top, out)?,
            Some("union") if list => union_rows(spec, sample, base, &inner, out)?,
            _ => {}
        }
    }
    Ok(())
}

/// The rows of a list of union `name`: one per variant key, whose value is the variant's first
/// field, and one per further field, in the variants that have it.
fn union_rows(
    spec: &Spec,
    sample: &Value,
    name: &str,
    prefix: &[Step],
    out: &mut Vec<Row>,
) -> Result<()> {
    let variants = spec.types[name].decl["variants"]
        .as_mapping()
        .with_context(|| format!("{name} has no variants"))?;
    let keys: Vec<String> = variants
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_lowercase)
        .collect();
    let mut further: Vec<(String, String, Vec<String>)> = Vec::new();
    for ((variant, target), key) in variants.iter().zip(&keys) {
        let target = target
            .as_str()
            .with_context(|| format!("{name}: variant {variant:?} names no type"))?;
        let fields: Vec<&Value> = spec.types[target].decl["fields"]
            .as_sequence()
            .into_iter()
            .flatten()
            .collect();
        let first = fields
            .first()
            .with_context(|| format!("{target} has no field"))?;
        let mut path = prefix.to_vec();
        path.push(Step::Key(key.clone()));
        out.push(Row {
            key: written(&path),
            shown: spec.shown(first["type"].as_str().unwrap_or("String")),
            default: format!(
                "one of {}",
                keys.iter()
                    .map(|k| format!("`{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            rule: String::new(),
        });
        for field in &fields[1..] {
            let field_name = field["name"].as_str().context("a field has no name")?;
            let expr = field["type"].as_str().context("a field has no type")?;
            match further.iter_mut().find(|(n, _, _)| n == field_name) {
                Some((_, _, owners)) => owners.push(key.clone()),
                None => further.push((field_name.to_owned(), expr.to_owned(), vec![key.clone()])),
            }
        }
    }
    for (field, expr, owners) in further {
        let mut path = prefix.to_vec();
        path.push(Step::Key(field));
        let found = probe(sample, &path, false)?;
        out.push(Row {
            key: written(&path),
            shown: spec.shown(&expr),
            default: default_cell(&found, &expr),
            rule: format!(
                "only beside {}",
                owners
                    .iter()
                    .map(|k| format!("`{k}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        });
    }
    Ok(())
}

fn table(rows: &[Row]) -> String {
    let mut out = String::from("| Key | Type | Default | Rule |\n|---|---|---|---|\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            row.key,
            cell(&row.shown),
            cell(&row.default),
            cell(&row.rule)
        ));
    }
    out
}

/// What kind of type `t` is, in one sentence.
fn kind_line(spec: &Spec, name: &str) -> String {
    let t = &spec.types[name];
    match t.decl["kind"].as_str() {
        Some("enum") => format!("One of {}.", spec.shown(name)),
        Some("newtype") => format!(
            "Written as {}.",
            spec.shown(t.decl["of"].as_str().unwrap_or("String"))
        ),
        Some("union") => "One of the forms the keys table lists.".to_owned(),
        Some("struct") => {
            let fields: Vec<String> = t.decl["fields"]
                .as_sequence()
                .into_iter()
                .flatten()
                .filter_map(|f| f["name"].as_str())
                .map(|f| format!("`{f}`"))
                .collect();
            format!("Keys: {}.", fields.join(", "))
        }
        _ => String::new(),
    }
}

/// The config reference page for the repository at `root`.
///
/// # Errors
///
/// When the specification cannot be read, it declares a key [`SAMPLE`] does not set, or an
/// invariant takes a form the page cannot write.
pub fn page(root: &Path) -> Result<String> {
    let spec = read_spec(root)?;
    let sample: Value = serde_yaml::from_str(SAMPLE)?;
    if let Err(problems) = conductor_cli::config::parse(SAMPLE, Path::new(HOME)) {
        bail!(
            "the sample config does not validate: {}",
            problems
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let mut top = Vec::new();
    let root_type = &spec.types[ROOT];
    for field in root_type.decl["fields"].as_sequence().into_iter().flatten() {
        let key = field["name"].as_str().context("a field has no name")?;
        let expr = field["type"].as_str().context("a field has no type")?;
        let path = [Step::Key(key.to_owned())];
        // A section's keys are its own table's; it is written in the top table as one.
        let found = if SECTIONS.iter().any(|(section, _, _)| *section == key) {
            Found::Absent
        } else {
            probe(&sample, &path, true)?
        };
        top.push(Row {
            key: key.to_owned(),
            shown: spec.shown(expr),
            default: match found {
                Found::Absent if SECTIONS.iter().any(|(section, _, _)| *section == key) => {
                    "each key its own".to_owned()
                }
                found => default_cell(&found, expr),
            },
            rule: rule_cell(&spec, root_type, key, expr)?,
        });
    }
    let mut sections = String::new();
    for (section, name, title) in SECTIONS {
        let mut found = Vec::new();
        rows(
            &spec,
            &sample,
            name,
            &[Step::Key(section.to_owned())],
            true,
            &mut found,
        )?;
        sections.push_str(&format!(
            "## {title}\n\n{}\n\n{}\n",
            prose(&spec.types[name].comment),
            table(&found)
        ));
    }
    let mut instance = Vec::new();
    rows(&spec, &sample, INSTANCE, &[], false, &mut instance)?;

    let mut out = front_matter(
        "Config reference",
        2,
        "Every key of the conductor config file with its type, default and rule, generated from the specification.",
    );
    out.push_str(&format!(
        "# Config reference\n\nGenerated by `conductor-docs`. The keys, their types, the rules and \
         the type descriptions are the specification's, [`{SPEC}`]({SOURCE}/{SPEC}); each default \
         is what `conductor config show` prints for a file that leaves the key out, read through \
         the same library the binary runs. A key marked required is one whose absence `conductor \
         config validate` refuses. The hand-written [config guide]({SOURCE}/docs/config.md) says \
         which part of conductor reads each key.\n\n> {}\n\n## Top level\n\n{}\n\n{}\n{}## Instance\n\n{}\n\n\
         A key path written `a[].b` is the key `b` of each element of the list `a`. A default that \
         holds `<name>` holds the instance's name.\n\n{}\n## Types\n",
        prose(&spec.summary.split_whitespace().collect::<Vec<_>>().join(" ")),
        prose(&spec.types[ROOT].comment),
        table(&top),
        sections,
        prose(&spec.types[INSTANCE].comment),
        table(&instance),
    ));
    let mut reachable = Vec::new();
    spec.reachable(ROOT, &mut reachable);
    let mut shown: Vec<&String> = spec
        .order
        .iter()
        .filter(|name| reachable.contains(name) && *name != ROOT && *name != INSTANCE)
        .filter(|name| !SECTIONS.iter().any(|(_, section, _)| section == name))
        .collect();
    shown.extend(
        reachable
            .iter()
            .filter(|name| !spec.order.contains(name) && *name != ROOT && *name != INSTANCE)
            .filter(|name| !SECTIONS.iter().any(|(_, section, _)| section == name)),
    );
    for name in shown {
        let t = &spec.types[name.as_str()];
        out.push_str(&format!("\n### {}\n\n", short(name)));
        if !t.comment.is_empty() {
            out.push_str(&format!("{}\n\n", prose(&t.comment)));
        }
        out.push_str(&format!(
            "{} Specified as `{name}`.\n",
            prose(&kind_line(&spec, name))
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_references_are_taken_out_and_others_refused() {
        assert_eq!(
            without_plan_references("read from here (story:config-file). Then").unwrap(),
            "read from here. Then"
        );
        assert_eq!(
            without_plan_references("the command (`story:x`) runs").unwrap(),
            "the command runs"
        );
        assert!(without_plan_references("see story:x").is_err());
    }

    /// A story named after another reference in one parenthesis is taken out with its comma.
    #[test]
    fn a_plan_reference_after_another_in_one_parenthesis_is_taken_out() {
        assert_eq!(
            without_plan_references(
                "`delivered` (`session deliver`, story:mail-and-deliver): `to`"
            )
            .unwrap(),
            "`delivered` (`session deliver`): `to`"
        );
    }

    /// An `UNMAPPED:` note is an open question for whoever settles the specification, not text
    /// for a reader of the reference: it is left out to the end of its paragraph, story
    /// references and all, and the paragraphs around it stay.
    #[test]
    fn open_questions_are_left_out() {
        assert_eq!(
            without_plan_references(
                "What a line says. UNMAPPED: story:x says one thing (`a`); ask.\n\nThe next."
            )
            .unwrap(),
            "What a line says.\n\nThe next."
        );
        assert_eq!(
            without_plan_references("UNMAPPED: see story:x.\n\nKept.").unwrap(),
            "Kept."
        );
    }

    #[test]
    fn invariants_are_written_in_words() {
        let value: Value = serde_yaml::from_str(
            "any:\n  - not defined(default)\n  - exists: {in: instances, as: instance, that: instance.name == default}\n",
        )
        .unwrap();
        assert_eq!(
            invariant(&value).unwrap(),
            "`not defined(default)` or some `instance` in `instances` has `instance.name == default`"
        );
        let value: Value = serde_yaml::from_str("all: [a]").unwrap();
        assert!(invariant(&value).is_err());
    }
}
