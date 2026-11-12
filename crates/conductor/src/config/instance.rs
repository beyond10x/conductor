//! `conductor.config.AddInstance`, `PutInstance` and `RemoveInstance` (design A6, decision D2):
//! one whole instance of the config file added, replaced or removed, the whole file validated as
//! `conductor config validate` reads it ([`super::parse`]), written by rename, committed to the
//! config repository with the actor and the reason, and recorded in the daemon's store as a
//! `ConfigRevision` at the instance level with the path `.`: its value the instance's mapping as
//! the file writes it, none for a removal. `POST /v1/instances`, `PUT /v1/instances/{name}` and
//! `DELETE /v1/instances/{name}` call these ([`crate::daemon::api`]).
//!
//! They share [`super::put`]'s lock, its write by rename and its commit: one run at a time reads,
//! writes, commits and records, and a commit that fails puts the old text back. A refusal writes,
//! commits and records nothing. The daemon reconciles on its next tick, as for any change of the
//! file: an instance added is started, one replaced restarted where its runtime's placement or
//! binary changed, one removed stopped.
//!
//! What a mapping may say is what the file may say for an instance: nothing else is checked here,
//! and nothing of the mapping reaches anything but the file (decision D8).

use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::ConfigRevisionStorage;
use conductor_model::config::{
    AddInstance, AnyConfigRevision, ConfigRevision, ConfigRevisionData, InstanceName, Level,
    PutInstance, RemoveInstance, RevisionId,
};
use conductor_model::operator::{Actor, ActorKind};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_yaml::{Mapping, Value as Yaml};

use super::{Problem, VERSION_2, parse};
use crate::store::Store;

/// Why a change of a whole instance was refused; nothing was written, committed or recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `conductor.operator.ActorNotHuman`: only a person or the control plane changes an
    /// instance.
    ActorNotHuman(Box<Actor>),
    /// `conductor.config.InstanceExists`: the file names an instance of this name already.
    Exists(String),
    /// `conductor.instance.UnknownInstance`: the file names no instance of this name.
    Unknown(String),
    /// `conductor.config.ConfigInvalid`: the file with the change does not validate; every
    /// problem by its YAML path, the first the one named.
    Invalid(Vec<Problem>),
}

impl Refusal {
    /// The YAML path an invalid change is named by: its first problem's.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Invalid(problems) => problems.first().map(|problem| problem.path.as_str()),
            _ => None,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActorNotHuman(actor) => write!(
                f,
                "ActorNotHuman: {} changes no instance; only a person or the control plane does, \
                 and nothing was written",
                crate::actor::written(actor)
            ),
            Self::Exists(name) => write!(
                f,
                "InstanceExists: the config names an instance {name} already; nothing was written"
            ),
            Self::Unknown(name) => write!(
                f,
                "UnknownInstance: the config names no instance {name}; nothing was written"
            ),
            Self::Invalid(problems) => {
                let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
                write!(f, "{}", said.join("; "))
            }
        }
    }
}

/// Which change of a whole instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Add,
    Put,
    Remove,
}

impl Change {
    const fn verb(self) -> &'static str {
        match self {
            Self::Add => "added",
            Self::Put => "replaced",
            Self::Remove => "removed",
        }
    }
}

/// What one change asks.
struct Asked<'a> {
    change: Change,
    actor: &'a Actor,
    name: &'a str,
    mapping: Option<&'a str>,
    reason: &'a str,
    changed_at: &'a Timestamp,
}

fn invalid(path: &str, message: impl Into<String>) -> Refusal {
    Refusal::Invalid(vec![Problem {
        path: path.to_owned(),
        message: message.into(),
    }])
}

/// `conductor.config.AddInstance`: `input.mapping`, one instance's mapping as YAML text, added to
/// the config file `file` (`~` read as `home`) under `input.name`, recorded in `store`, the
/// daemon's. The mapping names the instance or names none, and is then given the name.
///
/// # Errors
///
/// The file cannot be read or written, the commit fails (the old text is put back), or the
/// revision cannot be recorded.
pub fn add_instance(
    file: &Path,
    home: &Path,
    store: &mut Store,
    input: &AddInstance,
) -> Result<Result<ConfigRevisionData, Refusal>> {
    change(
        file,
        home,
        store,
        &Asked {
            change: Change::Add,
            actor: &input.actor,
            name: &input.name.0,
            mapping: Some(&input.mapping),
            reason: &input.reason,
            changed_at: &input.changed_at,
        },
    )
}

/// `conductor.config.PutInstance`: the instance `input.name` of the config file replaced by
/// `input.mapping`, as [`add_instance`] adds one.
///
/// # Errors
///
/// As [`add_instance`].
pub fn put_instance(
    file: &Path,
    home: &Path,
    store: &mut Store,
    input: &PutInstance,
) -> Result<Result<ConfigRevisionData, Refusal>> {
    change(
        file,
        home,
        store,
        &Asked {
            change: Change::Put,
            actor: &input.actor,
            name: &input.name.0,
            mapping: Some(&input.mapping),
            reason: &input.reason,
            changed_at: &input.changed_at,
        },
    )
}

/// `conductor.config.RemoveInstance`: the instance `input.name` removed from the config file, as
/// [`add_instance`] adds one; its data is the daemon's to keep.
///
/// # Errors
///
/// As [`add_instance`].
pub fn remove_instance(
    file: &Path,
    home: &Path,
    store: &mut Store,
    input: &RemoveInstance,
) -> Result<Result<ConfigRevisionData, Refusal>> {
    change(
        file,
        home,
        store,
        &Asked {
            change: Change::Remove,
            actor: &input.actor,
            name: &input.name.0,
            mapping: None,
            reason: &input.reason,
            changed_at: &input.changed_at,
        },
    )
}

/// The mapping `text` as the instance `name`'s: its `name` first, given where it names none; a
/// refusal when it is no mapping or names another instance.
fn named(text: &str, name: &str) -> Result<Mapping, Refusal> {
    let Ok(Yaml::Mapping(written)) = serde_yaml::from_str::<Yaml>(text) else {
        return Err(invalid(
            "mapping",
            "is not one instance's mapping: write the instance's keys as the config file writes \
             them",
        ));
    };
    match written.get("name") {
        None | Some(Yaml::Null) => {}
        Some(Yaml::String(own)) if own == name => {}
        Some(_) => {
            return Err(invalid(
                "name",
                format!(
                    "names another instance than {name:?}: the instance is named by the path, \
                     and a replace does not rename one"
                ),
            ));
        }
    }
    let mut mapping = Mapping::new();
    mapping.insert(Yaml::from("name"), Yaml::from(name));
    for (key, value) in written {
        if key.as_str() != Some("name") {
            mapping.insert(key, value);
        }
    }
    Ok(mapping)
}

fn change(
    file: &Path,
    home: &Path,
    store: &mut Store,
    asked: &Asked<'_>,
) -> Result<Result<ConfigRevisionData, Refusal>> {
    if !matches!(asked.actor.kind, ActorKind::Human | ActorKind::ControlPlane) {
        return Ok(Err(Refusal::ActorNotHuman(Box::new(asked.actor.clone()))));
    }
    let name = asked.name;
    let mapping = match asked.mapping.map(|text| named(text, name)).transpose() {
        Ok(mapping) => mapping,
        Err(refusal) => return Ok(Err(refusal)),
    };
    // Read, write, commit and record under the config file's one lock (`config/put.rs`).
    let _held = super::put::lock(file)?;
    let old = fs::read_to_string(file)
        .with_context(|| format!("read the config file {}", file.display()))?;
    let Ok(mut yaml) = serde_yaml::from_str::<Yaml>(&old) else {
        return Ok(Err(Refusal::Invalid(
            parse(&old, home).err().unwrap_or_default(),
        )));
    };
    if yaml.get("version").and_then(Yaml::as_str) != Some(VERSION_2) {
        return Ok(Err(invalid(
            "version",
            format!("is not {VERSION_2}: run conductor config migrate first"),
        )));
    }
    let Some(root) = yaml.as_mapping_mut() else {
        return Ok(Err(invalid("the file", "is not a mapping")));
    };
    let Some(instances) = root
        .entry(Yaml::from("instances"))
        .or_insert_with(|| Yaml::Sequence(Vec::new()))
        .as_sequence_mut()
    else {
        return Ok(Err(invalid("instances", "is not a list")));
    };
    let at = instances
        .iter()
        .position(|instance| instance.get("name").and_then(Yaml::as_str) == Some(name));
    match (asked.change, at, mapping.clone()) {
        (Change::Add, Some(_), _) => return Ok(Err(Refusal::Exists(name.to_owned()))),
        (Change::Put | Change::Remove, None, _) => {
            return Ok(Err(Refusal::Unknown(name.to_owned())));
        }
        (Change::Add, None, Some(mapping)) => instances.push(Yaml::Mapping(mapping)),
        (Change::Put, Some(index), Some(mapping)) => instances[index] = Yaml::Mapping(mapping),
        (Change::Remove, Some(index), _) => {
            instances.remove(index);
        }
        (Change::Add | Change::Put, _, None) => {
            return Ok(Err(invalid(
                "mapping",
                "is missing: write the instance's mapping",
            )));
        }
    }
    let new = serde_yaml::to_string(&yaml).context("write the changed config as YAML")?;
    if let Err(problems) = parse(&new, home) {
        return Ok(Err(Refusal::Invalid(problems)));
    }
    let value = mapping
        .map(|mapping| serde_yaml::to_string(&Yaml::Mapping(mapping)))
        .transpose()
        .context("write the instance's mapping as YAML")?;
    let who = crate::actor::written(asked.actor);
    let subject = format!("config: instance {name} {} by {who}", asked.change.verb());
    let body = format!("{}\n\nActor: {who}", asked.reason);
    let committed = super::put::replace(file, &old, &new, &subject, &body)?;
    let data = ConfigRevisionData {
        revision_id: RevisionId(Uuid(eventlog_core::new_event_id())),
        actor: asked.actor.clone(),
        level: Level::Instance,
        instance: Some(InstanceName(name.to_owned())),
        role: None,
        path: ".".to_owned(),
        value,
        reason: asked.reason.to_owned(),
        commit: committed.unwrap_or_default(),
        changed_at: asked.changed_at.clone(),
    };
    ConfigRevisionStorage::put(
        store,
        AnyConfigRevision::Committed(ConfigRevision::new(data.clone())).snapshot(),
    );
    store
        .check()
        .map_err(|error| anyhow!("record the instance change: {error}"))?;
    Ok(Ok(data))
}
