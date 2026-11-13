//! The Configuration page (`story:dashboard-config-page`): `GET /v1/settings` answers the
//! effective config of the instance whose dashboard answers (`conductor.config.ConfigPage`), and
//! `POST /v1/settings` checks or writes the page's staged changes (`conductor.config.WriteConfig`).
//!
//! The view reads the config file and an existing store, and writes nothing.
//! Every key `config show` writes for the instance is one `ConfigField`, its path under the
//! instance (`cadence.watch`), with the value the file's instance takes when it writes no such
//! key, and when a change of it takes effect ([`effect`]).
//!
//! The write is taken only as the Decisions page's answers are ([`super::answers::admitted`]):
//! the server's own host and origin on 127.0.0.1, the page's token, JSON of a bounded length.
//! Each staged change sets or removes one key of the instance's mapping in the file; the result
//! is converted as `config validate` converts it, and refused with each problem by its YAML path.
//! A valid result keeps the old file beside it as `<file>.<UTC time>.bak`, replaces it, writes
//! each activity change as its repository's mark by the operator, and records one class-C
//! decision by the operator, recorded by `dashboard`, with the diff as its evidence. The file is
//! written from its YAML values: a comment in it is kept only in the backup.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_model::config::{
    ActivityChange, ConfigChange, ConfigEffect, ConfigProblem, ConfigWriteRefusal, Instance,
};
use conductor_model::decision::DecisionClass;
use conductor_model::direction::{Activity, MarkedRepository};
use serde::Deserialize;
use serde_json::{Value, json};
use serde_yaml::Value as Yaml;
use time::OffsetDateTime;

use super::Sources;
use super::answers::{RECORDED_BY, Refused};
use crate::{config, decide, repository};

/// The page's data and its write.
pub const CONFIG_PATH: &str = "/v1/settings";

/// The keys under an instance the page shows and never changes: `config migrate` moves them, and
/// the control plane alone writes the instance's standing rules and its forge access
/// ([`CONTROL_PLANE`]).
const FIXED: [&str; 9] = [
    "name",
    "session_prefix",
    "dashboard_port",
    "records",
    "state",
    "cache",
    "checkouts",
    STANDING_RULES,
    FORGE_ACCESS,
];

/// The instance's standing rules (story:instance-rules-text): the control plane's text, which
/// `POST` and `PUT /v1/instances` write; the instance's own settings write is no second writer.
const STANDING_RULES: &str = "standing_rules";

/// The instance's forge access (story:forge-access-anonymous): the control plane's, which `POST`
/// and `PUT /v1/instances` write; the instance's own settings write is no second writer, so a
/// session cannot give its instance back a forge it was started without.
const FORGE_ACCESS: &str = "forge_access";

/// The keys under an instance the control plane alone writes.
const CONTROL_PLANE: [&str; 2] = [STANDING_RULES, FORGE_ACCESS];

/// When a change of the key at `path` (under the instance) takes effect.
#[must_use]
pub fn effect(path: &str) -> ConfigEffect {
    let head = path.split(['.', '[']).next().unwrap_or_default();
    match head {
        _ if FIXED.contains(&head) => ConfigEffect::Fixed,
        "thresholds" if path == "thresholds.context_handover" => ConfigEffect::Cycle,
        "thresholds" | "retention" => ConfigEffect::WatchTick,
        "controllers" | "authority" | "sources" | "repositories" | "reports" | "catalog" => {
            ConfigEffect::Cycle
        }
        _ => ConfigEffect::LoopRestart,
    }
}

/// The word of `effect` as the page reads it.
fn effect_word(effect: ConfigEffect) -> &'static str {
    match effect {
        ConfigEffect::WatchTick => "WatchTick",
        ConfigEffect::Cycle => "Cycle",
        ConfigEffect::LoopRestart => "LoopRestart",
        ConfigEffect::Fixed => "Fixed",
    }
}

/// `instance`'s keys as `config show` writes them, each `(path under the instance, value)`.
fn fields(instance: &Instance) -> Vec<(String, String)> {
    let mut out = String::new();
    config::lines(&config::document(instance), "", &mut out);
    out.lines()
        .filter_map(|line| line.split_once(": "))
        .filter_map(|(path, value)| {
            path.strip_prefix("instances[0].")
                .map(|path| (path.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The file's text, and its config converted, read for `home`.
fn read(file: &Path, home: &Path) -> Result<(String, Yaml), String> {
    let text =
        fs::read_to_string(file).map_err(|error| format!("read {}: {error}", file.display()))?;
    let value: Yaml = serde_yaml::from_str(&text)
        .map_err(|error| format!("{}: not YAML: {error}", file.display()))?;
    let _ = home;
    Ok((text, value))
}

/// The index of the instance named `name` in `config`, else 0.
fn index_of(config: &conductor_model::config::Config, name: Option<&str>) -> usize {
    name.and_then(|name| {
        config
            .instances
            .iter()
            .position(|instance| instance.name.0 == name)
    })
    .unwrap_or(0)
}

/// The keys the instance at `index` of the file `value` takes when it writes only what places
/// it: its name, sources, checkouts, records, state, cache, prefix and port.
fn defaults(value: &Yaml, index: usize, home: &Path) -> Vec<(String, String)> {
    let Some(Yaml::Mapping(written)) = value
        .get("instances")
        .and_then(|instances| instances.get(index))
    else {
        return Vec::new();
    };
    let mut kept = serde_yaml::Mapping::new();
    for key in [
        "name",
        "session_prefix",
        "dashboard_port",
        "sources",
        "checkouts",
        "records",
        "state",
        "cache",
    ] {
        if let Some(held) = written.get(key) {
            kept.insert(Yaml::from(key), held.clone());
        }
    }
    let mut file = serde_yaml::Mapping::new();
    file.insert(Yaml::from("version"), Yaml::from(config::VERSION));
    file.insert(
        Yaml::from("instances"),
        Yaml::Sequence(vec![Yaml::Mapping(kept)]),
    );
    let text = serde_yaml::to_string(&Yaml::Mapping(file)).unwrap_or_default();
    // Unheld: the built file is a version-1 one, and holding its keys for the instance's name
    // would show the real file's instance, read next, as a version-1 one.
    config::parse_unheld(&text, home)
        .ok()
        .and_then(|config| config.instances.into_iter().next())
        .map(|instance| fields(&instance))
        .unwrap_or_default()
}

/// The forge of `instance`'s first source.
fn forge(instance: &Instance) -> &'static str {
    use conductor_model::config::Source;
    match instance.sources.first() {
        Some(Source::GitHub(_)) => "github",
        Some(Source::GitLab(_)) => "gitlab",
        Some(Source::Local(_)) => "local",
        None => "",
    }
}

/// The answer of `GET /v1/settings`: the instance's effective settings, each with its default.
#[must_use]
pub fn answer(sources: &Sources, state: Option<&Path>) -> Value {
    let mut errors = Vec::new();
    let home = &sources.home;
    let file = sources.config_file.clone();
    let mut page = json!({
        "file": file.as_ref().map(|file| file.display().to_string()).unwrap_or_default(),
        "instance": sources.instance_name.clone().unwrap_or_default(),
        "index": 0,
        "fields": [],
        "instances": [],
        "repositories": [],
        "errors": [],
        "writable": false,
    });
    if let Some(file) = &file {
        match read(file, home) {
            Ok((text, value)) => match config::parse(&text, home) {
                Ok(parsed) => {
                    let index = index_of(&parsed, sources.instance_name.as_deref());
                    let defaults = defaults(&value, index, home);
                    if let Some(instance) = parsed.instances.get(index) {
                        let rows: Vec<Value> = fields(instance)
                            .into_iter()
                            .map(|(path, value)| {
                                let default = defaults
                                    .iter()
                                    .find(|(known, _)| *known == path)
                                    .map(|(_, default)| default.clone())
                                    .filter(|default| *default != value);
                                json!({
                                    "path": path,
                                    "value": value,
                                    "default": default,
                                    "takes_effect": effect_word(effect(&path)),
                                })
                            })
                            .collect();
                        page["instance"] = json!(instance.name.0);
                        page["index"] = json!(index);
                        page["fields"] = json!(rows);
                        page["writable"] = json!(true);
                    }
                }
                Err(problems) => errors.extend(
                    problems
                        .iter()
                        .map(|problem| format!("{}: {problem}", file.display())),
                ),
            },
            Err(error) => errors.push(error),
        }
    } else {
        errors.push(
            "this instance runs on the built-in defaults: there is no config file to show or \
             write"
                .to_owned(),
        );
    }
    page["instances"] = json!(instances());
    if let Some(state) = state.filter(|state| crate::store::present(state)) {
        match crate::store::open_reader(state) {
            Ok(store) => {
                let rows: Vec<Value> = repository::activity(&store)
                    .into_iter()
                    .map(|row| {
                        json!({
                            "repository": row.repository.0,
                            "activity": format!("{:?}", row.activity),
                            "decided_by": format!("{:?}", row.decided_by),
                            "marked_by": row.marked_by.map(|by| format!("{by:?}")),
                            "reason": row.reason,
                        })
                    })
                    .collect();
                page["repositories"] = json!(rows);
            }
            Err(error) => errors.push(format!("{error:#}")),
        }
    }
    page["errors"] = json!(errors);
    page
}

/// The instances of the config file this process read, each with its forge, the one this runtime
/// serves marked `current`; another is shown through the daemon's page for it
/// (`story:instance-api`). The host registry that listed every config file's is gone in v1
/// (design architecture-next § 11).
fn instances() -> Vec<Value> {
    let active = config::active();
    if !active.from_file {
        return Vec::new();
    }
    active
        .all()
        .into_iter()
        .map(|instance| {
            json!({
                "name": instance.name.0,
                "session_prefix": instance.session_prefix.as_ref().map(|prefix| prefix.0.clone()),
                "forge": forge(instance),
                "current": instance.name == active.instance.name,
            })
        })
        .collect()
}

/// The write as the page sends it (`conductor.config.WriteConfig`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    #[serde(default)]
    changes: Vec<Change>,
    #[serde(default)]
    activity: Vec<Switch>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    check: Option<bool>,
}

/// The write in the generated types.
struct Staged {
    changes: Vec<ConfigChange>,
    activity: Vec<ActivityChange>,
    reason: Option<String>,
}

/// One staged change as the page sends it (`conductor.config.ConfigChange`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: String,
    #[serde(default)]
    value: Option<String>,
}

/// One activity switch as the page sends it (`conductor.config.ActivityChange`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Switch {
    repository: String,
    activity: String,
    #[serde(default)]
    reason: String,
}

impl Change {
    fn typed(self) -> ConfigChange {
        ConfigChange {
            path: self.path,
            value: self.value,
        }
    }
}

impl Switch {
    fn typed(self) -> Result<ActivityChange, String> {
        let activity = match self.activity.as_str() {
            "Active" => Activity::Active,
            "Inactive" => Activity::Inactive,
            other => return Err(format!("activity {other:?} is not Active or Inactive")),
        };
        Ok(ActivityChange {
            repository: MarkedRepository(self.repository),
            activity,
            reason: self.reason,
        })
    }
}

/// A refusal of the write with its status, as `conductor.config.ConfigWriteRefused`.
fn refused(
    status: u16,
    reason: &'static str,
    refusal: ConfigWriteRefusal,
    detail: impl Into<String>,
    problems: &[ConfigProblem],
) -> Refused {
    let body = json!({
        "refusal": format!("{refusal:?}"),
        "detail": detail.into(),
        "problems": problems.iter().map(|problem| json!({
            "path": problem.path, "message": problem.message,
        })).collect::<Vec<_>>(),
    });
    Refused {
        status,
        reason,
        why: format!("{body}\n"),
    }
}

/// Sets the key at `path` (segments split on `.`, each with an optional `[<index>]`) under
/// `root` to `value`, creating each mapping on the way; `None` removes it.
fn set(root: &mut Yaml, path: &str, value: Option<Yaml>) -> Result<(), String> {
    let mut steps: Vec<(String, Option<usize>)> = Vec::new();
    for segment in path.split('.') {
        let (key, index) = match segment.split_once('[') {
            Some((key, rest)) => {
                let index = rest
                    .strip_suffix(']')
                    .and_then(|index| index.parse::<usize>().ok())
                    .ok_or_else(|| format!("{segment:?} is no key[index]"))?;
                (key, Some(index))
            }
            None => (segment, None),
        };
        if key.is_empty() {
            return Err("an empty key".to_owned());
        }
        steps.push((key.to_owned(), index));
    }
    let Some(((last_key, last_index), walk)) = steps.split_last() else {
        return Err("no path".to_owned());
    };
    let mut here = root;
    for (key, index) in walk {
        if !here.is_mapping() {
            *here = Yaml::Mapping(serde_yaml::Mapping::new());
        }
        let map = here.as_mapping_mut().ok_or("not a mapping")?;
        let next = map
            .entry(Yaml::from(key.as_str()))
            .or_insert_with(|| Yaml::Mapping(serde_yaml::Mapping::new()));
        here = match index {
            Some(index) => next
                .as_sequence_mut()
                .and_then(|items| items.get_mut(*index))
                .ok_or_else(|| format!("{key}[{index}] is not in the file"))?,
            None => next,
        };
    }
    if !here.is_mapping() {
        *here = Yaml::Mapping(serde_yaml::Mapping::new());
    }
    let map = here.as_mapping_mut().ok_or("not a mapping")?;
    match (last_index, value) {
        (None, Some(value)) => {
            map.insert(Yaml::from(last_key.as_str()), value);
        }
        (None, None) => {
            map.remove(last_key.as_str());
        }
        (Some(index), value) => {
            let items = map
                .get_mut(last_key.as_str())
                .and_then(Yaml::as_sequence_mut)
                .ok_or_else(|| format!("{last_key} is no list in the file"))?;
            match value {
                Some(value) if *index < items.len() => items[*index] = value,
                Some(value) if *index == items.len() => items.push(value),
                None if *index < items.len() => {
                    items.remove(*index);
                }
                _ => return Err(format!("{last_key}[{index}] is not in the file")),
            }
        }
    }
    Ok(())
}

/// The diff of the instance's keys `before` and `after`: `- path: old` and `+ path: new`, in
/// the order `after` lists them, removals last.
fn diff(before: &[(String, String)], after: &[(String, String)]) -> Vec<String> {
    let mut lines = Vec::new();
    for (path, value) in after {
        match before.iter().find(|(known, _)| known == path) {
            Some((_, old)) if old == value => {}
            Some((_, old)) => {
                lines.push(format!("- {path}: {old}"));
                lines.push(format!("+ {path}: {value}"));
            }
            None => lines.push(format!("+ {path}: {value}")),
        }
    }
    for (path, old) in before {
        if !after.iter().any(|(known, _)| known == path) {
            lines.push(format!("- {path}: {old}"));
        }
    }
    lines
}

/// The word of `activity` as the diff writes it.
fn activity_word(activity: Activity) -> &'static str {
    match activity {
        Activity::Active => "active",
        Activity::Inactive => "inactive",
    }
}

/// Checks the write `body` against the config file of `sources`, and writes it unless it asks
/// only for a check; the answer (`conductor.config.ConfigWritten`) or the refusal.
pub(super) fn write(
    sources: &Sources,
    state: Option<&Path>,
    body: &[u8],
) -> Result<Value, Refused> {
    let staged: Write = serde_json::from_slice(body).map_err(|error| Refused {
        status: 400,
        reason: "Bad Request",
        why: format!(
            "the body is no write {{\"changes\": [{{\"path\", \"value\"}}], \"activity\", \
             \"reason\", \"check\"}}: {error}\n"
        ),
    })?;
    let check = staged.check.unwrap_or(false);
    let changes: Vec<ConfigChange> = staged.changes.into_iter().map(Change::typed).collect();
    let activity: Vec<ActivityChange> = staged
        .activity
        .into_iter()
        .map(Switch::typed)
        .collect::<Result<_, _>>()
        .map_err(|why| Refused {
            status: 400,
            reason: "Bad Request",
            why: format!("{why}\n"),
        })?;
    let staged = Staged {
        changes,
        activity,
        reason: staged.reason,
    };
    let invalid = |problems: Vec<ConfigProblem>| {
        refused(
            422,
            "Unprocessable Content",
            ConfigWriteRefusal::Invalid,
            "the staged config is not valid; nothing was written",
            &problems,
        )
    };
    if staged.changes.is_empty() && staged.activity.is_empty() {
        return Err(refused(
            422,
            "Unprocessable Content",
            ConfigWriteRefusal::NothingStaged,
            "nothing is staged",
            &[],
        ));
    }
    let Some(file) = sources.config_file.clone() else {
        return Err(refused(
            409,
            "Conflict",
            ConfigWriteRefusal::NoConfigFile,
            "this instance runs on the built-in defaults: there is no config file to write",
            &[],
        ));
    };
    let home = &sources.home;
    // Read, change and write the file under its one lock, the one `PUT /v1/config` and the
    // instance calls take ([`config::put::lock`]): a write that read the file before another wrote
    // it would put the old text back, an instance's standing rules among it. Released once the
    // file is written, before the records of the write. A check writes nothing and takes none.
    let held = if check {
        None
    } else {
        Some(config::put::lock(&file).map_err(|error| Refused {
            status: 500,
            reason: "Internal Server Error",
            why: format!("{error:#}\n"),
        })?)
    };
    let (text, mut value) = read(&file, home).map_err(|error| {
        refused(
            409,
            "Conflict",
            ConfigWriteRefusal::NoConfigFile,
            error,
            &[],
        )
    })?;
    let old = config::parse(&text, home).map_err(|problems| {
        invalid(
            problems
                .into_iter()
                .map(|problem| ConfigProblem {
                    path: problem.path,
                    message: format!("the file on disk: {}", problem.message),
                })
                .collect(),
        )
    })?;
    let index = index_of(&old, sources.instance_name.as_deref());
    let prefix = format!("instances[{index}]");
    let mut problems = Vec::new();
    {
        let Some(instance) = value
            .get_mut("instances")
            .and_then(|instances| instances.get_mut(index))
        else {
            return Err(invalid(vec![ConfigProblem {
                path: prefix,
                message: "the file holds no such instance".to_owned(),
            }]));
        };
        for change in &staged.changes {
            let path = format!("{prefix}.{}", change.path);
            if effect(&change.path) == ConfigEffect::Fixed {
                let message = if change
                    .path
                    .split(['.', '['])
                    .next()
                    .is_some_and(|head| CONTROL_PLANE.contains(&head))
                {
                    "the page does not change this key; the control plane writes it, with \
                     `PUT /v1/instances/{name}`"
                } else {
                    "the page does not change this key; `conductor config migrate` moves it"
                };
                problems.push(ConfigProblem {
                    path,
                    message: message.to_owned(),
                });
                continue;
            }
            let parsed = match change.value.as_deref().map(serde_yaml::from_str::<Yaml>) {
                None => None,
                Some(Ok(parsed)) => Some(parsed),
                Some(Err(error)) => {
                    problems.push(ConfigProblem {
                        path,
                        message: format!("not YAML: {error}"),
                    });
                    continue;
                }
            };
            if let Err(message) = set(instance, &change.path, parsed) {
                problems.push(ConfigProblem { path, message });
            }
        }
    }
    for change in &staged.activity {
        if change.reason.trim().is_empty() {
            problems.push(ConfigProblem {
                path: format!("repositories.{}.reason", change.repository.0),
                message: "required when its activity changes".to_owned(),
            });
        }
    }
    let new_text = serde_yaml::to_string(&value).unwrap_or_default();
    let new = match config::parse(&new_text, home) {
        Ok(new) => Some(new),
        Err(found) => {
            problems.extend(found.into_iter().map(|problem| ConfigProblem {
                path: problem.path,
                message: problem.message,
            }));
            None
        }
    };
    if let Some(new) = &new {
        // As `config validate` refuses it: an instance without a prefix.
        problems.extend(
            config::unique_problems(new)
                .into_iter()
                .map(|problem| ConfigProblem {
                    path: problem.path,
                    message: problem.message,
                }),
        );
    }
    let before = old.instances.get(index).map(fields).unwrap_or_default();
    let after = new
        .as_ref()
        .and_then(|new| new.instances.get(index))
        .map(fields)
        .unwrap_or_default();
    let mut lines = if new.is_some() {
        diff(&before, &after)
    } else {
        Vec::new()
    };
    for change in &staged.activity {
        lines.push(format!(
            "+ repositories.{}: {} ({})",
            change.repository.0,
            activity_word(change.activity),
            change.reason.trim()
        ));
    }
    let restart = staged
        .changes
        .iter()
        .any(|change| effect(&change.path) == ConfigEffect::LoopRestart);
    let answer = |backup: Option<String>, decision: Option<String>, problems: &[ConfigProblem]| {
        json!({
            "diff": lines.join("\n"),
            "restart": restart,
            "backup": backup,
            "decision_id": decision,
            "problems": problems.iter().map(|problem| json!({
                "path": problem.path, "message": problem.message,
            })).collect::<Vec<_>>(),
        })
    };
    if check {
        return Ok(answer(None, None, &problems));
    }
    if !problems.is_empty() {
        return Err(invalid(problems));
    }
    let reason = staged
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .ok_or_else(|| {
            refused(
                422,
                "Unprocessable Content",
                ConfigWriteRefusal::ReasonMissing,
                "a write needs the operator's reason",
                &[],
            )
        })?;
    let failed = |what: String| Refused {
        status: 500,
        reason: "Internal Server Error",
        why: format!("{what}\n"),
    };
    let mut backup = None;
    if !staged.changes.is_empty() {
        let at = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            at.year(),
            u8::from(at.month()),
            at.day(),
            at.hour(),
            at.minute(),
            at.second()
        );
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kept = file.with_file_name(format!("{name}.{stamp}.bak"));
        fs::write(&kept, &text)
            .map_err(|error| failed(format!("keep {}: {error}", kept.display())))?;
        let next: PathBuf = file.with_file_name(format!("{name}.{}.next", std::process::id()));
        fs::write(&next, &new_text)
            .and_then(|()| fs::rename(&next, &file))
            .map_err(|error| failed(format!("write {}: {error}", file.display())))?;
        backup = Some(kept.display().to_string());
    }
    drop(held);
    for change in &staged.activity {
        repository::mark_as_operator(
            state,
            &change.repository.0,
            change.activity,
            change.reason.trim(),
            Some(RECORDED_BY.to_owned()),
        )
        .map_err(|error| failed(format!("{error:#}")))?;
    }
    let instance = sources.instance_name.clone().unwrap_or_default();
    let decision = decide::record_as_operator(
        state,
        &decide::OperatorDecision {
            decision_class: DecisionClass::C,
            question: format!("Change the configuration of instance {instance}?"),
            options: "write the staged changes; keep the configuration".to_owned(),
            choice: "write the staged changes".to_owned(),
            reason: reason.to_owned(),
            evidence: format!(
                "{}{}",
                lines.join("\n"),
                backup
                    .as_deref()
                    .map(|kept| format!("\nthe old file is kept as {kept}"))
                    .unwrap_or_default()
            ),
            recorded_by: RECORDED_BY.to_owned(),
            operator_words: reason.to_owned(),
        },
    )
    .map_err(|error| failed(format!("{error:#}")))?;
    Ok(answer(backup, Some(decision), &[]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_creates_replaces_and_removes_a_key() {
        let mut value: Yaml = serde_yaml::from_str("name: a\ncadence: {watch: 3m}\n").unwrap();
        set(&mut value, "cadence.watch", Some(Yaml::from("1m"))).unwrap();
        set(&mut value, "thresholds.disk_low", Some(Yaml::from("200G"))).unwrap();
        assert_eq!(value["cadence"]["watch"], Yaml::from("1m"));
        assert_eq!(value["thresholds"]["disk_low"], Yaml::from("200G"));
        set(&mut value, "cadence.watch", None).unwrap();
        assert!(value["cadence"].get("watch").is_none());
        assert!(set(&mut value, "sources[3].exclude", Some(Yaml::from("x"))).is_err());
    }

    #[test]
    fn the_diff_names_each_changed_key() {
        let before = vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "2".to_owned()),
        ];
        let after = vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "3".to_owned()),
        ];
        assert_eq!(diff(&before, &after), vec!["- b: 2", "+ b: 3"]);
    }

    #[test]
    fn paths_and_names_are_fixed_and_cadence_needs_a_restart() {
        assert_eq!(effect("records"), ConfigEffect::Fixed);
        assert_eq!(effect("checkouts.root"), ConfigEffect::Fixed);
        assert_eq!(effect("cadence.watch"), ConfigEffect::LoopRestart);
        assert_eq!(effect("thresholds.disk_low"), ConfigEffect::WatchTick);
        assert_eq!(effect("controllers.max_working"), ConfigEffect::Cycle);
        // The control plane's text (story:instance-rules-text): the page never changes it.
        assert_eq!(effect("standing_rules"), ConfigEffect::Fixed);
        // The control plane's (story:forge-access-anonymous): the page never changes it.
        assert_eq!(effect("forge_access"), ConfigEffect::Fixed);
    }
}
