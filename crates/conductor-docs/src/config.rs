//! The config reference page: every key of a config file with its type, default and rule.
//!
//! The keys, their types, the rules and the type descriptions come from the specification,
//! `spec/domains/config.yaml`: its types are walked from `conductor.config.Config`, its invariants
//! become rules, and the comment above each type is its description. The defaults come from the
//! library the `conductor` binary runs: [`SAMPLE`], a file that sets every key, is read with one key
//! left out at a time through `conductor_cli::config::parse`, and the value
//! `conductor_cli::config::document` shows for it (what `conductor config show` prints) is its
//! default. A key whose absence the library refuses is required.

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
/// The name of the instance of [`SAMPLE`], written `<name>` where a default holds it.
const NAME: &str = "sample";
/// The home directory [`SAMPLE`] is read against, written `~` where a default holds it.
const HOME: &str = "/sample-home";

/// A config file that sets every key of the specification to a value that differs from its
/// default, so that leaving one out shows the default the library fills. Generation fails when a
/// key of the specification is missing here.
const SAMPLE: &str = "\
version: conductor.config/1
default: sample
instances:
  - name: sample
    sources:
      - github: example-org
      - local: ~/alpha
        exclude: [archive]
      - gitlab: example-group
        exclude: [legacy]
    checkouts:
      root: ~/example-org
      trees: ~/trees/example-org
    records: ~/example-org/records
    state: ~/example-org/state
    cache: ~/example-org/cache
    roles:
      - role: controller
        harness: codex
        model: sonnet
        agent: repo-controller
        settings: ~/example-org/controller-settings.json
        profile: ~/example-org/repo-controller.md
    controllers:
      max_working: 2
      max_subagents: 2
    repositories:
      - {match: alpha, activity: inactive}
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
    retention:
      snapshots: 3
    reports:
      - {channel: example-channel, as: example-bot, when: [daily]}
    authority:
      class_c: operator
      class_o: conductor
    operator: the operator
    catalog:
      repository: registry
      path: catalog/entries
      names: hex
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

/// `text` without the plan references it carries, such as ` (story:config-file)`.
fn without_plan_references(text: &str) -> Result<String> {
    let mut out = text.to_owned();
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
                Some("distinct") => format!(
                    "`{}` distinct across `{}`",
                    body["by"].as_str().context("distinct without by")?,
                    body["in"].as_str().context("distinct without in")?
                ),
                Some("exists") => format!(
                    "some `{}` in `{}` has `{}`",
                    body["as"].as_str().context("exists without as")?,
                    body["in"].as_str().context("exists without in")?,
                    body["that"].as_str().context("exists without that")?
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

/// Reads [`SAMPLE`] with the instance key at `path` left out (or the file key, when `top`).
fn probe(sample: &Value, path: &[Step], top: bool) -> Result<Found> {
    let mut file = sample.clone();
    let removed = if top {
        remove(&mut file, path)
    } else {
        remove(&mut file["instances"][0], path)
    };
    ensure!(removed > 0, "the sample file sets no {}", written(path));
    let text = serde_yaml::to_string(&file)?;
    let Ok(config) = conductor_cli::config::parse(&text, Path::new(HOME)) else {
        return Ok(Found::Required);
    };
    if top {
        return Ok(Found::Absent);
    }
    let instance = config
        .instances
        .first()
        .context("the sample file read back without its instance")?;
    let document = conductor_cli::config::document(instance);
    Ok(lookup(&document["instances"][0], path)
        .filter(|value| !value.is_null())
        .map_or(Found::Absent, |value| Found::Value(value.clone())))
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

/// The rows of the fields of struct `name`, keys prefixed with `prefix` (written) and `path`.
fn rows(
    spec: &Spec,
    sample: &Value,
    name: &str,
    prefix: &[Step],
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
            let found = probe(sample, &path, false)?;
            out.push(Row {
                key: written(&path),
                shown: spec.shown(expr),
                default: default_cell(&found, expr),
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
            Some("struct") => rows(spec, sample, base, &inner, out)?,
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
        let found = probe(&sample, &path, true)?;
        top.push(Row {
            key: key.to_owned(),
            shown: spec.shown(expr),
            default: default_cell(&found, expr),
            rule: rule_cell(&spec, root_type, key, expr)?,
        });
    }
    let mut instance = Vec::new();
    rows(&spec, &sample, INSTANCE, &[], &mut instance)?;

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
         which part of conductor reads each key.\n\n> {}\n\n## Top level\n\n{}\n\n{}\n## Instance\n\n{}\n\n\
         A key path written `a[].b` is the key `b` of each element of the list `a`. A default that \
         holds `<name>` holds the instance's name.\n\n{}\n## Types\n",
        prose(&spec.summary.split_whitespace().collect::<Vec<_>>().join(" ")),
        prose(&spec.types[ROOT].comment),
        table(&top),
        prose(&spec.types[INSTANCE].comment),
        table(&instance),
    ));
    let mut reachable = Vec::new();
    spec.reachable(ROOT, &mut reachable);
    let mut shown: Vec<&String> = spec
        .order
        .iter()
        .filter(|name| reachable.contains(name) && *name != ROOT && *name != INSTANCE)
        .collect();
    shown.extend(
        reachable
            .iter()
            .filter(|name| !spec.order.contains(name) && *name != ROOT && *name != INSTANCE),
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
