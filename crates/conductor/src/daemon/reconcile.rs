//! The reconciler of conductord (`story:daemon-reconcile`, design architecture-next § 4): the
//! config file is the desired set of instances, the runtimes the daemon started and that still
//! run are the actual set, and one pass makes the second equal the first.
//!
//! A pass ([`Daemon::pass`]) reads the config file and, for each instance:
//! - starts its runtime (`conductor instance serve`, the unit [`unit_name`]) when none runs:
//!   never started, or started and no longer running (recorded as `LostRuntime`);
//! - restarts it when it runs on another placement or another binary (path or `--version`) than
//!   the config and the daemon's binary now say;
//! - leaves it alone when it matches, and records nothing.
//!
//! and stops each runtime it started whose instance the config no longer names. Each action is
//! one `conductor.instance.Reconciliation` in the daemon's store; a start or stop the service
//! manager refuses is recorded `Refused`, with what it said as the detail. What the daemon
//! started is kept beside its store in [`RUNTIMES_FILE`], so a restarted daemon finds the
//! runtimes it started running and does nothing.
//!
//! A pass runs ([`Daemon::due`]) at the first tick, when the config file's text changed
//! (`ConfigChange`), when a runtime it started stopped running (`LostRuntime`), and every
//! `daemon.cadence.reconcile` (`Cadence`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::config::{Config, Instance, InstanceName, SessionName};
use conductor_model::instance::{
    AnyReconciliation, InstanceState, ReconcileAction, ReconcileReason, ReconcileResult,
    Reconciliation, ReconciliationData, ReconciliationId,
};
use conductor_model::primitives::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::units::Units;
use crate::confine::systemd::{self, Service};
use crate::store::Store;

/// What the daemon started, beside its store: one [`Started`] per instance, by name.
pub const RUNTIMES_FILE: &str = "runtimes.json";

/// The unit an instance's runtime runs in: `conductor_<instance>-runtime.service`, escaped as a
/// session's unit is ([`systemd::unit_name`]).
#[must_use]
pub fn unit_name(instance: &str) -> String {
    systemd::unit_name(&SessionName(format!("{instance}-runtime")))
}

/// The conductor binary a runtime runs: its path and what its `--version` printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    /// Where it is.
    pub path: PathBuf,
    /// Its version, the last word of its `--version`, or `unknown`.
    pub version: String,
}

/// The binary at `path`, its version read from `<path> --version`; `unknown` when that fails.
#[must_use]
pub fn probe(path: &Path) -> Binary {
    let version = Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .last()
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned());
    Binary {
        path: path.to_owned(),
        version,
    }
}

/// One runtime the daemon started, as [`RUNTIMES_FILE`] keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Started {
    /// Its unit.
    pub unit: String,
    /// The placement it was started on.
    pub placement: String,
    /// The binary it runs.
    pub binary: String,
    /// That binary's version.
    pub version: String,
    /// When the daemon last saw it running, RFC 3339.
    pub last_seen: Option<String>,
}

/// One action a pass took, as it was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    /// The instance.
    pub instance: String,
    /// What was done.
    pub action: ReconcileAction,
    /// Why.
    pub reason: ReconcileReason,
    /// What came of it.
    pub result: ReconcileResult,
    /// What the service manager said, when it refused.
    pub detail: Option<String>,
}

/// conductord's reconciler over the unit control `U`.
pub struct Daemon<U: Units> {
    /// The config file, the desired state.
    pub config_file: PathBuf,
    /// The home directory the file's `~` stands for.
    pub home: PathBuf,
    /// The daemon's own directory: its store and [`RUNTIMES_FILE`].
    pub dir: PathBuf,
    /// The binary every runtime is to run.
    pub binary: Binary,
    /// The service manager.
    pub units: U,
    store: Store,
    /// `daemon.store` as the file wrote it at start: the store above, handed every runtime.
    url: String,
    config: Config,
    text: String,
    started: BTreeMap<String, Started>,
    running: BTreeMap<String, bool>,
    last_pass: Option<Instant>,
}

impl<U: Units> Daemon<U> {
    /// The daemon of `config_file` with its store and [`RUNTIMES_FILE`] under `dir`.
    ///
    /// # Errors
    ///
    /// The config file cannot be read or is not valid, the store does not open, or
    /// [`RUNTIMES_FILE`] does not read.
    pub fn open(
        config_file: PathBuf,
        home: PathBuf,
        dir: PathBuf,
        binary: Binary,
        units: U,
    ) -> Result<Self> {
        let (text, config) = read_config(&config_file, &home)?;
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let url = written_store(&config_file, &text)?;
        if let Ok(crate::store::Location::Sqlite(path)) = crate::store::Location::parse(&url)
            && let Some(parent) = path.parent()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("daemon.store: create {}", parent.display()))?;
        }
        let store = crate::store::open_url(&url, super::TENANT)
            .map_err(|error| anyhow!("daemon.store: {error:#}"))?;
        let file = dir.join(RUNTIMES_FILE);
        let started = match fs::read_to_string(&file) {
            Ok(text) => {
                serde_json::from_str(&text).with_context(|| format!("read {}", file.display()))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error).with_context(|| format!("read {}", file.display())),
        };
        Ok(Self {
            config_file,
            home,
            dir,
            binary,
            units,
            store,
            url,
            config,
            text,
            started,
            running: BTreeMap::new(),
            last_pass: None,
        })
    }

    /// The config as the last pass read it.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The store the reconciliations are recorded in.
    #[must_use]
    pub const fn store(&self) -> &Store {
        &self.store
    }

    /// What the daemon started, by instance.
    #[must_use]
    pub const fn started(&self) -> &BTreeMap<String, Started> {
        &self.started
    }

    /// `daemon.cadence.reconcile`, 60 s when it does not read as seconds.
    #[must_use]
    pub fn cadence(&self) -> Duration {
        Duration::from_secs(
            crate::config::seconds(&self.config.daemon.cadence.reconcile)
                .unwrap_or(crate::config::RECONCILE_EVERY),
        )
    }

    /// Why a pass is due at `now`, if one is: the first tick; the config file's text changed;
    /// a runtime the daemon started stopped running; or a cadence passed since the last pass.
    /// It notes, for [`Daemon::rows`], which runtimes run.
    ///
    /// # Errors
    ///
    /// The service manager cannot be asked.
    pub fn due(&mut self, now: Instant) -> Result<Option<ReconcileReason>> {
        let lost = self.observe()?;
        let Some(last) = self.last_pass else {
            return Ok(Some(ReconcileReason::Cadence));
        };
        if fs::read_to_string(&self.config_file).ok().as_deref() != Some(self.text.as_str()) {
            return Ok(Some(ReconcileReason::ConfigChange));
        }
        if lost {
            return Ok(Some(ReconcileReason::LostRuntime));
        }
        Ok((now.saturating_duration_since(last) >= self.cadence())
            .then_some(ReconcileReason::Cadence))
    }

    /// [`Daemon::due`], then the pass it calls for.
    ///
    /// # Errors
    ///
    /// What [`Daemon::due`] or [`Daemon::pass`] answers.
    pub fn tick(&mut self, now: Instant) -> Result<Vec<Done>> {
        match self.due(now)? {
            Some(reason) => self.pass(reason, now),
            None => Ok(Vec::new()),
        }
    }

    /// Which started runtimes run, and whether one that ran at the last look no longer does.
    fn observe(&mut self) -> Result<bool> {
        let at = crate::records::now()?;
        let mut lost = false;
        let names: Vec<String> = self.started.keys().cloned().collect();
        for name in names {
            let unit = self.started[&name].unit.clone();
            let running = self.units.active(&unit)?;
            if running {
                if let Some(started) = self.started.get_mut(&name) {
                    started.last_seen = Some(at.0.clone());
                }
            } else {
                lost |= self.running.get(&name).copied().unwrap_or(true);
            }
            self.running.insert(name, running);
        }
        Ok(lost)
    }

    /// One pass for `reason` at `now`, as the module documentation says; answers the actions it
    /// took, each recorded.
    ///
    /// # Errors
    ///
    /// The config file does not read or is not valid (nothing is done), the service manager
    /// cannot be asked, a reconciliation cannot be recorded, or [`RUNTIMES_FILE`] cannot be
    /// written.
    pub fn pass(&mut self, reason: ReconcileReason, now: Instant) -> Result<Vec<Done>> {
        let (text, config) = read_config(&self.config_file, &self.home)?;
        self.text = text;
        self.config = config;
        self.last_pass = Some(now);
        let at = crate::records::now()?;
        let mut done = Vec::new();
        let instances = self.config.instances.clone();
        for instance in &instances {
            let name = instance.name.0.clone();
            let wanted = Started {
                unit: unit_name(&name),
                placement: instance.placement.0.clone(),
                binary: self.binary.path.display().to_string(),
                version: self.binary.version.clone(),
                last_seen: Some(at.0.clone()),
            };
            let held = self.started.get(&name).cloned();
            let running = match &held {
                Some(held) => self.units.active(&held.unit)?,
                None => false,
            };
            let (action, why, outcome) = match held {
                Some(held) if running => {
                    let same = held.placement == wanted.placement
                        && held.binary == wanted.binary
                        && held.version == wanted.version;
                    if same {
                        if let Some(kept) = self.started.get_mut(&name) {
                            kept.last_seen = wanted.last_seen.clone();
                        }
                        self.running.insert(name, true);
                        continue;
                    }
                    let service = self.service(instance);
                    let outcome = self
                        .units
                        .stop(&held.unit)
                        .and_then(|()| self.units.start(&service));
                    (ReconcileAction::Restart, reason, outcome)
                }
                held => {
                    let why = if held.is_some() {
                        ReconcileReason::LostRuntime
                    } else {
                        reason
                    };
                    let service = self.service(instance);
                    (ReconcileAction::Start, why, self.units.start(&service))
                }
            };
            let succeeded = outcome.is_ok();
            if succeeded {
                self.started.insert(name.clone(), wanted);
            } else {
                self.started.remove(&name);
            }
            self.running.insert(name.clone(), succeeded);
            done.push(self.record(&name, action, why, outcome, &at)?);
        }
        let gone: Vec<String> = self
            .started
            .keys()
            .filter(|name| !instances.iter().any(|instance| &instance.name.0 == *name))
            .cloned()
            .collect();
        for name in gone {
            self.running.remove(&name);
            let Some(held) = self.started.remove(&name) else {
                continue;
            };
            if self.units.active(&held.unit)? {
                let outcome = self.units.stop(&held.unit);
                done.push(self.record(&name, ReconcileAction::Stop, reason, outcome, &at)?);
            }
        }
        self.save()?;
        Ok(done)
    }

    /// The unit `instance`'s runtime is started as: `<binary> --config <file> instance serve` in
    /// the instance's slice and state directory, with the instance, its config file and the
    /// deployment's store URL in its environment, restarted 30 s after a failure.
    fn service(&self, instance: &Instance) -> Service {
        let name = &instance.name.0;
        let file = self.config_file.display().to_string();
        let mut env = vec![
            format!("{}={file}", crate::config::CONFIG_VARIABLE),
            format!("{}={name}", crate::config::INSTANCE_VARIABLE),
            format!("{}={}", crate::store::STORE_VARIABLE, self.url),
        ];
        if let Some(path) = std::env::var_os("PATH") {
            env.push(format!("PATH={}", path.to_string_lossy()));
        }
        // The runtime starts in its state directory, which a first start creates; a directory
        // that cannot be made fails the start, which names it.
        let _ = fs::create_dir_all(&instance.state);
        Service {
            unit: unit_name(name),
            slice: systemd::slice_name(
                instance
                    .session_prefix
                    .as_ref()
                    .map(|prefix| prefix.0.as_str()),
            ),
            limits: instance.limits.clone(),
            cwd: PathBuf::from(&instance.state),
            properties: [
                "Restart=on-failure".to_owned(),
                "RestartSec=30".to_owned(),
                format!("Description=conductor instance {name}"),
            ]
            .to_vec(),
            env,
            command: vec![
                self.binary.path.display().to_string(),
                "--config".to_owned(),
                file,
                "instance".to_owned(),
                "serve".to_owned(),
            ],
        }
    }

    /// Records one `Reconciliation` of `action` on `instance`, as the daemon.
    fn record(
        &mut self,
        instance: &str,
        action: ReconcileAction,
        reason: ReconcileReason,
        outcome: Result<()>,
        at: &conductor_model::primitives::Timestamp,
    ) -> Result<Done> {
        let (result, detail) = match outcome {
            Ok(()) => (ReconcileResult::Succeeded, None),
            Err(error) => (ReconcileResult::Refused, Some(format!("{error:#}"))),
        };
        let data = ReconciliationData {
            reconciliation_id: ReconciliationId(Uuid(eventlog_core::new_event_id())),
            instance: InstanceName(instance.to_owned()),
            action,
            reason,
            result,
            detail: detail.clone(),
            actor: crate::actor::daemon(),
            at: at.clone(),
        };
        ReconciliationStorage::put(
            &mut self.store,
            AnyReconciliation::Recorded(Reconciliation::new(data)).snapshot(),
        );
        self.store
            .check()
            .map_err(|error| anyhow!("record the reconciliation of {instance}: {error}"))?;
        Ok(Done {
            instance: instance.to_owned(),
            action,
            reason,
            result,
            detail,
        })
    }

    fn save(&self) -> Result<()> {
        let file = self.dir.join(RUNTIMES_FILE);
        let next = self.dir.join(format!("{RUNTIMES_FILE}.next"));
        let text = serde_json::to_string_pretty(&self.started)?;
        fs::write(&next, text + "\n").with_context(|| format!("write {}", next.display()))?;
        fs::rename(&next, &file).with_context(|| format!("write {}", file.display()))
    }

    /// The rows of `GET /v1/instances` (`conductor.instance.Instances`): every instance of the
    /// config with its placement and roles, and from what runs its state (`Running`, `Stopped`
    /// when the daemon started none, `Unreachable` when the one it started stopped), the
    /// version of the binary its runtime runs, and when the daemon last saw it running.
    #[must_use]
    pub fn rows(&self) -> Vec<Value> {
        self.config
            .instances
            .iter()
            .map(|instance| {
                let name = &instance.name.0;
                let started = self.started.get(name);
                let state = match (started, self.running.get(name)) {
                    (None, _) => InstanceState::Stopped,
                    (Some(_), Some(false)) => InstanceState::Unreachable,
                    (Some(_), _) => InstanceState::Running,
                };
                json!({
                    "name": name,
                    "state": state_name(state),
                    "placement": instance.placement.0,
                    "roles": instance.roles.iter().map(|role| role.role.clone()).collect::<Vec<_>>(),
                    "binary_version": started.map(|started| started.version.clone()),
                    "last_seen": started.and_then(|started| started.last_seen.clone()),
                    "box_id": Value::Null,
                    "changed_by": crate::actor::to_json(&crate::actor::daemon()),
                })
            })
            .collect()
    }
}

/// The state's name as the specification spells it.
#[must_use]
pub const fn state_name(state: InstanceState) -> &'static str {
    match state {
        InstanceState::Purged => "Purged",
        InstanceState::Running => "Running",
        InstanceState::Starting => "Starting",
        InstanceState::Stopped => "Stopped",
        InstanceState::Unreachable => "Unreachable",
        InstanceState::Upgrading => "Upgrading",
    }
}

/// The text of `file` and the config it holds, `~` read as `home`.
/// The `daemon.store` the config file's `text` writes (`story:store-shared`): the daemon starts
/// only on a URL the file names, never on the default. A postgres password written as
/// `secret:<name>` is refused: no secrets source resolves it yet.
fn written_store(file: &Path, text: &str) -> Result<String> {
    let written = serde_yaml::from_str::<serde_yaml::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("daemon")?
                .get("store")?
                .as_str()
                .map(str::to_owned)
        })
        .filter(|url| !url.is_empty());
    let Some(url) = written else {
        return Err(anyhow!(
            "the config file {} writes no daemon.store: write sqlite:<path> or postgres://…",
            file.display()
        ));
    };
    if url.contains(":secret:") {
        return Err(anyhow!(
            "daemon.store writes its password as secret:<name>, which the daemon does not \
             resolve yet"
        ));
    }
    Ok(url)
}

fn read_config(file: &Path, home: &Path) -> Result<(String, Config)> {
    let text = fs::read_to_string(file)
        .with_context(|| format!("read the config file {}", file.display()))?;
    let config = crate::config::parse(&text, home).map_err(|problems| {
        let lines: Vec<String> = problems
            .iter()
            .map(|problem| format!("{}: {problem}", file.display()))
            .collect();
        anyhow!("{}", lines.join("\n"))
    })?;
    Ok((text, config))
}
