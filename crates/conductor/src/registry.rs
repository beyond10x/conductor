//! The host registry (`story:instance-names-unique`, `conductor.host.InstanceRegistry`): every
//! config file that has run on the host, with what each instance of it holds that no other
//! instance on the host may hold.
//!
//! It is one file per host, [`FILE`] in the host's own directory `~/.b10x/conductor/host/`
//! ([`dir`]), whatever config file a command reads; the host file `host.yaml`, which holds the
//! per-host roles, sits beside it. `conductor config validate` writes a valid file's entry, and
//! `conductor session start` writes the entry of the file it starts from ([`register`]), each
//! under the lock [`LOCK`] beside it, by a rename of a whole new file. A registered file that is
//! no longer there is dropped whenever the registry is written.
//!
//! [`collisions`] answers what a config file's instances hold that another registered file's
//! instance holds: a session prefix (equal, or one followed by `-` begins the other), a dashboard
//! port, a state directory or a records directory, each directory compared where it lands.
//! [`others`] answers the instances of every other registered file, which the guard reads to tell
//! whose a session is (`crate::guard::rules`).

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use conductor_model::config::{Checkouts, Config, Instance, InstanceName, SessionPrefix};
use conductor_model::host::{
    CollisionKey, InstanceCollision, InstanceRegistry, RegisteredFile, RegisteredInstance,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::config;

/// The host's own directory under the home directory: one per host, whatever config file a
/// command reads.
pub const DIR: &str = ".b10x/conductor/host";

/// The registry's file in [`DIR`].
pub const FILE: &str = "instances.json";

/// The lock every write of the registry holds, in [`DIR`].
pub const LOCK: &str = "instances.lock";

/// The version the registry carries: `conductor.host.InstanceRegistry`'s invariant.
pub const VERSION: &str = "conductor.host.instances/1";

/// The host's own directory under the home directory `home`.
#[must_use]
pub fn dir(home: &Path) -> PathBuf {
    home.join(DIR)
}

/// The registry's file under the home directory `home`.
#[must_use]
pub fn path(home: &Path) -> PathBuf {
    dir(home).join(FILE)
}

/// What the registry keeps of `instance`, its directories where they land.
#[must_use]
pub fn registered(instance: &Instance) -> RegisteredInstance {
    RegisteredInstance {
        name: instance.name.clone(),
        session_prefix: instance.session_prefix.clone(),
        dashboard_port: instance.dashboard_port,
        state: landed(&instance.state),
        records: landed(&instance.records),
        checkouts: Checkouts {
            root: landed(&instance.checkouts.root),
            trees: landed(&instance.checkouts.trees),
        },
    }
}

/// `path` where it lands ([`config::physical`]), as text.
fn landed(path: &str) -> String {
    config::physical(Path::new(path))
        .to_string_lossy()
        .into_owned()
}

/// The registry under the home directory `home`; an empty one when there is no file.
///
/// # Errors
///
/// The file cannot be read, or is not a registry.
pub fn read(home: &Path) -> Result<InstanceRegistry> {
    let file = path(home);
    match fs::read_to_string(&file) {
        Ok(text) => {
            let value: Value = serde_json::from_str(&text)
                .with_context(|| format!("the host registry {} is no JSON", file.display()))?;
            from_json(&value)
                .with_context(|| format!("the host registry {} is no registry", file.display()))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(InstanceRegistry {
            version: VERSION.to_owned(),
            files: Vec::new(),
        }),
        Err(error) => Err(error).with_context(|| format!("read {}", file.display())),
    }
}

/// Writes the entry of the config file `file` (absolute), holding `config`'s instances, into the
/// registry under `home`, replacing the file's earlier entry and dropping every registered file
/// that is no longer there.
///
/// # Errors
///
/// The host directory, its lock or the registry cannot be written, or the registry there is not
/// one.
pub fn register(home: &Path, file: &Path, config: &Config) -> Result<()> {
    let dir = dir(home);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK))
        .with_context(|| format!("open {}", dir.join(LOCK).display()))?;
    lock.lock()
        .with_context(|| format!("lock {}", dir.join(LOCK).display()))?;
    let mut registry = read(home)?;
    let key = text_of(file);
    registry
        .files
        .retain(|entry| entry.config != key && Path::new(&entry.config).exists());
    registry.files.push(RegisteredFile {
        config: key,
        registered_at: crate::host::now(OffsetDateTime::now_utc()),
        instances: config.instances.iter().map(registered).collect(),
    });
    registry.files.sort_by(|a, b| a.config.cmp(&b.config));
    let target = path(home);
    let staged = dir.join(format!("{FILE}.{}.tmp", std::process::id()));
    let text = serde_json::to_string_pretty(&to_json(&registry))? + "\n";
    fs::write(&staged, text).with_context(|| format!("write {}", staged.display()))?;
    fs::rename(&staged, &target).with_context(|| format!("write {}", target.display()))?;
    drop(lock);
    Ok(())
}

/// The instances of every registered config file but `file` that is still there, each with its
/// file.
///
/// # Errors
///
/// The registry cannot be read.
pub fn others(home: &Path, file: &Path) -> Result<Vec<(String, RegisteredInstance)>> {
    let key = text_of(file);
    Ok(read(home)?
        .files
        .into_iter()
        .filter(|entry| entry.config != key && Path::new(&entry.config).exists())
        .flat_map(|entry| {
            let config = entry.config;
            entry
                .instances
                .into_iter()
                .map(move |instance| (config.clone(), instance))
        })
        .collect())
}

/// What each instance of `config`, the file `file`, holds that an instance of another registered
/// file holds, by the instance's index in the file: one collision per key and instance, against
/// the first registered instance that holds it.
///
/// # Errors
///
/// The registry cannot be read.
pub fn collisions(
    home: &Path,
    file: &Path,
    config: &Config,
) -> Result<Vec<(usize, InstanceCollision)>> {
    let others = others(home, file)?;
    let mut found = Vec::new();
    for (index, instance) in config.instances.iter().enumerate() {
        let this = registered(instance);
        for key in [
            CollisionKey::SessionPrefix,
            CollisionKey::DashboardPort,
            CollisionKey::State,
            CollisionKey::Records,
        ] {
            let Some((other_config, other)) =
                others.iter().find(|(_, other)| holds(key, &this, other))
            else {
                continue;
            };
            found.push((
                index,
                InstanceCollision {
                    key,
                    value: value_of(key, &this),
                    instance: this.name.clone(),
                    config: text_of(file),
                    other_instance: other.name.clone(),
                    other_config: other_config.clone(),
                },
            ));
        }
    }
    Ok(found)
}

/// Whether `this` and `other` hold the same `key`.
fn holds(key: CollisionKey, this: &RegisteredInstance, other: &RegisteredInstance) -> bool {
    match key {
        CollisionKey::SessionPrefix => match (&this.session_prefix, &other.session_prefix) {
            (Some(SessionPrefix(a)), Some(SessionPrefix(b))) => nests(a, b),
            _ => false,
        },
        CollisionKey::DashboardPort => this.dashboard_port == other.dashboard_port,
        CollisionKey::State => this.state == other.state,
        CollisionKey::Records => this.records == other.records,
    }
}

/// Whether the prefixes `a` and `b` are one, or one followed by `-` begins the other, so that one
/// session name would carry both.
#[must_use]
pub fn nests(a: &str, b: &str) -> bool {
    a == b || config::carries(a, b) || config::carries(b, a)
}

/// The value of `key` `instance` holds, as a message shows it.
fn value_of(key: CollisionKey, instance: &RegisteredInstance) -> String {
    match key {
        CollisionKey::SessionPrefix => instance
            .session_prefix
            .as_ref()
            .map_or_else(String::new, |prefix| format!("{:?}", prefix.0)),
        CollisionKey::DashboardPort => instance.dashboard_port.to_string(),
        CollisionKey::State => instance.state.clone(),
        CollisionKey::Records => instance.records.clone(),
    }
}

/// The key of a collision as the config file writes it.
#[must_use]
pub fn key_name(key: CollisionKey) -> &'static str {
    match key {
        CollisionKey::SessionPrefix => "session_prefix",
        CollisionKey::DashboardPort => "dashboard_port",
        CollisionKey::State => "state",
        CollisionKey::Records => "records",
    }
}

/// What a collision says, after its YAML path: the value, both instances and both files.
#[must_use]
pub fn message(collision: &InstanceCollision, registry: &Path) -> String {
    let what = match collision.key {
        CollisionKey::SessionPrefix => "session names, slices and units",
        CollisionKey::DashboardPort => "a dashboard port",
        CollisionKey::State => "a state directory",
        CollisionKey::Records => "a records directory",
    };
    let held = if collision.key == CollisionKey::SessionPrefix {
        "is or nests with the session_prefix of"
    } else {
        "is held by"
    };
    format!(
        "{} {held} instance {} of {} (host registry {}): instance {} of {} would share {what} \
         with it; write one no instance on the host holds",
        collision.value,
        collision.other_instance.0,
        collision.other_config,
        registry.display(),
        collision.instance.0,
        collision.config,
    )
}

/// `file` as the registry keys it.
fn text_of(file: &Path) -> String {
    file.to_string_lossy().into_owned()
}

/// The registry as its file writes it.
fn to_json(registry: &InstanceRegistry) -> Value {
    json!({
        "version": registry.version,
        "files": registry.files.iter().map(|file| json!({
            "config": file.config,
            "registered_at": file.registered_at.0,
            "instances": file.instances.iter().map(|instance| json!({
                "name": instance.name.0,
                "session_prefix": instance.session_prefix.as_ref().map(|prefix| &prefix.0),
                "dashboard_port": instance.dashboard_port,
                "state": instance.state,
                "records": instance.records,
                "checkouts": {
                    "root": instance.checkouts.root,
                    "trees": instance.checkouts.trees,
                },
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// The registry its file holds.
fn from_json(value: &Value) -> Result<InstanceRegistry> {
    let text = |value: &Value, what: &str| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("{what} is not text"))
    };
    let version = text(&value["version"], "version")?;
    if version != VERSION {
        return Err(anyhow!("version {version:?} is not {VERSION}"));
    }
    let files = value["files"]
        .as_array()
        .ok_or_else(|| anyhow!("files is not a list"))?
        .iter()
        .map(|file| {
            let instances = file["instances"]
                .as_array()
                .ok_or_else(|| anyhow!("instances is not a list"))?
                .iter()
                .map(|instance| {
                    Ok(RegisteredInstance {
                        name: InstanceName(text(&instance["name"], "name")?),
                        session_prefix: match &instance["session_prefix"] {
                            Value::Null => None,
                            prefix => Some(SessionPrefix(text(prefix, "session_prefix")?)),
                        },
                        dashboard_port: instance["dashboard_port"]
                            .as_i64()
                            .ok_or_else(|| anyhow!("dashboard_port is not a number"))?,
                        state: text(&instance["state"], "state")?,
                        records: text(&instance["records"], "records")?,
                        checkouts: Checkouts {
                            root: text(&instance["checkouts"]["root"], "checkouts.root")?,
                            trees: text(&instance["checkouts"]["trees"], "checkouts.trees")?,
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(RegisteredFile {
                config: text(&file["config"], "config")?,
                registered_at: Timestamp(text(&file["registered_at"], "registered_at")?),
                instances,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(InstanceRegistry { version, files })
}
