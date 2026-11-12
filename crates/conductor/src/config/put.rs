//! `conductor.config.PutConfig` (`story:config-globals`, design architecture-next § 9): one key
//! changed at one level of the config file, validated, written, committed to the config
//! repository with the actor and the reason, and recorded in the daemon's store as a
//! `ConfigRevision` with its `ConfigChanged`. [`put_config`] is the one function `PUT /v1/config`
//! and `conductor config put` call. The daemon reconciles on its next tick: it reads the file's
//! text changed (`ReconcileReason::ConfigChange`).
//!
//! The file is written whole, as YAML reads it back, so a comment of the file is not kept. A
//! file in no git checkout, or one its checkout ignores, is written and not committed: its
//! revision's `commit` is empty. A commit that fails puts the old text back, when the file still
//! holds this run's text, and nothing is recorded. One run at a time reads, writes, commits and
//! records: each holds an exclusive lock on `.<name>.lock` beside the file.

use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::ConfigRevisionStorage;
use conductor_model::config::{
    AnyConfigRevision, ConfigChanged, ConfigInvalid, ConfigRevision, ConfigRevisionData, Level,
    LevelScopeMismatch, PutConfig, PutConfigOutcome, RevisionId,
};
use conductor_model::operator::{ActorKind, ActorNotHuman};
use conductor_model::primitives::Uuid;
use serde_yaml::{Mapping, Value as Yaml};

use super::{Problem, VERSION_2, parse};
use crate::store::{Store, level_name};

/// Why a change was refused; a refusal wrote, committed and recorded nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutRefusal {
    /// The declared outcome: `LevelScopeMismatch`, `ActorNotHuman` or `Invalid`.
    pub outcome: PutConfigOutcome,
    /// For `Invalid`, every problem of the changed file, each by its YAML path.
    pub problems: Vec<Problem>,
}

impl fmt::Display for PutRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.outcome {
            PutConfigOutcome::LevelScopeMismatch { error } => write!(
                f,
                "LevelScopeMismatch: a Global change names no instance and no role, an Instance \
                 change names an instance, a Role change an instance and a role, and none is \
                 written at Default; this {} change does not fit, and nothing was written",
                level_name(error.level)
            ),
            PutConfigOutcome::ActorNotHuman { error } => write!(
                f,
                "ActorNotHuman: {} changes only a key the tuning grant (globals.tuning) names, \
                 to a value inside its range; nothing was written",
                crate::actor::written(&error.actor)
            ),
            PutConfigOutcome::Invalid { error } => {
                let said: Vec<String> = self.problems.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "ConfigInvalid: {}; nothing was written (the change of {})",
                    said.join("; "),
                    error.path
                )
            }
            PutConfigOutcome::Changed { .. } => f.write_str("changed"),
        }
    }
}

impl PutRefusal {
    fn invalid(input: &PutConfig, path: &str, message: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            outcome: PutConfigOutcome::Invalid {
                error: ConfigInvalid {
                    path: input.path.clone(),
                },
            },
            problems: vec![Problem {
                path: path.to_owned(),
                message: message.into(),
            }],
        })
    }
}

/// The YAML path of the mapping a change at `input.level` is written under, and that mapping in
/// `file`; a refusal when the file does not hold it.
fn scope<'f>(
    file: &'f mut Yaml,
    input: &PutConfig,
) -> Result<(String, &'f mut Mapping), Box<PutRefusal>> {
    let Some(root) = file.as_mapping_mut() else {
        return Err(PutRefusal::invalid(input, "the file", "is not a mapping"));
    };
    if input.level == Level::Global {
        let globals = root
            .entry(Yaml::from("globals"))
            .or_insert_with(|| Yaml::Mapping(Mapping::new()));
        return match globals.as_mapping_mut() {
            Some(map) => Ok(("globals".to_owned(), map)),
            None => Err(PutRefusal::invalid(input, "globals", "is not a mapping")),
        };
    }
    let name = input
        .instance
        .as_ref()
        .map(|name| name.0.clone())
        .unwrap_or_default();
    let instances = root
        .get_mut("instances")
        .and_then(Yaml::as_sequence_mut)
        .ok_or_else(|| PutRefusal::invalid(input, "instances", "names no instance"))?;
    let Some((index, instance)) = instances
        .iter_mut()
        .enumerate()
        .find(|(_, instance)| instance.get("name").and_then(Yaml::as_str) == Some(&name))
    else {
        return Err(PutRefusal::invalid(
            input,
            "instances",
            format!("names no instance {name:?}"),
        ));
    };
    let at = format!("instances[{index}]");
    let Some(instance) = instance.as_mapping_mut() else {
        return Err(PutRefusal::invalid(input, &at, "is not a mapping"));
    };
    if input.level == Level::Instance {
        return Ok((at, instance));
    }
    let role = input
        .role
        .as_ref()
        .map(|role| role.0.clone())
        .unwrap_or_default();
    let refusal = || {
        PutRefusal::invalid(
            input,
            &format!("{at}.roles"),
            format!(
                "writes no role {role:?}: write the role in the instance's roles first, since a \
                 list the instance writes replaces the built-in roles"
            ),
        )
    };
    let roles = instance
        .get_mut("roles")
        .and_then(Yaml::as_sequence_mut)
        .ok_or_else(refusal)?;
    let found = roles
        .iter_mut()
        .enumerate()
        .find(|(_, one)| one.get("role").and_then(Yaml::as_str) == Some(&role));
    match found {
        Some((index, one)) => match one.as_mapping_mut() {
            Some(map) => Ok((format!("{at}.roles[{index}]"), map)),
            None => Err(refusal()),
        },
        None => Err(refusal()),
    }
}

/// Sets (or, with no `value`, removes) the dotted `path` under `map`, making each mapping on the
/// way; a refusal when a step is not a mapping.
fn set(
    map: &mut Mapping,
    path: &str,
    value: Option<Yaml>,
    under: &str,
    input: &PutConfig,
) -> Result<(), Box<PutRefusal>> {
    let mut steps: Vec<&str> = path.split('.').collect();
    let last = steps.pop().unwrap_or_default();
    if last.is_empty() || steps.iter().any(|step| step.is_empty()) {
        return Err(PutRefusal::invalid(
            input,
            under,
            format!("{path:?} is not a key path: write keys joined by '.'"),
        ));
    }
    let mut at = map;
    let mut walked = under.to_owned();
    for step in steps {
        walked = format!("{walked}.{step}");
        let next = at
            .entry(Yaml::from(step))
            .or_insert_with(|| Yaml::Mapping(Mapping::new()));
        at = next
            .as_mapping_mut()
            .ok_or_else(|| PutRefusal::invalid(input, &walked, "is not a mapping"))?;
    }
    match value {
        Some(value) => {
            at.insert(Yaml::from(last), value);
        }
        None => {
            at.remove(last);
        }
    }
    Ok(())
}

/// Whether an agent's change lies inside the tuning grant of the file `config`: the grant names
/// the key's path and the value is a number inside its range.
fn granted(config: &conductor_model::config::Config, input: &PutConfig) -> bool {
    let Some(value) = input
        .value
        .as_deref()
        .and_then(|value| value.trim().parse::<f64>().ok())
    else {
        return false;
    };
    config.globals.tuning.keys.iter().any(|key| {
        let bound = |decimal: &conductor_model::primitives::Decimal| decimal.0.parse::<f64>().ok();
        key.path == input.path
            && bound(&key.min).is_some_and(|min| value >= min)
            && bound(&key.max).is_some_and(|max| value <= max)
    })
}

/// Runs `git -C <dir> <args>`; its standard output when it succeeds.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = crate::instance::command("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("run git")?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(anyhow!(
            "git {}: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Commits `file` alone with `message`, when it lies in a git checkout that does not ignore it:
/// the commit's id, or none for a file in no checkout or one the checkout ignores.
fn commit(file: &Path, subject: &str, body: &str) -> Result<Option<String>> {
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("{} has no file name", file.display()))?;
    if git(dir, &["rev-parse", "--is-inside-work-tree"]).is_err()
        || git(dir, &["check-ignore", "-q", "--", name]).is_ok()
    {
        return Ok(None);
    }
    git(dir, &["add", "--", name])?;
    git(
        dir,
        &["commit", "-q", "-m", subject, "-m", body, "--", name],
    )?;
    git(dir, &["rev-parse", "HEAD"]).map(Some)
}

/// The exclusive lock of the config file `file`, held while the returned file is open: on
/// `.<name>.lock` beside it, since the file itself is replaced by a rename on each write. Waits
/// for another run that holds it.
///
/// # Errors
///
/// The lock file cannot be opened or locked.
pub(super) fn lock(file: &Path) -> Result<fs::File> {
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("{} has no file name", file.display()))?;
    let path = dir.join(format!(".{name}.lock"));
    let held = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open the config file's lock {}", path.display()))?;
    held.lock()
        .with_context(|| format!("lock {}", path.display()))?;
    Ok(held)
}

/// Writes `new` over the config `file`, which held `old`, by a rename beside it, and commits it
/// with `subject` and `body` ([`commit`]): the commit's id, or none for a file in no checkout.
/// Held under [`lock`].
///
/// # Errors
///
/// The file cannot be written, or the commit fails: the old text is put back when the file still
/// holds `new`, else it is left as it is.
pub(super) fn replace(
    file: &Path,
    old: &str,
    new: &str,
    subject: &str,
    body: &str,
) -> Result<Option<String>> {
    let next = file.with_extension("yaml.next");
    fs::write(&next, new).with_context(|| format!("write {}", next.display()))?;
    fs::rename(&next, file).with_context(|| format!("write {}", file.display()))?;
    match commit(file, subject, body) {
        Ok(committed) => Ok(committed),
        Err(error) => {
            // Only this run's own text is taken back: a file that holds another text now was
            // changed by someone else since, and is left as it is.
            let ours = fs::read_to_string(file).is_ok_and(|now| now == new);
            if !ours {
                return Err(error.context(format!(
                    "commit {}; the file was changed since this run wrote it, so it is left as \
                     it is, and nothing was recorded",
                    file.display()
                )));
            }
            fs::write(&next, old)
                .and_then(|()| fs::rename(&next, file))
                .with_context(|| format!("put the old text back into {}", file.display()))?;
            Err(error.context(format!(
                "commit {}; the old text is back and nothing was recorded",
                file.display()
            )))
        }
    }
}

/// Changes one key of the config file `file` (`~` read as `home`) as `input` says, and records
/// the revision in `store`, the daemon's: the refusal when the level does not fit its scope, the
/// actor is an agent outside the tuning grant, or the changed file does not validate.
///
/// # Errors
///
/// The file cannot be read or written, the commit fails (the old text is put back), or the
/// revision cannot be recorded.
pub fn put_config(
    file: &Path,
    home: &Path,
    store: &mut Store,
    input: &PutConfig,
) -> Result<Result<ConfigRevisionData, PutRefusal>> {
    let fits = match input.level {
        Level::Default => false,
        Level::Global => input.instance.is_none() && input.role.is_none(),
        Level::Instance => input.instance.is_some() && input.role.is_none(),
        Level::Role => input.instance.is_some() && input.role.is_some(),
    };
    if !fits {
        return Ok(Err(PutRefusal {
            outcome: PutConfigOutcome::LevelScopeMismatch {
                error: LevelScopeMismatch { level: input.level },
            },
            problems: Vec::new(),
        }));
    }
    // Read, write, commit and record under one exclusive lock: a run that read the file before
    // another wrote it would write the others' changes away, or put its old text back over them.
    let _held = lock(file)?;
    let old = fs::read_to_string(file)
        .with_context(|| format!("read the config file {}", file.display()))?;
    let mut yaml: Yaml = match serde_yaml::from_str(&old) {
        Ok(yaml) => yaml,
        Err(_) => {
            let problems = parse(&old, home).err().unwrap_or_default();
            return Ok(Err(PutRefusal {
                outcome: PutConfigOutcome::Invalid {
                    error: ConfigInvalid {
                        path: input.path.clone(),
                    },
                },
                problems,
            }));
        }
    };
    if yaml.get("version").and_then(Yaml::as_str) != Some(VERSION_2) {
        return Ok(Err(*PutRefusal::invalid(
            input,
            "version",
            format!("is not {VERSION_2}: run conductor config migrate first"),
        )));
    }
    let value = input.value.as_deref().map(|text| {
        serde_yaml::from_str::<Yaml>(text).unwrap_or_else(|_| Yaml::from(text.to_owned()))
    });
    let under = match scope(&mut yaml, input) {
        Ok((under, map)) => {
            if let Err(refusal) = set(map, &input.path, value, &under, input) {
                return Ok(Err(*refusal));
            }
            under
        }
        Err(refusal) => return Ok(Err(*refusal)),
    };
    let new = serde_yaml::to_string(&yaml).context("write the changed config as YAML")?;
    let config = match parse(&new, home) {
        Ok(config) => config,
        Err(problems) => {
            return Ok(Err(PutRefusal {
                outcome: PutConfigOutcome::Invalid {
                    error: ConfigInvalid {
                        path: input.path.clone(),
                    },
                },
                problems,
            }));
        }
    };
    if input.actor.kind != ActorKind::Human && !granted(&config, input) {
        return Ok(Err(PutRefusal {
            outcome: PutConfigOutcome::ActorNotHuman {
                error: ActorNotHuman {
                    actor: input.actor.clone(),
                },
            },
            problems: Vec::new(),
        }));
    }
    let who = crate::actor::written(&input.actor);
    let subject = format!(
        "config: {under}.{} at {} by {who}",
        input.path,
        level_name(input.level).to_ascii_lowercase()
    );
    let body = format!("{}\n\nActor: {who}", input.reason);
    let committed = replace(file, &old, &new, &subject, &body)?;
    let data = ConfigRevisionData {
        revision_id: RevisionId(Uuid(eventlog_core::new_event_id())),
        actor: input.actor.clone(),
        level: input.level,
        instance: input.instance.clone(),
        role: input.role.clone(),
        path: input.path.clone(),
        value: input.value.clone(),
        reason: input.reason.clone(),
        commit: committed.unwrap_or_default(),
        changed_at: input.changed_at.clone(),
    };
    ConfigRevisionStorage::put(
        store,
        AnyConfigRevision::Committed(ConfigRevision::new(data.clone())).snapshot(),
    );
    store
        .check()
        .map_err(|error| anyhow!("record the config change: {error}"))?;
    Ok(Ok(data))
}

/// The `ConfigChanged` a revision publishes.
#[must_use]
pub fn changed(data: &ConfigRevisionData) -> ConfigChanged {
    ConfigChanged {
        revision_id: data.revision_id.clone(),
        actor: data.actor.clone(),
        level: data.level,
        instance_name: data.instance.clone(),
        role: data.role.clone(),
        path: data.path.clone(),
        value: data.value.clone(),
        reason: data.reason.clone(),
    }
}
