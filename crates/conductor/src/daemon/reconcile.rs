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
//! - leaves it alone when it matches, and records nothing;
//! - does not start again at once a runtime that stopped running before it ran [`RAN_BOUND`]
//!   (`story:box-crash-loop-reads-failed`): found stopped at a look less than [`START_BOUND`]
//!   and one look after its start it is a failed start, recorded as a `Start` that was
//!   `Refused` with what the runtime wrote to its log since that start; either way its instance
//!   is `Unreachable` and its next start waits [`retry_after`] the failed starts in a row.
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
    Reconciliation, ReconciliationData, ReconciliationId, ReconciliationSnapshot,
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

/// A runtime that no longer runs this long after its start did not start: a failed start
/// (`story:box-crash-loop-reads-failed`). The daemon looks every tick, so the look that finds
/// it is less than this and one tick after the start ([`start_window`]).
pub const START_BOUND: Duration = Duration::from_secs(10);

/// A runtime that ran this long ends the row of its failed starts: stopped later, it is started
/// again at once; stopped sooner, its next start waits [`retry_after`] and the row goes on.
pub const RAN_BOUND: Duration = Duration::from_secs(60);

/// How long after a start a look that finds the runtime stopped finds a failed start:
/// [`START_BOUND`] and the one tick the daemon looks at.
fn start_window() -> Duration {
    START_BOUND + super::TICK
}

/// How long after a start a look that finds the runtime stopped finds one that ran
/// [`RAN_BOUND`]: that and the one tick the daemon looks at. A look sooner may find a runtime
/// that stopped just short of it.
fn ran_window() -> Duration {
    RAN_BOUND + super::TICK
}

/// The longest wait before a runtime whose starts fail is started again ([`retry_after`]).
pub const RETRY_CAP: Duration = Duration::from_secs(300);

/// The most of a runtime's log a failed start's record carries: its last bytes and lines.
const SAID_BYTES: u64 = 4096;
const SAID_LINES: usize = 8;

/// How long a runtime waits for its next start after `failures` failed starts in a row:
/// [`START_BOUND`], twice as long after each further one, at most [`RETRY_CAP`].
#[must_use]
pub fn retry_after(failures: u32) -> Duration {
    let doubled = 1_u32
        .checked_shl(failures.saturating_sub(1))
        .unwrap_or(u32::MAX);
    START_BOUND.saturating_mul(doubled).min(RETRY_CAP)
}

/// The last start of one runtime by this process; a restarted daemon knows none, and starts a
/// runtime it finds stopped at once.
#[derive(Debug, Clone)]
struct Attempt {
    /// When it was started.
    started: Instant,
    /// How long its log was then: what follows is this start's.
    log_from: u64,
    /// The failed starts in a row, this one included once it failed.
    failures: u32,
    /// When it is started again, once it failed.
    retry_at: Option<Instant>,
}

/// The log of `instance`'s runtime in a box ([`crate::placement::RUNTIME_LOG`]).
fn runtime_log(instance: &Instance) -> PathBuf {
    crate::placement::sessions_dir(Path::new(&instance.state)).join(crate::placement::RUNTIME_LOG)
}

/// A runtime's log as the daemon may read it.
enum Log {
    /// There is none.
    Absent,
    /// A regular file, open.
    File(fs::File),
    /// Something else is in its place: a link, a pipe, a directory, a link for its directory.
    Irregular,
}

/// Opens `log` to read it. The box writes the directory it is in, and the daemon runs in no
/// box: no link is followed, for the log or for its directory, nothing that is no regular file
/// is read, and no open waits (a named pipe without a writer).
fn open_log(log: &Path) -> Log {
    use nix::errno::Errno;
    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;
    let (Some(directory), Some(name)) = (log.parent(), log.file_name()) else {
        return Log::Irregular;
    };
    let held = match open(
        directory,
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    ) {
        Ok(held) => held,
        Err(Errno::ENOENT) => return Log::Absent,
        Err(_) => return Log::Irregular,
    };
    let opened = match openat(
        &held,
        name,
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC,
        Mode::empty(),
    ) {
        Ok(opened) => fs::File::from(opened),
        Err(Errno::ENOENT) => return Log::Absent,
        Err(_) => return Log::Irregular,
    };
    match opened.metadata() {
        Ok(metadata) if metadata.is_file() => Log::File(opened),
        _ => Log::Irregular,
    }
}

/// How long `log` is: where a start's own output begins. Nothing when it is no regular file.
fn log_len(log: &Path) -> u64 {
    match open_log(log) {
        Log::File(file) => file.metadata().map_or(0, |metadata| metadata.len()),
        Log::Absent | Log::Irregular => 0,
    }
}

/// What a failed start's record says of `log` from the byte `from` on, as one line: its last
/// [`SAID_LINES`] lines that are not empty, of its last [`SAID_BYTES`] bytes, joined by ` | `,
/// each [`printable`]. A log shorter than `from` was replaced, so all of it counts. A log that
/// is no regular file is named as that and none of it is read ([`open_log`]).
fn runtime_said(log: &Path, from: u64) -> String {
    use std::io::{Read as _, Seek as _, SeekFrom};
    let nothing = || format!("{} holds nothing of this start", log.display());
    let mut file = match open_log(log) {
        Log::File(file) => file,
        Log::Absent => return nothing(),
        Log::Irregular => {
            return format!(
                "{} is no regular file, nothing of it is read",
                log.display()
            );
        }
    };
    let mut bytes = Vec::new();
    let read = file.metadata().and_then(|metadata| {
        let len = metadata.len();
        let from = if from > len { 0 } else { from };
        file.seek(SeekFrom::Start(from.max(len.saturating_sub(SAID_BYTES))))?;
        file.take(SAID_BYTES).read_to_end(&mut bytes)
    });
    if read.is_err() {
        return nothing();
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|line| printable(line.trim()))
        .filter(|line| !line.is_empty())
        .collect();
    let kept = &lines[lines.len().saturating_sub(SAID_LINES)..];
    if kept.is_empty() {
        return nothing();
    }
    // A replaced byte is three in the record: the bound holds for what enters it.
    let said = kept.join(" | ");
    let mut from = said.len().saturating_sub(SAID_BYTES as usize);
    while !said.is_char_boundary(from) {
        from += 1;
    }
    said[from..].to_owned()
}

/// Whether `character` reorders or breaks the one line of a record where it is shown: a
/// bidirectional mark, embedding, override or isolate, or the line or paragraph separator.
fn reorders(character: char) -> bool {
    matches!(
        character,
        '\u{61c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

/// `line` with nothing a terminal acts on: each escape sequence (CSI to its final byte, OSC to
/// its BEL or `ESC \`, else `ESC` and the character after it) and each other control character
/// (C0 but the tab, DEL, C1) and each character that [`reorders`] is one U+FFFD. The text goes to the record and from there to
/// conductord's output.
fn printable(line: &str) -> String {
    const REPLACED: char = char::REPLACEMENT_CHARACTER;
    let mut out = String::with_capacity(line.len());
    let mut rest = line.chars().peekable();
    while let Some(character) = rest.next() {
        if character == '\u{1b}' {
            match rest.next() {
                Some('[') => {
                    for inner in rest.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&inner) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(inner) = rest.next() {
                        if inner == '\u{7}' {
                            break;
                        }
                        if inner == '\u{1b}' {
                            rest.next_if_eq(&'\\');
                            break;
                        }
                    }
                }
                _ => {}
            }
            out.push(REPLACED);
        } else if (character.is_control() && character != '\t') || reorders(character) {
            out.push(REPLACED);
        } else {
            out.push(character);
        }
    }
    out
}

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
    /// The reconciliations the store refused, in the order they were taken, held until
    /// [`Daemon::record_held`] writes them (`story:daemon-probes-and-metrics`).
    held: Vec<ReconciliationSnapshot>,
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
    /// The last start of each runtime this process started, by instance.
    attempts: BTreeMap<String, Attempt>,
    /// When the daemon last looked ([`Daemon::due`], [`Daemon::pass`]).
    looked: Option<Instant>,
    last_pass: Option<Instant>,
    /// Why the next pass runs while a record the store refused left one undone: the reason of
    /// the pass it is part of, `Cadence` for a record of no pass ([`Daemon::pass`],
    /// [`Daemon::due`]).
    rerun: Option<ReconcileReason>,
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
            held: Vec::new(),
            url,
            config,
            text,
            started,
            running: BTreeMap::new(),
            pins,
            hosts: BTreeMap::new(),
            attempts: BTreeMap::new(),
            looked: None,
            last_pass: None,
            rerun: None,
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
    /// a runtime the daemon started stopped running, or one whose start failed has waited its
    /// time ([`retry_after`]); or a cadence passed since the last pass. A pass a refused record
    /// left undone runs again at the next tick for its own reason, or for the config change
    /// when the file changed since it read it (`story:daemon-probes-and-metrics`).
    /// It notes, for [`Daemon::rows`], which runtimes run.
    ///
    /// # Errors
    ///
    /// The service manager cannot be asked.
    pub fn due(&mut self, now: Instant) -> Result<Option<ReconcileReason>> {
        let lost = self.observe()?;
        self.looked = Some(now);
        let changed =
            fs::read_to_string(&self.config_file).ok().as_deref() != Some(self.text.as_str());
        let Some(last) = self.last_pass else {
            return Ok(Some(match self.rerun {
                Some(_) if changed => ReconcileReason::ConfigChange,
                Some(reason) => reason,
                None => ReconcileReason::Cadence,
            }));
        };
        if changed {
            return Ok(Some(ReconcileReason::ConfigChange));
        }
        let retry = self.attempts.iter().any(|(name, attempt)| {
            attempt.retry_at.is_some_and(|at| now >= at) && self.started.contains_key(name)
        });
        if lost || retry {
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
            .and_then(|()| match request.refused.clone() {
                Some(why) => Err(why),
                None => substrate.restart(&opened.workspace, &request),
            });
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
    /// A pass that fails still writes [`RUNTIMES_FILE`] with what it did before it failed, so a
    /// daemon stopped before its next pass knows each runtime it started, restarted or stopped.
    /// One that fails on a record the store refused is not done: the next tick runs it again for
    /// `reason` ([`Daemon::due`], `story:daemon-probes-and-metrics`).
    ///
    /// # Errors
    ///
    /// The config file does not read or is not valid (nothing is done), the service manager
    /// cannot be asked, a reconciliation cannot be recorded, or [`RUNTIMES_FILE`] cannot be
    /// written.
    pub fn pass(&mut self, reason: ReconcileReason, now: Instant) -> Result<Vec<Done>> {
        let passed = self.pass_once(reason, now);
        if passed.is_err() {
            if self.holds_records() {
                self.rerun = Some(reason);
            }
            if let Err(unkept) = self.save() {
                return passed.map_err(|error| error.context(format!("{unkept:#}")));
            }
        }
        passed
    }

    /// [`Daemon::pass`] up to its first failure.
    fn pass_once(&mut self, reason: ReconcileReason, now: Instant) -> Result<Vec<Done>> {
        let (text, config) = read_config(&self.config_file, &self.home)?;
        self.text = text;
        self.config = config;
        self.last_pass = Some(now);
        self.rerun = None;
        self.looked = Some(now);
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
            let log_from = log_len(&runtime_log(instance));
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
                    // A start that failed is recorded once and waits; nothing starts meanwhile.
                    if held.is_some()
                        && let Some(failed) = self.failed_start(instance, reason, now, &at)?
                    {
                        done.extend(failed);
                        self.running.insert(name, false);
                        continue;
                    }
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
                let failures = match action {
                    ReconcileAction::Start => self
                        .attempts
                        .get(&name)
                        .map_or(0, |attempt| attempt.failures),
                    _ => 0,
                };
                self.attempts.insert(
                    name.clone(),
                    Attempt {
                        started: now,
                        log_from,
                        failures,
                        retry_at: None,
                    },
                );
                self.started.insert(name.clone(), wanted);
                self.pins.entry(name.clone()).or_insert_with(|| Pinned {
                    desired,
                    previous: None,
                    upgrading: None,
                });
            } else {
                self.started.remove(&name);
                self.attempts.remove(&name);
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
        self.attempts
            .retain(|name, _| instances.iter().any(|instance| &instance.name.0 == name));
        for name in gone {
            self.running.remove(&name);
            let Some(held) = self.started.remove(&name) else {
                continue;
            };
            // A runtime that stopped leaves its box open while it waits for its next start:
            // closed here too, its leases, its `boxes.json` entry and its slot released.
            let dead = self
                .boxes
                .get(&name)
                .is_some_and(|opened| boxed_unit(opened) == held.unit);
            if self.runs(&name, &held.unit)? || dead {
                let outcome = self.stop_runtime(&name, &held.unit);
                done.push(self.record(&name, ReconcileAction::Stop, reason, outcome, &at)?);
            }
        }
        done.extend(self.keep_host_roles(reason)?);
        self.save()?;
        Ok(done)
    }

    /// Whether the runtime of `instance`, found stopped at `now`, is not to be started in this
    /// pass (`story:box-crash-loop-reads-failed`). `None`: start it. `Some`: it waits; the
    /// first pass that finds it records why and answers that.
    ///
    /// A start failed when the look that finds the runtime stopped is less than
    /// [`start_window`] after it. It is recorded as a `Start` for `LostRuntime` that was
    /// `Refused`, the detail what the runtime in a box wrote to its log since that start
    /// ([`runtime_said`]); a runtime in a unit has no such log, its unit's journal holds what
    /// it said. A runtime found stopped later that ran less than [`RAN_BOUND`] is recorded the
    /// same way as one that stopped short of it. Both count in the row of failed starts: the
    /// next start is [`retry_after`] that row away, and the instance reads `Unreachable` until
    /// then. A pass for a changed config file or an operator's command ends the wait: the cause
    /// may be gone. A runtime that ran [`RAN_BOUND`] ends the row and is started at once.
    fn failed_start(
        &mut self,
        instance: &Instance,
        reason: ReconcileReason,
        now: Instant,
        at: &conductor_model::primitives::Timestamp,
    ) -> Result<Option<Option<Done>>> {
        let name = &instance.name.0;
        let asked = matches!(
            reason,
            ReconcileReason::ConfigChange | ReconcileReason::OperatorCommand
        );
        let Some(attempt) = self.attempts.get_mut(name) else {
            return Ok(None);
        };
        if let Some(retry_at) = attempt.retry_at {
            return Ok((now < retry_at && !asked).then_some(None));
        }
        let ran = now.saturating_duration_since(attempt.started);
        if ran >= ran_window() {
            attempt.failures = 0;
            return Ok(None);
        }
        attempt.failures = attempt.failures.saturating_add(1);
        let wait = if asked {
            Duration::ZERO
        } else {
            retry_after(attempt.failures)
        };
        attempt.retry_at = Some(now + wait);
        let (failures, from) = (attempt.failures, attempt.log_from);
        let said =
            crate::placement::boxed(instance).then(|| runtime_said(&runtime_log(instance), from));
        let how = if ran < start_window() {
            format!("exited within {} s of its start", START_BOUND.as_secs())
        } else {
            format!(
                "was found stopped {} s after its start, not known to have run the {} s that end \
                 a row of failed starts",
                ran.as_secs(),
                RAN_BOUND.as_secs()
            )
        };
        let failed = anyhow!(
            "the runtime of {name} {how} (failed start {failures} in a row, the next start in \
             {} s){}",
            wait.as_secs(),
            said.map(|said| format!(": {said}")).unwrap_or_default()
        );
        self.record(
            name,
            ReconcileAction::Start,
            ReconcileReason::LostRuntime,
            Err(failed),
            at,
        )
        .map(|done| Some(Some(done)))
    }

    /// Whether the newest start of `name` follows a failed one and was not yet seen to hold:
    /// the daemon's last look was less than [`start_window`] after it.
    fn unproven(&self, name: &str) -> bool {
        self.attempts.get(name).is_some_and(|attempt| {
            attempt.failures > 0
                && attempt.retry_at.is_none()
                && self.looked.is_none_or(|looked| {
                    looked.saturating_duration_since(attempt.started) < start_window()
                })
        })
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
    /// substrate's name, and nothing falls back. The runtime's token is not in the box's
    /// environment: a local box whose daemon declares the instance's sealed slot takes it
    /// through that slot ([`Daemon::box_request`], [`crate::placement::api_token_slot`]), any
    /// other box not at all, and its model token goes through the slot pool
    /// ([`crate::placement::SlotPool`]).
    pub(super) fn start_runtime(
        &mut self,
        instance: &Instance,
        service: &Service,
    ) -> Result<String> {
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
    /// its environment: substrate seals only slots its daemon declares. The token `service`
    /// carries for the runtime goes in through the instance's sealed slot instead
    /// (`story:box-reaches-forge-and-daemon`), where the local substrate daemon declares that
    /// slot ([`crate::placement::Substrate::declares`]): it is written into the slot's file
    /// under the daemon's directory here, just before the box is asked for, and a file that
    /// cannot be written refuses the box ([`crate::placement::seal_api_token`]). Where the
    /// daemon does not declare the slot the request names none, nothing is written and nothing
    /// is refused: the box is asked for as before there was a slot, and its runtime mints a
    /// token of its own, as that of a box on a remote daemon does, which has no such slot. A
    /// box inherits nothing, so its `HOME` is set: the daemon's, which the config file's `~`
    /// stands for. The box writes nothing of the daemon's own directory
    /// ([`crate::placement::Host::daemon`]).
    fn box_request(
        &mut self,
        instance: &Instance,
        service: &Service,
    ) -> crate::placement::BoxRequest {
        let variable = format!("{}=", crate::instance::TOKEN_VARIABLE);
        let token = service
            .env
            .iter()
            .find_map(|pair| pair.strip_prefix(&variable));
        let mut env: Vec<(String, String)> = service
            .env
            .iter()
            .filter(|pair| !pair.starts_with(&variable))
            .filter_map(|pair| pair.split_once('='))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect();
        if env.iter().all(|(name, _)| name != "HOME") {
            env.push(("HOME".to_owned(), self.home.display().to_string()));
        }
        let binary = PathBuf::from(service.command.first().cloned().unwrap_or_default());
        let mut host = crate::placement::Host::of(&env);
        host.daemon = Some(self.dir.clone());
        let mut request =
            crate::placement::request_on(instance, &self.config_file, &binary, env, &host);
        if request.refused.is_none()
            && let Some(slot) = request.api_token.take()
        {
            // The slot is named only where the daemon declares it and there is a token to seal.
            match token {
                Some(token) if self.substrate.declares(&slot.name) => {
                    if let Err(refused) = crate::placement::seal_api_token(&slot, token) {
                        request.refused = Some(refused);
                    }
                    request.api_token = Some(slot);
                }
                _ => {}
            }
        }
        request
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
    pub(super) fn stop_runtime(&mut self, name: &str, unit: &str) -> Result<()> {
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
        // In the box's writable state root, which substrate refuses by name when it is absent.
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
        self.held
            .push(AnyReconciliation::Recorded(Reconciliation::new(data)).snapshot());
        if let Err(error) = self.record_held() {
            // The pass this record is part of is not done: the next tick runs one, for that
            // pass's reason ([`Daemon::pass`]), `Cadence` for a record of no pass.
            self.last_pass = None;
            self.rerun = self.rerun.or(Some(ReconcileReason::Cadence));
            return Err(error.context(format!("record the reconciliation of {instance}")));
        }
        Ok(Done {
            instance: instance.to_owned(),
            action,
            reason,
            result,
            detail,
        })
    }

    /// Writes the reconciliations the store refused, in the order they were taken
    /// (`story:daemon-probes-and-metrics`). A handle that failed refuses every later write
    /// ([`Store::check`]), so the store is opened anew first when this one failed. Each record
    /// keeps its id, and a reconciliation is one stream by its id: one whose refused write did
    /// land is not recorded twice. Nothing is held when it answers.
    ///
    /// # Errors
    ///
    /// The store does not open, or refuses a write again: the records stay held.
    pub fn record_held(&mut self) -> Result<()> {
        if self.held.is_empty() {
            return Ok(());
        }
        if self.store.check().is_err() {
            self.store = crate::store::open_url(&self.url, super::TENANT)
                .map_err(|error| anyhow!("daemon.store: {error:#}"))?;
        }
        for snapshot in &self.held {
            ReconciliationStorage::put(&mut self.store, snapshot.clone());
        }
        self.store.check().map_err(|error| anyhow!("{error}"))?;
        self.held.clear();
        Ok(())
    }

    /// Whether a reconciliation the store refused is still held ([`Self::record_held`]).
    #[must_use]
    pub const fn holds_records(&self) -> bool {
        !self.held.is_empty()
    }

    /// Writes [`RUNTIMES_FILE`] and [`BINARIES_FILE`].
    pub(super) fn save(&self) -> Result<()> {
        keep(&self.dir, RUNTIMES_FILE, &self.started)?;
        keep(&self.dir, BOXES_FILE, &self.boxes)?;
        keep(&self.dir, BINARIES_FILE, &self.pins)
    }

    /// The rows of `GET /v1/instances` (`conductor.instance.Instances`): every instance of the
    /// config with its placement and roles, and from what runs its state (`Running`, `Stopped`
    /// when the daemon started none, `Unreachable` when the one it started stopped, so also
    /// while one whose start failed waits for its next start and until the start after it held
    /// the start bound ([`Daemon::unproven`]), `Upgrading`
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
                    // An upgrade under way reads as that until it settled, whatever came before.
                    (Some(_), _) if upgrading => InstanceState::Upgrading,
                    // The start after a failed one has not held the start bound yet.
                    (Some(_), _) if self.unproven(name) => InstanceState::Unreachable,
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
                        tcp: None,
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

/// What `daemon serve` reads of `config_file` before its store answers
/// (`story:daemon-probes-and-metrics`): the config, `~` read as `home`, and the `daemon.store`
/// the file writes. [`Daemon::open`] reads both again once the store answers.
///
/// # Errors
///
/// As [`Daemon::open`] refuses a config file: it does not read, is not valid, or writes no
/// `daemon.store` the daemon opens.
pub fn written(config_file: &Path, home: &Path) -> Result<(Config, String)> {
    let (text, config) = read_config(config_file, home)?;
    let url = written_store(config_file, &text)?;
    Ok((config, url))
}

/// `at` in whole seconds since the epoch; 0 before it.
fn epoch(at: std::time::SystemTime) -> u64 {
    at.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
