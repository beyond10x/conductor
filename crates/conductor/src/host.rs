//! `conductor host`: the host ledger of slots and disk reservations (`story:build-run`), and the
//! operator item a disk shortfall becomes (`story:disk-escalation`).
//!
//! The ledger is the `conductor.host.HostGrant`s of the store under the state directory. A
//! command reads its input from its flags, opens the store ([`crate::state::open`]), runs the
//! generated behaviour, and checks that the store kept what the behaviour reported before it
//! answers, as `conductor resource` does. `request-disk` and `take-host-grant` print the id of
//! the grant on one line. A refused command exits 1 with one line on standard error: the command,
//! the error the specification declares, and what it means. `--input-json` is not read yet, and
//! answers [`NotImplemented`](crate::NotImplemented). `host grants` opens only an existing store
//! and lists the grants newest first.
//!
//! [`Ledger`] is what `conductor build run` ([`crate::build`]) reserves through: it holds the
//! ledger's lock, `host.lock` in the state directory, from the request to its answer, so a second
//! run reserving at the same time counts the first. Nothing expires by time: whenever a run
//! reads the ledger it first releases, with `ProcessGroupGone`, every running grant whose process
//! group is gone ([`Ledger::sweep`]).
//!
//! **Escalation.** A refused reservation records one operator item ([`escalate`]): a decision
//! request raised and escalated as class H, so `decision hands-todo` lists it, naming the
//! shortfall, what reclaim left by path and size, and the one command only a person runs. One item
//! per fall of free disk, whatever command or repository is refused in it
//! (`story:disk-item-per-fall`): `host.fall` in the state directory holds the item's id, and a
//! refusal while it is there records none. A reservation that fits does not end the fall; free
//! disk back at `thresholds.disk_clear` does, read by the next `build run` or the watch's pass
//! ([`clear_fall`]), which withdraws the item (`WithdrawRequest`) and removes the marker. The item
//! is the instance's, not one repository's. It names no personal path
//! (`story:disk-item-redacted`): the entries of `/var/tmp` only as a count and a total size, and
//! the home directory as `~` wherever it appears.

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    DecisionRequestStorage, Generated, HostGrantStorage, TryContext, unmet_context,
};
use conductor_model::config::{InstanceName, SessionName};
use conductor_model::decision::{DecisionRequestState, RequestId};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::{GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::host::obligations::{
    GrantsQuery, RecordPeakBehavior, ReleaseGrantBehavior, RequestDiskBehavior,
    ReserveDiskBehavior, StartRunBehavior, TakeHostGrantBehavior,
};
use conductor_model::host::{
    GrantId, GrantKind, Grants, HostGrantSnapshot, HostGrantState, ReclaimRunId, RecordPeak,
    RecordPeakOutcome, ReleaseGrant, ReleaseGrantOutcome, ReleaseReason, RequestDisk,
    RequestDiskOutcome, ReserveDisk, ReserveDiskOutcome, StartRun, StartRunOutcome, TakeHostGrant,
    TakeHostGrantOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::Timestamp;
use nix::errno::Errno;
use nix::sys::signal::killpg;
use nix::unistd::Pid;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{
    EscalateRequestArgs, JsonInput, RaiseDecisionRequestArgs, RecordPeakArgs, ReleaseGrantArgs,
    RequestDiskArgs, ReserveDiskArgs, StartRunArgs, TakeHostGrantArgs, ViewArgs,
    WithdrawRequestArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// The ledger's lock, in the state directory.
const LOCK: &str = "host.lock";

/// The open fall's operator item, in the state directory.
const FALL: &str = "host.fall";

/// One GiB.
const GIB: u64 = 1 << 30;

/// `at` as the RFC 3339 instant a `Timestamp` carries.
#[must_use]
pub fn now(at: OffsetDateTime) -> Timestamp {
    Timestamp(
        at.format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned()),
    )
}

/// Bytes as whole GiB, rounded up, as the watch writes free disk.
#[must_use]
pub fn gib(bytes: u64) -> u64 {
    bytes.div_ceil(GIB)
}

/// Whether the process group `group` still has a process. A group that cannot be signalled for
/// another reason than its absence (another user's) is taken as alive.
#[must_use]
pub fn alive(group: i64) -> bool {
    let Ok(group) = i32::try_from(group) else {
        return false;
    };
    if group <= 0 {
        return false;
    }
    !matches!(killpg(Pid::from_raw(group), None), Err(Errno::ESRCH))
}

/// The ledger, held under its lock from a request to its answer.
pub struct Ledger {
    generated: Generated<Ports>,
    state: PathBuf,
    _lock: File,
}

/// The directory of the run locks: each `build run` holds `<state>/host-runs/<grant>.lock` while it
/// lives, so a sweep never releases a grant whose command ended but whose wrapper is still recording
/// its peak and releasing it.
const RUNS: &str = "host-runs";

/// The run lock of `grant`, held by its `build run` until the grant is released.
///
/// # Errors
///
/// The directory or the file cannot be made, or locked.
pub fn hold_run(state: &Path, grant: &GrantId) -> Result<File> {
    let dir = state.join(RUNS);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!("{}.lock", grant.0));
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    file.lock()
        .with_context(|| format!("lock {}", path.display()))?;
    Ok(file)
}

/// Drops the run lock of `grant` and its file, once the grant is released.
pub fn end_run(state: &Path, grant: &GrantId, lock: File) {
    drop(lock);
    let _ = fs::remove_file(state.join(RUNS).join(format!("{}.lock", grant.0)));
}

/// Whether a live `build run` holds the run lock of `grant`. A grant with no lock file (a run of an
/// older binary) is not held.
fn run_held(state: &Path, grant: &GrantId) -> bool {
    let path = state.join(RUNS).join(format!("{}.lock", grant.0));
    let Ok(file) = OpenOptions::new().write(true).open(&path) else {
        return false;
    };
    file.try_lock().is_err()
}

/// How [`Ledger::reserve`] answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// It fitted.
    Granted,
    /// It fitted once a reclaim run had freed space.
    GrantedAfterReclaim,
    /// Reclaim could not reach the size.
    Refused,
}

impl Ledger {
    /// Takes the ledger's lock in the state directory `state`, waiting for another holder, and
    /// opens the store there.
    ///
    /// # Errors
    ///
    /// The directory or its lock file cannot be made or locked, or the store does not open.
    pub fn lock(state: &Path) -> Result<Self> {
        fs::create_dir_all(state).with_context(|| format!("create {}", state.display()))?;
        let path = state.join(LOCK);
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("open the host ledger's lock {}", path.display()))?;
        lock.lock()
            .with_context(|| format!("lock the host ledger {}", path.display()))?;
        let store = crate::store::open(state)?;
        Ok(Self {
            generated: Generated::new(Ports { store }),
            state: state.to_owned(),
            _lock: lock,
        })
    }

    /// [`Ledger::lock`] without waiting: `None` while another holder has the lock.
    ///
    /// # Errors
    ///
    /// The directory or its lock file cannot be made, or the store does not open.
    pub fn try_lock(state: &Path) -> Result<Option<Self>> {
        fs::create_dir_all(state).with_context(|| format!("create {}", state.display()))?;
        let path = state.join(LOCK);
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("open the host ledger's lock {}", path.display()))?;
        if lock.try_lock().is_err() {
            return Ok(None);
        }
        let store = crate::store::open(state)?;
        Ok(Some(Self {
            generated: Generated::new(Ports { store }),
            state: state.to_owned(),
            _lock: lock,
        }))
    }

    /// Closes every grant whose `build run` is gone, and answers their ids: a request no run holds
    /// is answered refused (its requester died while it waited); a granted grant no run holds is
    /// released (its requester died before its command started); a running grant whose process
    /// group is gone and no run holds is released. Each is released with `ProcessGroupGone`. A run
    /// holds its lock from its request to its release ([`hold_run`]).
    ///
    /// # Errors
    ///
    /// The store does not read or keep a release.
    pub fn sweep(&mut self, at: &Timestamp) -> Result<Vec<GrantId>> {
        let state = self.state.clone();
        let grants = HostGrantStorage::list(&self.generated.ports);
        kept("host ledger", &self.generated.ports.store)?;
        let mut requests = Vec::new();
        let mut gone = Vec::new();
        for grant in grants {
            let id = grant.data.grant_id;
            if run_held(&state, &id) {
                continue;
            }
            match grant.state {
                HostGrantState::Requested => requests.push((id, grant.data.bytes.unwrap_or(1))),
                HostGrantState::Granted => gone.push(id),
                HostGrantState::Running
                    if grant.data.process_group.is_some_and(|group| !alive(group)) =>
                {
                    gone.push(id);
                }
                _ => {}
            }
        }
        let mut closed = Vec::new();
        for (grant, bytes) in requests {
            let shortfall = u64::try_from(bytes).unwrap_or(1).max(1);
            self.reserve(&grant, shortfall, 0)?;
            closed.push(grant);
        }
        for grant in gone {
            release_in(
                &mut self.generated,
                &grant,
                Some(ReleaseReason::ProcessGroupGone),
                at,
            )?;
            closed.push(grant);
        }
        Ok(closed)
    }

    /// The bytes the live grants still hold, but `except`: a granted grant its whole reservation;
    /// a running one what it has not written yet, its reservation less the growth its run
    /// recorded (`peak_bytes`), since free disk already counts what it wrote.
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn reserved(&self, except: Option<&GrantId>) -> Result<u64> {
        let total = live(&HostGrantStorage::list(&self.generated.ports))
            .filter(|grant| except != Some(&grant.data.grant_id))
            .map(|grant| {
                let bytes = grant
                    .data
                    .bytes
                    .map_or(0, |bytes| u64::try_from(bytes).unwrap_or(0));
                let written = if grant.state == HostGrantState::Running {
                    grant
                        .data
                        .peak_bytes
                        .map_or(0, |peak| u64::try_from(peak).unwrap_or(0))
                } else {
                    0
                };
                bytes.saturating_sub(written)
            })
            .sum();
        kept("host ledger", &self.generated.ports.store)?;
        Ok(total)
    }

    /// `RequestDisk`: records the request of `bytes` for `command`, and answers its grant's id.
    ///
    /// # Errors
    ///
    /// The specification refuses it (`InvalidSize`), or the store does not keep it.
    pub fn request(
        &mut self,
        instance: &str,
        repository: Option<&str>,
        session_name: Option<&str>,
        bytes: u64,
        command: &str,
        at: &Timestamp,
    ) -> Result<GrantId> {
        const COMMAND: &str = "host request-disk";
        let input = RequestDisk {
            instance: InstanceName(instance.to_owned()),
            repository: repository.map(|name| RepositoryName(name.to_owned())),
            session_name: session_name.map(|name| SessionName(name.to_owned())),
            bytes: i64::try_from(bytes).unwrap_or(i64::MAX),
            command: command.to_owned(),
            requested_at: at.clone(),
        };
        let outcome = self
            .generated
            .request_disk(input)
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        kept(COMMAND, &self.generated.ports.store)?;
        match outcome {
            RequestDiskOutcome::Requested { disk_requested } => Ok(disk_requested.grant_id),
            RequestDiskOutcome::InvalidSize { error } => bail!(
                "{COMMAND}: InvalidSize: a reservation asks for a positive size, not {} bytes; \
                 nothing was requested",
                error.bytes
            ),
        }
    }

    /// `ReserveDisk`: answers the request `grant` with what the disk lacks for it and what a
    /// reclaim run freed first.
    ///
    /// # Errors
    ///
    /// The specification refuses it, or the store does not keep it.
    pub fn reserve(&mut self, grant: &GrantId, shortfall: u64, reclaimed: u64) -> Result<Answer> {
        const COMMAND: &str = "host reserve-disk";
        let input = ReserveDisk {
            grant_id: grant.clone(),
            shortfall_bytes: i64::try_from(shortfall).unwrap_or(i64::MAX),
            reclaimed_bytes: i64::try_from(reclaimed).unwrap_or(i64::MAX),
        };
        let outcome = self
            .generated
            .reserve_disk(input)
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        kept(COMMAND, &self.generated.ports.store)?;
        reserve_answer(COMMAND, outcome)
    }
}

/// The live grants of `grants`: granted, or running.
fn live(grants: &[HostGrantSnapshot]) -> impl Iterator<Item = &HostGrantSnapshot> {
    grants.iter().filter(|grant| {
        matches!(
            grant.state,
            HostGrantState::Granted | HostGrantState::Running
        )
    })
}

fn reserve_answer(command: &str, outcome: ReserveDiskOutcome) -> Result<Answer> {
    match outcome {
        ReserveDiskOutcome::Granted { .. } => Ok(Answer::Granted),
        ReserveDiskOutcome::GrantedAfterReclaim { .. } => Ok(Answer::GrantedAfterReclaim),
        ReserveDiskOutcome::Refused { .. } => Ok(Answer::Refused),
        ReserveDiskOutcome::InvalidSize { error } => bail!(
            "{command}: InvalidSize: no count is negative, not {}; the request stays open",
            error.bytes
        ),
        ReserveDiskOutcome::NoSuchGrant { error } => {
            bail!(
                "{command}: HostGrantNotFound: {}",
                no_grant(&error.grant_id)
            )
        }
        ReserveDiskOutcome::WrongState { error } => bail!(
            "{command}: HostGrantStateConflict: grant {} was answered already",
            error.grant_id.0
        ),
    }
}

/// `StartRun`: the reserved run of `grant` started in the process group `group`.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn start_run(state: &Path, grant: &GrantId, group: i64, at: &Timestamp) -> Result<()> {
    const COMMAND: &str = "host start-run";
    let mut ledger = Ledger::lock(state)?;
    let generated = &mut ledger.generated;
    let outcome = generated
        .start_run(StartRun {
            grant_id: grant.clone(),
            process_group: group,
            started_at: at.clone(),
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    start_answer(COMMAND, outcome)
}

fn start_answer(command: &str, outcome: StartRunOutcome) -> Result<()> {
    match outcome {
        StartRunOutcome::Started { .. } => Ok(()),
        StartRunOutcome::NotADiskGrant { error } => bail!(
            "{command}: NotADiskGrant: grant {} is a slot, which runs no command",
            error.grant_id.0
        ),
        StartRunOutcome::NoSuchGrant { error } => {
            bail!(
                "{command}: HostGrantNotFound: {}",
                no_grant(&error.grant_id)
            )
        }
        StartRunOutcome::WrongState { error } => bail!(
            "{command}: HostGrantStateConflict: grant {} is not granted, and only a granted \
             reservation starts its run, once",
            error.grant_id.0
        ),
    }
}

/// `RecordPeak`: the run of `grant` measured `peak` bytes.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn record_peak_of(state: &Path, grant: &GrantId, peak: u64) -> Result<()> {
    let mut ledger = Ledger::lock(state)?;
    record_peak_in(&mut ledger, grant, peak)
}

/// [`record_peak_of`] while the run goes on: skipped, answering `false`, when another holder has
/// the ledger's lock, so the watchdog never waits on a reclaim another run holds the lock for.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn record_peak_now(state: &Path, grant: &GrantId, peak: u64) -> Result<bool> {
    let Some(mut ledger) = Ledger::try_lock(state)? else {
        return Ok(false);
    };
    record_peak_in(&mut ledger, grant, peak)?;
    Ok(true)
}

fn record_peak_in(ledger: &mut Ledger, grant: &GrantId, peak: u64) -> Result<()> {
    const COMMAND: &str = "host record-peak";
    let generated = &mut ledger.generated;
    let outcome = generated
        .record_peak(RecordPeak {
            grant_id: grant.clone(),
            peak_bytes: i64::try_from(peak).unwrap_or(i64::MAX),
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    peak_answer(COMMAND, outcome)
}

fn peak_answer(command: &str, outcome: RecordPeakOutcome) -> Result<()> {
    match outcome {
        RecordPeakOutcome::Recorded { .. } => Ok(()),
        RecordPeakOutcome::InvalidSize { error } => bail!(
            "{command}: InvalidSize: a measured size is not negative, not {}",
            error.bytes
        ),
        RecordPeakOutcome::NoSuchGrant { error } => {
            bail!(
                "{command}: HostGrantNotFound: {}",
                no_grant(&error.grant_id)
            )
        }
        RecordPeakOutcome::NotRunning { error } => bail!(
            "{command}: HostGrantStateConflict: grant {} is not running, and only a running \
             grant records a peak",
            error.grant_id.0
        ),
    }
}

/// `ReleaseGrant`: `grant` is released for `reason`.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn release(
    state: &Path,
    grant: &GrantId,
    reason: Option<ReleaseReason>,
    at: &Timestamp,
) -> Result<()> {
    let mut ledger = Ledger::lock(state)?;
    release_in(&mut ledger.generated, grant, reason, at)
}

fn release_in(
    generated: &mut Generated<Ports>,
    grant: &GrantId,
    reason: Option<ReleaseReason>,
    at: &Timestamp,
) -> Result<()> {
    const COMMAND: &str = "host release-grant";
    let outcome = generated
        .release_grant(ReleaseGrant {
            grant_id: grant.clone(),
            reason,
            released_at: at.clone(),
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    release_answer(COMMAND, outcome)
}

fn release_answer(command: &str, outcome: ReleaseGrantOutcome) -> Result<()> {
    match outcome {
        ReleaseGrantOutcome::Released { .. } => Ok(()),
        ReleaseGrantOutcome::NoSuchGrant { error } => {
            bail!(
                "{command}: HostGrantNotFound: {}",
                no_grant(&error.grant_id)
            )
        }
        ReleaseGrantOutcome::WrongState { error } => bail!(
            "{command}: HostGrantStateConflict: grant {} is neither granted nor running: it is \
             still a request, or was released or refused (see `conductor host grants`)",
            error.grant_id.0
        ),
    }
}

/// The rows of the `Grants` view over the existing store under `state`, newest first.
///
/// # Errors
///
/// There is no store, it does not open, or a grant does not read.
pub fn rows(state: &Path) -> Result<Vec<Grants>> {
    let generated = Generated::new(Ports {
        store: crate::store::open_existing(state)?,
    });
    let mut rows = generated
        .grants()
        .map_err(|unmet| anyhow!("host grants: {unmet}"))?;
    kept("host grants", &generated.ports.store)?;
    rows.reverse();
    Ok(rows)
}

/// What a disk shortfall is, for its operator item.
#[derive(Debug, Clone)]
pub struct Shortfall<'a> {
    /// The instance whose run fell short.
    pub instance: &'a str,
    /// The repository the run builds, if it named one.
    pub repository: Option<&'a str>,
    /// The command that did not run, as the ledger holds it.
    pub command: &'a str,
    /// The bytes it asked for.
    pub bytes: u64,
    /// The bytes the disk still lacks after reclaim.
    pub shortfall: u64,
    /// What reclaim left, by path and size: what only a person may remove.
    pub left: &'a [(String, u64)],
    /// The file system free disk is read on.
    pub disk: &'a Path,
    /// The home directory, written `~` wherever it appears in the item.
    pub home: Option<&'a Path>,
    /// `/var/tmp`: its entries are none of the instance's, and the item counts them, never names
    /// them.
    pub var_tmp: &'a Path,
    /// When.
    pub at: &'a Timestamp,
}

/// What a shortfall left, as an operator item names it: each path outside `/var/tmp` (the home
/// directory written `~`), and the entries of `/var/tmp` as one count and one total size.
#[derive(Debug)]
struct Left {
    /// What is named, by its redacted path, and its size.
    named: Vec<(String, u64)>,
    /// The `/var/tmp` entries: how many and their total size, when reclaim left any.
    outside: Option<(usize, u64)>,
}

impl Left {
    fn of(shortfall: &Shortfall<'_>) -> Self {
        let var_tmp = shortfall.var_tmp;
        let mut named = Vec::new();
        let mut entries = 0usize;
        let mut rest = false;
        let mut bytes = 0u64;
        for (path, size) in shortfall.left {
            let place = Path::new(path);
            if place == var_tmp {
                // The row of the smaller entries, which carries no count of its own.
                rest = true;
                bytes = bytes.saturating_add(*size);
            } else if place.parent() == Some(var_tmp) {
                entries += 1;
                bytes = bytes.saturating_add(*size);
            } else {
                named.push((redact(path, shortfall), *size));
            }
        }
        let outside = (entries > 0 || rest).then(|| {
            let listed = if rest {
                fs::read_dir(var_tmp).map_or(entries + 1, Iterator::count)
            } else {
                0
            };
            (entries.max(listed), bytes)
        });
        Self { named, outside }
    }

    /// What is left, largest first, each with its size, the `/var/tmp` entries as one.
    fn by_size(&self, shortfall: &Shortfall<'_>) -> Vec<(String, u64)> {
        let mut all = self.named.clone();
        if let Some((count, bytes)) = self.outside {
            all.push((outside(count, shortfall), bytes));
        }
        all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        all
    }
}

/// The `/var/tmp` entries as an item names them: `/var/tmp: 4 entries outside the instance`.
fn outside(count: usize, shortfall: &Shortfall<'_>) -> String {
    let entries = if count == 1 { "entry" } else { "entries" };
    format!(
        "{}: {count} {entries} outside the instance",
        redact(&shortfall.var_tmp.display().to_string(), shortfall)
    )
}

/// Records one operator item for `shortfall`, unless the open fall has one: a decision request
/// raised and escalated as class H, which `decision hands-todo` lists. Answers the item's id, or
/// none when the fall had one already.
///
/// # Errors
///
/// The decision store does not keep the item, or the fall's marker cannot be written.
pub fn escalate(state: &Path, shortfall: &Shortfall<'_>) -> Result<Option<String>> {
    let marker = state.join(FALL);
    if marker.exists() {
        return Ok(None);
    }
    let request_id = format!("disk-{}", eventlog_core::new_event_id());
    let short = gib(shortfall.shortfall);
    let in_repository = shortfall
        .repository
        .map(|repository| format!(" in {repository}"))
        .unwrap_or_default();
    let question = format!(
        "Disk floor not reached: {short}G short for `{}`{in_repository}, which asked {}G, the \
         first build refused in this fall; reclaim freed what needs no decision. What else may \
         go?",
        redact(shortfall.command, shortfall),
        gib(shortfall.bytes)
    );
    let left = Left::of(shortfall);
    let options = if shortfall.left.is_empty() {
        format!(
            "nothing reclaimable is left on {}",
            redact(&shortfall.disk.display().to_string(), shortfall)
        )
    } else {
        let mut options: Vec<String> = left
            .named
            .iter()
            .map(|(path, bytes)| format!("{path} ({}G)", gib(*bytes)))
            .collect();
        if let Some((count, bytes)) = left.outside {
            options.push(format!("{}, {}G", outside(count, shortfall), gib(bytes)));
        }
        options.join("; ")
    };
    let recommendation = person_command(shortfall, &left);
    crate::decide::raise_decision_request(
        Some(state),
        RaiseDecisionRequestArgs {
            request_id: Some(request_id.clone()),
            // One item stands for every refusal of the fall, whatever repository each builds.
            repository: Some(shortfall.instance.to_owned()),
            question: Some(question),
            options: Some(options),
            recommendation: Some(recommendation),
            raised_at: Some(shortfall.at.0.clone()),
            blocker: None,
            input_json: None,
        },
    )?;
    crate::decide::escalate_request(
        Some(state),
        EscalateRequestArgs {
            request_id: Some(request_id.clone()),
            escalation_class: Some("H".to_owned()),
            reason: Some(format!(
                "free disk is {short}G short of the floor and only a person may remove what is \
                 left"
            )),
            input_json: None,
        },
    )?;
    fs::write(&marker, format!("{request_id}\n"))
        .with_context(|| format!("write {}", marker.display()))?;
    Ok(Some(request_id))
}

/// What only a person does: review what is left, largest first, enough to cover the shortfall, and
/// remove what is no longer needed; or, with nothing left, free the shortfall by hand. Never a
/// removal command to paste: what is left is not known to be unneeded.
fn person_command(shortfall: &Shortfall<'_>, left: &Left) -> String {
    if shortfall.left.is_empty() {
        return format!(
            "free {}G on {} by hand, then run `conductor host reclaim`",
            gib(shortfall.shortfall),
            redact(&shortfall.disk.display().to_string(), shortfall)
        );
    }
    let mut covered = 0u64;
    let mut paths = Vec::new();
    for (path, bytes) in left.by_size(shortfall) {
        if covered >= shortfall.shortfall {
            break;
        }
        covered = covered.saturating_add(bytes);
        paths.push(path);
    }
    format!(
        "review and remove what is no longer needed, largest first: {}; then run `conductor host \
         reclaim`",
        paths.join(", ")
    )
}

/// `text` as an operator item writes it: the home directory written `~` wherever it appears as
/// a whole path, and every entry of `/var/tmp` named as `…`, so no record line names a person's
/// home or what another user keeps in `/var/tmp`.
fn redact(text: &str, shortfall: &Shortfall<'_>) -> String {
    let text = match shortfall.home.and_then(Path::to_str) {
        Some(home) => tilde(text, home.trim_end_matches('/')),
        None => text.to_owned(),
    };
    match shortfall.var_tmp.to_str() {
        Some(var_tmp) => entries_hidden(&text, var_tmp.trim_end_matches('/')),
        None => text,
    }
}

/// Whether `c` can be part of a path's name, so a path that continues with it is another one.
fn in_name(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '@')
}

/// Each `(start, end)` of `path` in `text` that stands as a whole path: nothing of a name or a
/// `/` before it, and the end of the text, a `/` or no name after it.
fn whole(text: &str, path: &str) -> Vec<(usize, usize)> {
    if path.is_empty() || path == "/" {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find(path) {
        let start = from + at;
        let end = start + path.len();
        let before = text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !in_name(c) && c != '/');
        let after = text[end..].chars().next().is_none_or(|c| !in_name(c));
        if before && after {
            found.push((start, end));
        }
        from = end;
    }
    found
}

/// `text` with the home directory `home` written `~`.
fn tilde(text: &str, home: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, end) in whole(text, home) {
        out.push_str(&text[last..start]);
        out.push('~');
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

/// `text` with the name of each entry of `var_tmp` written `…`.
fn entries_hidden(text: &str, var_tmp: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, end) in whole(text, var_tmp) {
        if !text[end..].starts_with('/') {
            continue;
        }
        let name = text[end + 1..]
            .find(|c: char| !in_name(c))
            .map_or(text.len(), |at| end + 1 + at);
        if name == end + 1 {
            continue;
        }
        out.push_str(&text[last..start]);
        out.push_str(var_tmp);
        out.push_str("/…");
        last = name;
    }
    out.push_str(&text[last..]);
    out
}

/// Ends the open fall once `free` bytes are free and that is at least `clear`
/// (`thresholds.disk_clear`): withdraws its operator item, if still open (`WithdrawRequest`),
/// and removes the marker, so the next shortfall records a new item. Answers the id of the item
/// withdrawn. The watch's pass calls it with what it reads; it takes the ledger's lock, and
/// leaves the fall to the next pass while a `build run` holds it.
///
/// # Errors
///
/// The ledger or the decision store does not open, or the withdrawal or the marker's removal is
/// not kept.
pub fn clear_fall(state: &Path, free: u64, clear: u64) -> Result<Option<String>> {
    if free < clear || !state.join(FALL).exists() {
        return Ok(None);
    }
    match Ledger::try_lock(state)? {
        Some(ledger) => ledger.clear_fall(free, clear),
        None => Ok(None),
    }
}

impl Ledger {
    /// [`clear_fall`] under this ledger's lock.
    ///
    /// # Errors
    ///
    /// As [`clear_fall`].
    pub fn clear_fall(&self, free: u64, clear: u64) -> Result<Option<String>> {
        if free < clear {
            return Ok(None);
        }
        let marker = self.state.join(FALL);
        let item = match fs::read_to_string(&marker) {
            Ok(text) => text.trim().to_owned(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| format!("read {}", marker.display()));
            }
        };
        let open = !item.is_empty()
            && DecisionRequestStorage::get(
                &crate::store::open(&self.state)?,
                &RequestId(item.clone()),
            )
            .is_some_and(|request| {
                matches!(
                    request.state,
                    DecisionRequestState::Open | DecisionRequestState::Escalated
                )
            });
        if open {
            crate::decide::withdraw_request(
                Some(&self.state),
                WithdrawRequestArgs {
                    request_id: Some(item.clone()),
                    reason: Some(format!(
                        "free disk is back at {}G, at or over disk_clear {}G: the fall is over",
                        free / GIB,
                        gib(clear)
                    )),
                    input_json: None,
                },
            )?;
        }
        match fs::remove_file(&marker) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("remove {}", marker.display()));
            }
        }
        Ok(open.then_some(item))
    }
}

/// `conductor host take-host-grant`: the command `conductor.host.TakeHostGrant`. Prints the id of
/// the grant.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `DiskThroughReservation`.
pub fn take_host_grant(state: Option<&Path>, args: TakeHostGrantArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host take-host-grant";
    flags_only(args.input_json, "host take-host-grant --input-json")?;
    let kind = flag(COMMAND, "--kind", args.kind)?;
    let input = TakeHostGrant {
        instance: InstanceName(flag(COMMAND, "--instance", args.instance)?),
        repository: RepositoryName(flag(COMMAND, "--repository", args.repository)?),
        kind: kind_named(&kind)
            .ok_or_else(|| anyhow!("{COMMAND}: --kind {kind:?} is no GrantKind"))?,
        bytes: args.bytes,
        session_name: SessionName(flag(COMMAND, "--session-name", args.session_name)?),
        requested_at: Timestamp(flag(COMMAND, "--requested-at", args.requested_at)?),
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .take_host_grant(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        TakeHostGrantOutcome::Granted { host_grant_taken } => {
            print_line(COMMAND, &host_grant_taken.grant_id.0)
        }
        TakeHostGrantOutcome::DiskThroughReservation { .. } => bail!(
            "{COMMAND}: DiskThroughReservation: disk is taken through a disk reservation \
             (`conductor build run`), not as a slot; nothing was taken"
        ),
    }
}

/// `conductor host request-disk`: the command `conductor.host.RequestDisk`. Prints the id of the
/// grant.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `InvalidSize`.
pub fn request_disk(state: Option<&Path>, args: RequestDiskArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host request-disk";
    flags_only(args.input_json, "host request-disk --input-json")?;
    let instance = flag(COMMAND, "--instance", args.instance)?;
    let bytes = flag(COMMAND, "--bytes", args.bytes)?;
    let command = flag(COMMAND, "--command", args.command)?;
    let at = Timestamp(flag(COMMAND, "--requested-at", args.requested_at)?);
    let mut ledger = Ledger::lock(&crate::state::dir(state)?)?;
    let bytes = u64::try_from(bytes).map_err(|_| {
        anyhow!(
            "{COMMAND}: InvalidSize: a reservation asks for a positive size, not {bytes} bytes; \
             nothing was requested"
        )
    })?;
    let grant = ledger.request(
        &instance,
        args.repository.as_deref(),
        args.session_name.as_deref(),
        bytes,
        &command,
        &at,
    )?;
    print_line(COMMAND, &grant.0)
}

/// `conductor host reserve-disk`: the command `conductor.host.ReserveDisk`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `InvalidSize`, `HostGrantNotFound`, `HostGrantStateConflict`.
pub fn reserve_disk(state: Option<&Path>, args: ReserveDiskArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host reserve-disk";
    flags_only(args.input_json, "host reserve-disk --input-json")?;
    let input = ReserveDisk {
        grant_id: GrantId(flag(COMMAND, "--grant-id", args.grant_id)?),
        shortfall_bytes: flag(COMMAND, "--shortfall-bytes", args.shortfall_bytes)?,
        reclaimed_bytes: flag(COMMAND, "--reclaimed-bytes", args.reclaimed_bytes)?,
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .reserve_disk(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    reserve_answer(COMMAND, outcome).map(|_| ExitCode::SUCCESS)
}

/// `conductor host start-run`: the command `conductor.host.StartRun`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `NotADiskGrant`, `HostGrantNotFound`, `HostGrantStateConflict`.
pub fn start_run_command(state: Option<&Path>, args: StartRunArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host start-run";
    flags_only(args.input_json, "host start-run --input-json")?;
    let grant = GrantId(flag(COMMAND, "--grant-id", args.grant_id)?);
    let group = flag(COMMAND, "--process-group", args.process_group)?;
    let at = Timestamp(flag(COMMAND, "--started-at", args.started_at)?);
    start_run(&crate::state::dir(state)?, &grant, group, &at)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor host record-peak`: the command `conductor.host.RecordPeak`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `InvalidSize`, `HostGrantNotFound`, `HostGrantStateConflict`.
pub fn record_peak(state: Option<&Path>, args: RecordPeakArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host record-peak";
    flags_only(args.input_json, "host record-peak --input-json")?;
    let input = RecordPeak {
        grant_id: GrantId(flag(COMMAND, "--grant-id", args.grant_id)?),
        peak_bytes: flag(COMMAND, "--peak-bytes", args.peak_bytes)?,
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .record_peak(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    peak_answer(COMMAND, outcome).map(|()| ExitCode::SUCCESS)
}

/// `conductor host release-grant`: the command `conductor.host.ReleaseGrant`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `HostGrantNotFound`, `HostGrantStateConflict`.
pub fn release_grant(state: Option<&Path>, args: ReleaseGrantArgs) -> Result<ExitCode> {
    const COMMAND: &str = "host release-grant";
    flags_only(args.input_json, "host release-grant --input-json")?;
    let grant = GrantId(flag(COMMAND, "--grant-id", args.grant_id)?);
    let reason = match args.reason {
        Some(reason) => Some(
            reason_named(&reason)
                .ok_or_else(|| anyhow!("{COMMAND}: --reason {reason:?} is no ReleaseReason"))?,
        ),
        None => None,
    };
    let at = Timestamp(flag(COMMAND, "--released-at", args.released_at)?);
    release(&crate::state::dir(state)?, &grant, reason, &at)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor host grants`: the view `conductor.host.Grants`, live and recent grants newest first
/// with each run's peak beside its reservation, over the existing store, rendered as `--format`
/// asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a grant does not read, or the rows cannot be written.
pub fn grants(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/host.yaml`).
    const COLUMNS: [&str; 16] = [
        "grant_id",
        "state",
        "instance",
        "repository",
        "kind",
        "bytes",
        "peak_bytes",
        "command",
        "process_group",
        "session_name",
        "reclaimed_bytes",
        "shortfall_bytes",
        "requested_at",
        "started_at",
        "released_at",
        "release_reason",
    ];
    let generated = Generated::new(Ports {
        store: crate::state::open_existing(state)?,
    });
    let mut held = generated
        .grants()
        .map_err(|unmet| anyhow!("host grants: {unmet}"))?;
    held.reverse();
    let rows: Vec<Vec<Value>> = held
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.grant_id.0),
                Value::from(state_name(row.state)),
                Value::from(row.instance.0),
                Value::from(row.repository.map(|name| name.0)),
                Value::from(kind_name(row.kind)),
                Value::from(row.bytes),
                Value::from(row.peak_bytes),
                Value::from(row.command),
                Value::from(row.process_group),
                Value::from(row.session_name.map(|name| name.0)),
                Value::from(row.reclaimed_bytes),
                Value::from(row.shortfall_bytes),
                Value::from(row.requested_at.0),
                Value::from(row.started_at.map(|at| at.0)),
                Value::from(row.released_at.map(|at| at.0)),
                Value::from(row.release_reason.map(reason_name)),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports.store,
        "host grants",
        &view.render(&COLUMNS, &rows),
    )
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
#[must_use]
pub fn state_name(state: HostGrantState) -> &'static str {
    match state {
        HostGrantState::Granted => "Granted",
        HostGrantState::Refused => "Refused",
        HostGrantState::Released => "Released",
        HostGrantState::Requested => "Requested",
        HostGrantState::Running => "Running",
    }
}

/// The state named `name`, as [`state_name`] writes it.
#[must_use]
pub fn state_named(name: &str) -> Option<HostGrantState> {
    [
        HostGrantState::Granted,
        HostGrantState::Refused,
        HostGrantState::Released,
        HostGrantState::Requested,
        HostGrantState::Running,
    ]
    .into_iter()
    .find(|state| state_name(*state) == name)
}

/// The kind's name as the specification spells it.
#[must_use]
pub fn kind_name(kind: GrantKind) -> &'static str {
    match kind {
        GrantKind::ControllerSlot => "ControllerSlot",
        GrantKind::BuildSlot => "BuildSlot",
        GrantKind::Disk => "Disk",
    }
}

/// The kind named `name`, as [`kind_name`] writes it.
#[must_use]
pub fn kind_named(name: &str) -> Option<GrantKind> {
    [
        GrantKind::ControllerSlot,
        GrantKind::BuildSlot,
        GrantKind::Disk,
    ]
    .into_iter()
    .find(|kind| kind_name(*kind) == name)
}

/// The reason's name as the specification spells it.
#[must_use]
pub fn reason_name(reason: ReleaseReason) -> &'static str {
    match reason {
        ReleaseReason::Exited => "Exited",
        ReleaseReason::Killed => "Killed",
        ReleaseReason::ProcessGroupGone => "ProcessGroupGone",
    }
}

/// The reason named `name`, as [`reason_name`] writes it.
#[must_use]
pub fn reason_named(name: &str) -> Option<ReleaseReason> {
    [
        ReleaseReason::Exited,
        ReleaseReason::Killed,
        ReleaseReason::ProcessGroupGone,
    ]
    .into_iter()
    .find(|reason| reason_name(*reason) == name)
}

/// The ports the host commands run over: the store, and the grant id they assign.
struct Ports {
    store: Store,
}

impl HostGrantStorage for Ports {
    fn get(&self, identity: &GrantId) -> Option<HostGrantSnapshot> {
        HostGrantStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: HostGrantSnapshot) {
        HostGrantStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &GrantId) {
        HostGrantStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<HostGrantSnapshot> {
        HostGrantStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_host_grant_id(&mut self) -> Result<GrantId, UnmetObligation> {
        Ok(GrantId(eventlog_core::new_event_id()))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<ReclaimRunId, UnmetObligation> {
        Err(unmet_context("conductor.host.ReclaimRunId"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }
}

/// Refuses `--input-json`, which no handler reads yet, as `what` not implemented.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `name`, or an error naming it.
fn flag<T>(command: &str, name: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| anyhow!("{command}: {name} is missing"))
}

/// Whether the store kept every read and write the behaviour made; an outcome is answered only
/// then.
fn kept(command: &str, store: &Store) -> Result<()> {
    store
        .check()
        .with_context(|| format!("{command}: the store did not keep it"))
}

/// Writes `line` to standard output.
fn print_line(command: &str, line: &str) -> Result<ExitCode> {
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}

fn no_grant(grant: &GrantId) -> String {
    format!("no host grant has the id {}", grant.0)
}
