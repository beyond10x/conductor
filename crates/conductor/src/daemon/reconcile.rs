//! The reconciler of conductord (`story:daemon-reconcile`, design architecture-next § 4): the
//! config file is the desired set of instances, the runtimes the daemon started and that still
//! run are the actual set, and one pass makes the second equal the first.
//!
//! A pass ([`Daemon::pass`]) reads the config file and, for each instance:
//! - starts its runtime (`conductor instance serve`, the unit [`unit_name`]) when none runs:
//!   never started, or started and no longer running (recorded as `LostRuntime`);
//! - restarts it when it runs on another placement than the config says, or another binary (path
//!   or `--version`) than the instance's desired binary ([`Pinned`], `story:instance-upgrade`):
//!   the daemon's own binary at the instance's first start, as a copy in its `binaries/<version>/`
//!   so an install over the daemon's path changes no runtime's binary, then what an upgrade or a
//!   rollback put there ([`super::upgrade`]);
//! - rolls an upgrading instance whose runtime stopped running back to its previous binary
//!   ([`Daemon::settle`]) rather than starting it again on the new one;
//! - leaves it alone when it matches, and records nothing.
//!
//! and stops each runtime it started whose instance the config no longer names; then it keeps the
//! host roles `globals.roles` lists running and stops one it no longer lists ([`host_roles`],
//! `story:host-roles-by-daemon`). Each instance action is
//! one `conductor.instance.Reconciliation` in the daemon's store; a start or stop the service
//! manager refuses is recorded `Refused`, with what it said as the detail. What the daemon
//! started is kept beside its store in [`RUNTIMES_FILE`], so a restarted daemon finds the
//! runtimes it started running and does nothing; each instance's desired and previous binary,
//! and the upgrade under way, in [`BINARIES_FILE`].
//!
//! A pass runs ([`Daemon::due`]) at the first tick, when the config file's text changed
//! (`ConfigChange`), when a runtime it started stopped running (`LostRuntime`), and every
//! `daemon.cadence.reconcile` (`Cadence`).

use std::collections::{BTreeMap, BTreeSet};
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

use super::api::Route;
use super::host_roles::{self, HostStart};
use super::units::Units;
use crate::store::Store;
use crate::systemd::{self, Service};

/// What the daemon started, beside its store: one [`Started`] per instance, by name.
pub const RUNTIMES_FILE: &str = "runtimes.json";

/// Each instance's binaries, beside [`RUNTIMES_FILE`]: one [`Pinned`] per instance, by name.
pub const BINARIES_FILE: &str = "binaries.json";

/// The box of each instance the daemon started in a substrate box, beside [`RUNTIMES_FILE`]: one
/// [`crate::placement::OpenedBox`] per instance, by name (`story:placement-substrate-local`).
pub const BOXES_FILE: &str = "boxes.json";

/// The unit [`Started`] names for a runtime in a substrate box: `substrate:<session>`.
#[must_use]
pub fn boxed_unit(opened: &crate::placement::OpenedBox) -> String {
    format!("substrate:{}", opened.session)
}

/// The substrate daemon a box is on: `local`, or the box opener `remote` makes for the remote
/// daemon `at`, its bearer from the secrets source `secrets`
/// ([`crate::placement::bearer_from`]).
fn substrate_of<'a>(
    local: &'a mut (dyn crate::placement::Substrate + Send),
    remote: &crate::placement::RemoteSubstrate,
    secrets: &conductor_model::config::SecretsSource,
    at: Option<&crate::placement::RemoteAt>,
) -> Box<dyn crate::placement::Substrate + Send + 'a> {
    match at {
        Some(at) => remote(at, crate::placement::bearer_from(secrets, at)),
        None => Box::new(local),
    }
}

/// The unit an instance's runtime runs in: `conductor_<instance>-runtime.service`, escaped as a
/// session's unit is ([`systemd::unit_name`]).
#[must_use]
pub fn unit_name(instance: &str) -> String {
    systemd::unit_name(&SessionName(format!("{instance}-runtime")))
}

/// The conductor binary a runtime runs: its path and what its `--version` printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The token the daemon minted for the runtime at its start ([`crate::instance::TOKEN_VARIABLE`]):
    /// a write the daemon proxies to it carries it (`story:instance-api`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// One instance's binaries, as [`BINARIES_FILE`] keeps them (`story:instance-upgrade`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pinned {
    /// The binary its runtime is to run.
    pub desired: Binary,
    /// The binary it ran before the last upgrade, which a rollback puts back.
    pub previous: Option<Binary>,
    /// The upgrade under way, until its runtime answers or it is rolled back.
    pub upgrading: Option<Upgrading>,
}

/// An upgrade under way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrading {
    /// The version upgraded to.
    pub version: String,
    /// When it rolls back unless its runtime answered: seconds since the Unix epoch.
    pub deadline: u64,
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
    /// The binary a runtime is to run at its instance's first start; after that, the instance's
    /// own desired binary ([`Pinned`]).
    pub binary: Binary,
    /// The service manager.
    pub units: U,
    /// What fetches a release and runs its doctor for an upgrade ([`super::upgrade`]).
    pub releases: Box<dyn super::upgrade::Releases>,
    /// The substrate daemon an instance whose placement is a substrate box runs in
    /// ([`crate::placement`]); the local daemon through the SDK by default.
    pub substrate: Box<dyn crate::placement::Substrate + Send>,
    /// What opens a box opener for a remote substrate daemon, with its bearer
    /// (`story:placement-substrate-remote`); the SDK's remote transport by default.
    pub remote: crate::placement::RemoteSubstrate,
    /// Where a box's model token comes from: the daemon's secrets source by default
    /// ([`crate::placement::token_source`]).
    pub tokens: crate::placement::TokenSource,
    pub(super) boxes: BTreeMap<String, crate::placement::OpenedBox>,
    store: Store,
    /// `daemon.store` as the file wrote it at start: the store above, handed every runtime.
    url: String,
    config: Config,
    text: String,
    pub(super) started: BTreeMap<String, Started>,
    pub(super) running: BTreeMap<String, bool>,
    pub(super) pins: BTreeMap<String, Pinned>,
    /// Each host role the daemon started, by role, and whether its unit ran at the last look
    /// (`story:host-roles-by-daemon`).
    hosts: BTreeMap<String, bool>,
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
        let started = read_kept(&dir.join(RUNTIMES_FILE))?;
        let pins = read_kept(&dir.join(BINARIES_FILE))?;
        let boxes = read_kept(&dir.join(BOXES_FILE))?;
        Ok(Self {
            config_file,
            home,
            dir,
            binary,
            units,
            releases: Box::new(super::upgrade::Github),
            substrate: Box::new(crate::placement::Sdk::default()),
            remote: crate::placement::sdk_remote(),
            tokens: crate::placement::token_source(),
            boxes,
            store,
            url,
            config,
            text,
            started,
            running: BTreeMap::new(),
            pins,
            hosts: BTreeMap::new(),
            last_pass: None,
        })
    }

    /// The binary `name`'s runtime is to run: its desired one, else the daemon's own
    /// ([`Self::own_copy`]).
    fn desired(&self, name: &str) -> Binary {
        self.pins
            .get(name)
            .map_or_else(|| self.own_copy(), |pinned| pinned.desired.clone())
    }

    /// The daemon's own binary as an instance's first start pins it: a copy in
    /// `<dir>/binaries/<version>/conductor` ([`super::upgrade::BINARIES_DIR`]), made once, so an
    /// install over the daemon's own path never changes a runtime's pinned or rollback binary.
    /// The daemon's path itself when its version is no x.y.z or the copy cannot be made.
    fn own_copy(&self) -> Binary {
        if crate::upgrade::parse_version(&self.binary.version).is_none() {
            return self.binary.clone();
        }
        let dir = self
            .dir
            .join(super::upgrade::BINARIES_DIR)
            .join(&self.binary.version);
        let path = dir.join("conductor");
        let copied = if path.is_file() {
            Ok(path)
        } else {
            super::upgrade::install(&self.binary.path, &dir)
        };
        match copied {
            Ok(path) => Binary {
                path,
                version: self.binary.version.clone(),
            },
            Err(_) => self.binary.clone(),
        }
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

    /// [`Daemon::due`], then the pass it calls for, then [`Daemon::keep_boxes`].
    ///
    /// # Errors
    ///
    /// What [`Daemon::due`] or [`Daemon::pass`] answers.
    pub fn tick(&mut self, now: Instant) -> Result<Vec<Done>> {
        let mut done = match self.due(now)? {
            Some(reason) => self.pass(reason, now)?,
            None => Vec::new(),
        };
        done.extend(self.keep_boxes(std::time::SystemTime::now())?);
        Ok(done)
    }

    /// Keeps each box the daemon opened past substrate's bounds at `now`.
    ///
    /// # Errors
    ///
    /// A reconciliation cannot be recorded.
    /// A box's leases are renewed every [`crate::placement::RENEW_EVERY`], each by
    /// [`crate::placement::RUN_BOUND`]; its runtime, whose run substrate ends at its hard
    /// deadline, is stopped and started anew in the same box
    /// [`crate::placement::RESTART_BEFORE`] that deadline, recorded as a `Restart` for
    /// `RunBound`, with substrate's name when it refuses. A silent daemon is asked again at the
    /// next tick.
    pub fn keep_boxes(&mut self, now: std::time::SystemTime) -> Result<Vec<Done>> {
        let now = epoch(now);
        let renew_every = crate::placement::RENEW_EVERY.as_secs();
        let restart_at = (crate::placement::RUN_BOUND - crate::placement::RESTART_BEFORE).as_secs();
        let mut done = Vec::new();
        let mut changed = false;
        let names: Vec<String> = self.boxes.keys().cloned().collect();
        for name in names {
            let Some(mut opened) = self.boxes.get(&name).cloned() else {
                continue;
            };
            if opened.started_at == 0 {
                opened.started_at = now;
            }
            if now.saturating_sub(opened.started_at) >= restart_at {
                let instance = self
                    .config
                    .instances
                    .iter()
                    .find(|instance| instance.name.0 == name)
                    .cloned();
                if let Some(instance) = instance {
                    done.push(self.restart_in_box(&instance, &mut opened, now)?);
                }
            } else if now.saturating_sub(opened.renewed_at) >= renew_every {
                let renewed = substrate_of(
                    self.substrate.as_mut(),
                    &self.remote,
                    &self.config.daemon.secrets,
                    opened.remote.as_ref(),
                )
                .renew(
                    &opened.workspace,
                    &opened.session,
                    crate::placement::RUN_BOUND,
                );
                if renewed.is_ok() {
                    opened.renewed_at = now;
                }
            }
            if self.boxes.get(&name) != Some(&opened) {
                changed = true;
                self.boxes.insert(name, opened);
            }
        }
        if changed {
            self.save()?;
        }
        Ok(done)
    }

    /// Stops the runtime of `instance` in its box `opened` and starts it anew there, at `now`.
    fn restart_in_box(
        &mut self,
        instance: &Instance,
        opened: &mut crate::placement::OpenedBox,
        now: u64,
    ) -> Result<Done> {
        let name = instance.name.0.clone();
        let desired = self.desired(&name);
        let token = crate::dashboard::mint().context("mint the runtime's token")?;
        let service = self.service(instance, &desired, &token);
        let mut request = self.box_request(instance, &service);
        if opened.remote.is_none() {
            request.secret_slots = opened.slot.iter().cloned().collect();
        }
        let mut substrate = substrate_of(
            self.substrate.as_mut(),
            &self.remote,
            &self.config.daemon.secrets,
            opened.remote.as_ref(),
        );
        let restarted = substrate
            .stop(&opened.session)
            .map_err(|error| crate::placement::Refused {
                name: crate::placement::UNREACHABLE.to_owned(),
                detail: format!("{error:#}"),
            })
            .and_then(|()| substrate.restart(&opened.workspace, &request));
        drop(substrate);
        let outcome = match restarted {
            Ok(session) => {
                opened.session = session;
                opened.started_at = now;
                opened.renewed_at = now;
                let unit = boxed_unit(opened);
                if let Some(started) = self.started.get_mut(&name) {
                    started.unit = unit;
                    started.token = Some(token);
                }
                self.running.insert(name.clone(), true);
                Ok(())
            }
            Err(refused) => Err(anyhow!(
                "the runtime of {name} did not restart in its box {}: {}: {}",
                opened.workspace,
                refused.name,
                refused.detail
            )),
        };
        let at = crate::records::now()?;
        self.record(
            &name,
            ReconcileAction::Restart,
            ReconcileReason::RunBound,
            outcome,
            &at,
        )
    }

    /// Which started runtimes run, and whether one that ran at the last look no longer does.
    fn observe(&mut self) -> Result<bool> {
        let at = crate::records::now()?;
        let mut lost = false;
        let names: Vec<String> = self.started.keys().cloned().collect();
        for name in names {
            let unit = self.started[&name].unit.clone();
            // A silent remote daemon: `Unreachable`, and no lost runtime to start again.
            let Some(running) = self.look(&name, &unit)? else {
                self.running.insert(name, false);
                continue;
            };
            if running {
                if let Some(started) = self.started.get_mut(&name) {
                    started.last_seen = Some(at.0.clone());
                }
            } else {
                lost |= self.running.get(&name).copied().unwrap_or(true);
            }
            self.running.insert(name, running);
        }
        let roles: Vec<String> = self.hosts.keys().cloned().collect();
        for role in roles {
            let running = self.units.active(&host_roles::unit(&role))?;
            lost |= !running && self.hosts.get(&role).copied().unwrap_or(false);
            self.hosts.insert(role, running);
        }
        Ok(lost)
    }

    /// Each host role the daemon started, by role, and whether its unit ran at the last look.
    #[must_use]
    pub const fn host_roles(&self) -> &BTreeMap<String, bool> {
        &self.hosts
    }

    /// The supervisor's session as `POST /v1/ask` reaches it, while its unit runs.
    #[must_use]
    pub fn supervisor(&self) -> Option<host_roles::Supervisor> {
        (self.hosts.get(host_roles::SUPERVISOR) == Some(&true)).then(|| host_roles::Supervisor {
            state: host_roles::state_dir(&self.dir),
            name: host_roles::SUPERVISOR.to_owned(),
        })
    }

    /// The [`HostStart`] of `role`: `session start --role <role>` with the daemon's binary under
    /// the host roles' state directory, beside the config file, at [`host_roles::PLACEMENT`]; the
    /// instance it starts under is the file's `default`, else its first.
    fn host_start(&self, role: &str) -> HostStart {
        let file = self.config_file.display().to_string();
        let state = host_roles::state_dir(&self.dir);
        let mut env = vec![format!("{}={file}", crate::config::CONFIG_VARIABLE)];
        let instance = self
            .config
            .default
            .as_ref()
            .map(|name| name.0.clone())
            .or_else(|| {
                self.config
                    .instances
                    .first()
                    .map(|instance| instance.name.0.clone())
            });
        if let Some(instance) = instance {
            env.push(format!("{}={instance}", crate::config::INSTANCE_VARIABLE));
        }
        if let Some(path) = std::env::var_os("PATH") {
            env.push(format!("PATH={}", path.to_string_lossy()));
        }
        HostStart {
            role: role.to_owned(),
            placement: host_roles::PLACEMENT.to_owned(),
            unit: host_roles::unit(role),
            cwd: self
                .config_file
                .parent()
                .map_or_else(|| self.home.clone(), Path::to_owned),
            env,
            command: vec![
                self.binary.path.display().to_string(),
                "--config".to_owned(),
                file,
                "--state-dir".to_owned(),
                state.display().to_string(),
                "session".to_owned(),
                "start".to_owned(),
                "--role".to_owned(),
                role.to_owned(),
            ],
        }
    }

    /// Starts the host role `role` when its unit does not run; answers what was done.
    fn start_host(&mut self, role: &str, reason: ReconcileReason) -> Result<Option<Done>> {
        let unit = host_roles::unit(role);
        if self.units.active(&unit)? {
            self.hosts.insert(role.to_owned(), true);
            return Ok(None);
        }
        let start = self.host_start(role);
        let outcome = self.units.start_host_role(&start);
        let started = outcome.is_ok();
        self.hosts.insert(role.to_owned(), started);
        Ok(Some(host_done(
            role,
            ReconcileAction::Start,
            reason,
            outcome,
        )))
    }

    /// The host-role step of a pass (`story:host-roles-by-daemon`): starts each host role
    /// `globals.roles` lists whose unit does not run, and stops the running unit of a host role
    /// it does not list.
    fn keep_host_roles(&mut self, reason: ReconcileReason) -> Result<Vec<Done>> {
        let listed = host_roles::listed(&self.config);
        let mut done = Vec::new();
        for role in &listed {
            done.extend(self.start_host(role, reason)?);
        }
        for role in host_roles::HOST_ROLES {
            if listed.iter().any(|named| named == role) {
                continue;
            }
            self.hosts.remove(role);
            let unit = host_roles::unit(role);
            if self.units.active(&unit)? {
                let outcome = self.units.stop(&unit);
                done.push(host_done(role, ReconcileAction::Stop, reason, outcome));
            }
        }
        Ok(done)
    }

    /// Starts each host role of `roles` now, asked for through `POST /v1/roles/{role}/start`:
    /// one `globals.roles` lists and whose unit does not run is started; one it does not list is
    /// answered `Refused`, naming `globals.roles`.
    ///
    /// # Errors
    ///
    /// The service manager cannot be asked.
    pub fn start_asked(&mut self, roles: &BTreeSet<String>) -> Result<Vec<Done>> {
        let mut done = Vec::new();
        for role in roles {
            match host_roles::global_role(&self.config, role) {
                Ok(_) => done.extend(self.start_host(role, ReconcileReason::Cadence)?),
                Err(why) => done.push(host_done(
                    role,
                    ReconcileAction::Start,
                    ReconcileReason::Cadence,
                    Err(anyhow!(why)),
                )),
            }
        }
        Ok(done)
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
        self.pool()
            .prepare()
            .context("daemon.substrate_slots: prepare the slot files")?;
        let mut done = self.settle(std::time::SystemTime::now())?;
        let instances = self.config.instances.clone();
        for instance in &instances {
            let name = instance.name.0.clone();
            let desired = self.desired(&name);
            let mut wanted = Started {
                unit: unit_name(&name),
                placement: instance.placement.0.clone(),
                binary: desired.path.display().to_string(),
                version: desired.version.clone(),
                last_seen: Some(at.0.clone()),
                token: None,
            };
            let held = self.started.get(&name).cloned();
            let running = match &held {
                Some(held) => match self.look(&name, &held.unit)? {
                    Some(running) => running,
                    // A silent remote daemon: its box may run on; nothing starts in its place.
                    None => {
                        self.running.insert(name, false);
                        continue;
                    }
                },
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
                    let token = crate::dashboard::mint().context("mint the runtime's token")?;
                    let service = self.service(instance, &desired, &token);
                    wanted.token = Some(token);
                    let outcome = self
                        .stop_runtime(&name, &held.unit)
                        .and_then(|()| self.start_runtime(instance, &service));
                    (ReconcileAction::Restart, reason, outcome)
                }
                held => {
                    let why = if held.is_some() {
                        ReconcileReason::LostRuntime
                    } else {
                        reason
                    };
                    let token = crate::dashboard::mint().context("mint the runtime's token")?;
                    let service = self.service(instance, &desired, &token);
                    wanted.token = Some(token);
                    // A runtime whose session substrate ended (its run bound, its output bound)
                    // leaves its box open: closed, and its slot freed, before a new one opens.
                    let outcome = match held {
                        Some(held)
                            if self
                                .boxes
                                .get(&name)
                                .is_some_and(|opened| boxed_unit(opened) == held.unit) =>
                        {
                            self.stop_runtime(&name, &held.unit)
                                .and_then(|()| self.start_runtime(instance, &service))
                        }
                        _ => self.start_runtime(instance, &service),
                    };
                    (ReconcileAction::Start, why, outcome)
                }
            };
            let outcome = outcome.map(|unit| {
                wanted.unit = unit;
            });
            let succeeded = outcome.is_ok();
            if succeeded {
                self.started.insert(name.clone(), wanted);
                self.pins.entry(name.clone()).or_insert_with(|| Pinned {
                    desired,
                    previous: None,
                    upgrading: None,
                });
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
        self.pins
            .retain(|name, _| instances.iter().any(|instance| &instance.name.0 == name));
        for name in gone {
            self.running.remove(&name);
            let Some(held) = self.started.remove(&name) else {
                continue;
            };
            if self.runs(&name, &held.unit)? {
                let outcome = self.stop_runtime(&name, &held.unit);
                done.push(self.record(&name, ReconcileAction::Stop, reason, outcome, &at)?);
            }
        }
        done.extend(self.keep_host_roles(reason)?);
        self.save()?;
        Ok(done)
    }

    /// Whether the runtime of `name`, started as `unit`, runs: its session in its substrate box
    /// when it was started in one ([`boxed_unit`]), else its unit. A silent remote daemon counts
    /// as running here, so a box on it is closed, not forgotten ([`Daemon::look`]).
    fn runs(&mut self, name: &str, unit: &str) -> Result<bool> {
        Ok(self.look(name, unit)?.unwrap_or(true))
    }

    /// What [`Daemon::runs`] answers, and `None` when the runtime is in a box on a remote daemon
    /// that does not answer (`story:placement-substrate-remote`): its instance is `Unreachable`,
    /// a row and never an error, and nothing is started in its place.
    fn look(&mut self, name: &str, unit: &str) -> Result<Option<bool>> {
        match self.boxes.get(name).cloned() {
            Some(opened) if boxed_unit(&opened) == unit => {
                let remote = opened.remote.is_some();
                match substrate_of(
                    self.substrate.as_mut(),
                    &self.remote,
                    &self.config.daemon.secrets,
                    opened.remote.as_ref(),
                )
                .running(&opened.session)
                {
                    Ok(running) => Ok(Some(running)),
                    Err(_) if remote => Ok(None),
                    Err(error) => Err(error),
                }
            }
            _ => self.units.active(unit).map(Some),
        }
    }

    /// Starts `instance`'s runtime as `service` says and answers the unit it runs as. An instance
    /// whose placement is a substrate box ([`crate::placement::boxed`]) runs in a box opened for
    /// it ([`crate::placement::open_box`]), on the remote daemon its placement names when it
    /// names one, never through the service manager; a refusal is `PlacementRefused` with
    /// substrate's name, and nothing falls back. The runtime's token is not handed into the box:
    /// substrate seals only slots its daemon declares, and its model token goes through the
    /// slot pool ([`crate::placement::SlotPool`]).
    fn start_runtime(&mut self, instance: &Instance, service: &Service) -> Result<String> {
        if !crate::placement::boxed(instance) {
            self.units.start(service)?;
            return Ok(service.unit.clone());
        }
        let mut request = self.box_request(instance, service);
        let remote = instance
            .remote
            .as_ref()
            .map(crate::placement::RemoteAt::from);
        // A local box's model token goes through a free slot of the pool; a box without one
        // receives no slot. Conductor cannot make the request: refused before substrate is asked.
        let mut slot = None;
        let mut refusal = None;
        if remote.is_none() {
            request.secret_slots.clear();
            if let Some(name) = &instance.model_token {
                match (self.tokens)(&self.config.daemon.secrets, name) {
                    Some(value) => match self.pool().acquire(&self.held_slots(), &value) {
                        Ok(acquired) => {
                            request.secret_slots.push(acquired.clone());
                            slot = Some(acquired);
                        }
                        Err(refused) => refusal = Some(refused),
                    },
                    None => {
                        refusal = Some(crate::placement::Refused {
                            name: crate::placement::MODEL_TOKEN_UNAVAILABLE.to_owned(),
                            detail: format!(
                                "the secrets source holds no secret {} for model_token",
                                name.0
                            ),
                        });
                    }
                }
            }
        }
        let mut refusing;
        let mut substrate = match refusal {
            Some(refused) => {
                refusing = crate::placement::Refusing(refused);
                Box::new(&mut refusing) as Box<dyn crate::placement::Substrate + Send + '_>
            }
            None => substrate_of(
                self.substrate.as_mut(),
                &self.remote,
                &self.config.daemon.secrets,
                remote.as_ref(),
            ),
        };
        let opened = crate::placement::open_box(&mut self.store, substrate.as_mut(), &request);
        drop(substrate);
        let mut opened = match opened {
            Ok(opened) => opened,
            Err(refused) => {
                if let Some(slot) = &slot {
                    let _ = self.pool().free(slot);
                }
                return Err(refused.into());
            }
        };
        let now = epoch(std::time::SystemTime::now());
        opened.remote = remote;
        opened.slot = slot;
        opened.started_at = now;
        opened.renewed_at = now;
        let unit = boxed_unit(&opened);
        self.boxes.insert(instance.name.0.clone(), opened);
        Ok(unit)
    }

    /// The box `instance` asks for, its runtime started as `service` says, with no credential in
    /// its environment: substrate seals only slots its daemon declares.
    fn box_request(&self, instance: &Instance, service: &Service) -> crate::placement::BoxRequest {
        let token = format!("{}=", crate::instance::TOKEN_VARIABLE);
        let env = service
            .env
            .iter()
            .filter(|pair| !pair.starts_with(&token))
            .filter_map(|pair| pair.split_once('='))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect();
        let binary = PathBuf::from(service.command.first().cloned().unwrap_or_default());
        crate::placement::request(instance, &self.config_file, &binary, env)
    }

    /// The local substrate daemon's model-token slot pool (`daemon.substrate_slots`), under the
    /// daemon's directory.
    fn pool(&self) -> crate::placement::SlotPool {
        crate::placement::SlotPool {
            dir: self.dir.join(crate::placement::SLOTS_DIR),
            size: usize::try_from(self.config.daemon.substrate_slots).unwrap_or(0),
        }
    }

    /// The slots the open boxes hold.
    fn held_slots(&self) -> BTreeSet<String> {
        self.boxes
            .values()
            .filter_map(|opened| opened.slot.clone())
            .collect()
    }

    /// Stops the runtime of `name`, started as `unit`: closes its box when it runs in one, on the
    /// daemon it was opened on ([`crate::placement::close_box`]), else stops its unit.
    fn stop_runtime(&mut self, name: &str, unit: &str) -> Result<()> {
        match self.boxes.get(name).cloned() {
            Some(opened) if boxed_unit(&opened) == unit => {
                let mut substrate = substrate_of(
                    self.substrate.as_mut(),
                    &self.remote,
                    &self.config.daemon.secrets,
                    opened.remote.as_ref(),
                );
                let closed =
                    crate::placement::close_box(&mut self.store, substrate.as_mut(), &opened);
                drop(substrate);
                closed?;
                self.boxes.remove(name);
                if let Some(slot) = &opened.slot {
                    self.pool().free(slot)?;
                }
                Ok(())
            }
            _ => self.units.stop(unit),
        }
    }

    /// The unit `instance`'s runtime is started as: `<binary> --config <file> instance serve` in
    /// the instance's slice and state directory, with the instance, its config file and the
    /// deployment's store URL in its environment, restarted 30 s after a failure.
    pub(super) fn service(&self, instance: &Instance, binary: &Binary, token: &str) -> Service {
        let name = &instance.name.0;
        let file = self.config_file.display().to_string();
        let mut env = vec![
            format!("{}={file}", crate::config::CONFIG_VARIABLE),
            format!("{}={name}", crate::config::INSTANCE_VARIABLE),
            format!("{}={}", crate::store::STORE_VARIABLE, self.url),
            format!("{}={token}", crate::instance::TOKEN_VARIABLE),
            format!(
                "{}={}",
                crate::instance::SOCKET_VARIABLE,
                crate::instance::socket(Path::new(&instance.state)).display()
            ),
        ];
        if let Some(path) = std::env::var_os("PATH") {
            env.push(format!("PATH={}", path.to_string_lossy()));
        }
        // The runtime starts in its state directory, which a first start creates; a directory
        // that cannot be made fails the start, which names it.
        let _ = fs::create_dir_all(&instance.state);
        // The box's writable host root, which substrate refuses by name when it is absent.
        let _ = crate::placement::prepare_sessions(Path::new(&instance.state));
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
                binary.path.display().to_string(),
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
        self.put_at(
            instance,
            action,
            reason,
            result,
            detail,
            &crate::actor::daemon(),
            at,
        )
    }

    /// Records one `Reconciliation` of `action` on `instance` by `actor`, now.
    pub(super) fn put(
        &mut self,
        instance: &str,
        action: ReconcileAction,
        reason: ReconcileReason,
        result: ReconcileResult,
        detail: Option<String>,
        actor: &conductor_model::operator::Actor,
    ) -> Result<Done> {
        let at = crate::records::now()?;
        self.put_at(instance, action, reason, result, detail, actor, &at)
    }

    #[allow(clippy::too_many_arguments)]
    fn put_at(
        &mut self,
        instance: &str,
        action: ReconcileAction,
        reason: ReconcileReason,
        result: ReconcileResult,
        detail: Option<String>,
        actor: &conductor_model::operator::Actor,
        at: &conductor_model::primitives::Timestamp,
    ) -> Result<Done> {
        let data = ReconciliationData {
            reconciliation_id: ReconciliationId(Uuid(eventlog_core::new_event_id())),
            instance: InstanceName(instance.to_owned()),
            action,
            reason,
            result,
            detail: detail.clone(),
            actor: actor.clone(),
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

    /// Writes [`RUNTIMES_FILE`] and [`BINARIES_FILE`].
    pub(super) fn save(&self) -> Result<()> {
        keep(&self.dir, RUNTIMES_FILE, &self.started)?;
        keep(&self.dir, BOXES_FILE, &self.boxes)?;
        keep(&self.dir, BINARIES_FILE, &self.pins)
    }

    /// The rows of `GET /v1/instances` (`conductor.instance.Instances`): every instance of the
    /// config with its placement and roles, and from what runs its state (`Running`, `Stopped`
    /// when the daemon started none, `Unreachable` when the one it started stopped, `Upgrading`
    /// while an upgrade waits for its runtime to answer), the version of the binary its runtime
    /// runs, and when the daemon last saw it running.
    #[must_use]
    pub fn rows(&self) -> Vec<Value> {
        self.config
            .instances
            .iter()
            .map(|instance| {
                let name = &instance.name.0;
                let started = self.started.get(name);
                let upgrading = self
                    .pins
                    .get(name)
                    .is_some_and(|pinned| pinned.upgrading.is_some());
                let state = match (started, self.running.get(name)) {
                    (None, _) => InstanceState::Stopped,
                    (Some(_), Some(false)) => InstanceState::Unreachable,
                    (Some(_), _) if upgrading => InstanceState::Upgrading,
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
    /// The runtimes the API proxies to (`story:instance-api`): each instance of the config whose
    /// runtime the daemon started, with a token, and that was not seen stopped, by name, at its
    /// state directory's socket. A runtime in a box on a remote daemon is reached through a relay
    /// session in its box instead (`story:placement-substrate-remote`): the box's own conductor
    /// running `instance relay` on that socket ([`crate::placement::Exchange::relay`]).
    #[must_use]
    pub fn routes(&self) -> BTreeMap<String, Route> {
        self.config
            .instances
            .iter()
            .filter_map(|instance| {
                let name = &instance.name.0;
                if self.running.get(name) == Some(&false) {
                    return None;
                }
                let started = self.started.get(name)?;
                let token = started.token.clone()?;
                let socket = crate::instance::socket(Path::new(&instance.state));
                let remote = self.boxes.get(name).and_then(|opened| {
                    let at = opened.remote.clone()?;
                    let request = crate::placement::request(
                        instance,
                        &self.config_file,
                        Path::new(&started.binary),
                        Vec::new(),
                    );
                    Some(crate::placement::RemoteRoute {
                        bearer: crate::placement::bearer_from(&self.config.daemon.secrets, &at),
                        at,
                        opener: self.remote.clone(),
                        exchange: crate::placement::Exchange::relay(
                            &opened.workspace,
                            &request,
                            &socket,
                        ),
                    })
                });
                Some((
                    name.clone(),
                    Route {
                        socket,
                        token,
                        remote,
                    },
                ))
            })
            .collect()
    }
}

/// One action on the host role `role`, as a [`Done`] whose instance is `host:<role>`; it is not
/// recorded, since a `Reconciliation` names an instance (`story:host-roles-by-daemon`).
fn host_done(
    role: &str,
    action: ReconcileAction,
    reason: ReconcileReason,
    outcome: Result<()>,
) -> Done {
    let (result, detail) = match outcome {
        Ok(()) => (ReconcileResult::Succeeded, None),
        Err(error) => (ReconcileResult::Refused, Some(format!("{error:#}"))),
    };
    Done {
        instance: format!("{}:{role}", host_roles::PLACEMENT),
        action,
        reason,
        result,
        detail,
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

/// What `file` keeps, JSON; empty when there is no file.
fn read_kept<T: serde::de::DeserializeOwned + Default>(file: &Path) -> Result<T> {
    match fs::read_to_string(file) {
        Ok(text) => serde_json::from_str(&text).with_context(|| format!("read {}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error).with_context(|| format!("read {}", file.display())),
    }
}

/// Writes `value` as JSON to `dir/name`, through a file beside it.
fn keep<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<()> {
    let file = dir.join(name);
    let next = dir.join(format!("{name}.next"));
    let text = serde_json::to_string_pretty(value)?;
    fs::write(&next, text + "\n").with_context(|| format!("write {}", next.display()))?;
    fs::rename(&next, &file).with_context(|| format!("write {}", file.display()))
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
    // A `conductor.config/1` file is read once by `conductor config migrate`, never by the
    // daemon (`story:config-globals`).
    if crate::config::first_version(&text) {
        return Err(anyhow!("{}", crate::config::first_version_refusal(file)));
    }
    let config = crate::config::parse(&text, home).map_err(|problems| {
        let lines: Vec<String> = problems
            .iter()
            .map(|problem| format!("{}: {problem}", file.display()))
            .collect();
        anyhow!("{}", lines.join("\n"))
    })?;
    Ok((text, config))
}

/// `at` in whole seconds since the epoch; 0 before it.
fn epoch(at: std::time::SystemTime) -> u64 {
    at.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
