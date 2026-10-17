//! `conductor host`: the host ledger of slots and disk reservations (`story:build-run`), and the
//! operator item a disk shortfall becomes (`story:disk-escalation`).
//!
//! The ledger is the `conductor.host.HostGrant`s of the store in the host directory, `host/`
//! beside the config file ([`crate::config::host_dir`], `story:instance-isolation`): one ledger
//! for every instance of the file, whatever `--state-dir` says. An instance whose `state` still
//! holds the ledger of an older binary has it moved there once ([`migrate`]). A
//! command reads its input from its flags, opens the store ([`dir`]), runs the
//! generated behaviour, and checks that the store kept what the behaviour reported before it
//! answers, as `conductor resource` does. `request-disk` and `take-host-grant` print the id of
//! the grant on one line. A refused command exits 1 with one line on standard error: the command,
//! the error the specification declares, and what it means. `--input-json` is not read yet, and
//! answers [`NotImplemented`](crate::NotImplemented). `host grants` opens only an existing store
//! and lists the grants newest first.
//!
//! [`Ledger`] is what `conductor build run` ([`crate::build`]) reserves through: it holds the
//! ledger's lock, `host.lock` in the host directory, from the request to its answer, so a second
//! run reserving at the same time counts the first. Nothing expires by time: whenever a run
//! reads the ledger it first releases, with `ProcessGroupGone`, every running grant whose process
//! group is gone ([`Ledger::sweep`]).
//!
//! **Waiting without the lock** (`story:build-queue-convoy`). A run keeps the store it opened for
//! all its passes: [`Ledger::unlock`] lets `host.lock` go and keeps the handle ([`Idle`]), and
//! [`Idle::lock`] takes the lock again without opening the store again; the handle reopens it only
//! when it reads a grant another writer changed. A pass reads the open grants only, from
//! `ledger.open` beside the store, never the closed ones. Every write of a grant bumps
//! `ledger.gen`, a counter written whole ([`generation`]); every sweep renews its age
//! ([`swept_age`]). A run waiting in the queue reads it without the lock, and takes the lock only
//! when it moved, or when the ledger has not been swept for `thresholds.watchdog_every`.
//!
//! **Queue.** Disk requests are answered in arrival order (`story:build-run-queue`): a request
//! with a waiting request made before it ([`Ledger::ahead`]) waits, even when it alone would fit,
//! unless every request ahead of it lacks more than the live reservations hold, so it can only be
//! refused: then one that fits is granted past them ([`Ledger::ahead_bytes`],
//! `story:build-run-backfill`).
//!
//! **Escalation.** A refused reservation records one operator item ([`escalate`]): a decision
//! request raised and escalated as class H, so `decision hands-todo` lists it, naming the
//! shortfall, what reclaim left by path and size, and the one command only a person runs. One item
//! per fall of free disk, whatever command or repository is refused in it
//! (`story:disk-item-per-fall`): `falls/<prefix>.json` in the host directory holds the item's id
//! and its instance, one file per instance (`story:host-fall-per-instance`), and a refusal while
//! the instance's own is there records none. A reservation that fits does not end
//! the fall; free disk back at `thresholds.disk_clear` does, read by the next `build run` or the
//! watch's pass of the item's instance ([`clear_fall`]), which withdraws the item
//! (`WithdrawRequest`) in that instance's store and removes its file; another instance's pass
//! never reads it. A `host.fall` of 0.16.x is read once as the fall of the instance it names and
//! moved to that instance's file. The item is the instance's, not one repository's. It names no personal path
//! (`story:disk-item-redacted`): the entries of `/var/tmp` only as a count and a total size, and
//! the home directory as `~` wherever it appears. A shortfall left only by scratch and caches
//! records none ([`needs_person`], `story:reclaim-scratch-class-c`): freeing them is class C, and
//! reclaim deletes each entry once it is stale and nothing holds it.

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
    ReserveDiskBehavior, StartRunBehavior, TakeHostGrantBehavior, WithdrawDiskRequestBehavior,
};
use conductor_model::host::{
    GrantId, GrantKind, Grants, HostGrantSnapshot, HostGrantState, ReclaimRunId, RecordPeak,
    RecordPeakOutcome, ReleaseGrant, ReleaseGrantOutcome, ReleaseReason, RequestDisk,
    RequestDiskOutcome, ReserveDisk, ReserveDiskOutcome, StartRun, StartRunOutcome, TakeHostGrant,
    TakeHostGrantOutcome, WithdrawDiskRequest, WithdrawDiskRequestOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::Timestamp;
use conductor_model::session::SessionId;
use nix::errno::Errno;
use nix::sys::signal::killpg;
use nix::unistd::Pid;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{
    EscalateRequestArgs, Format, JsonInput, RaiseDecisionRequestArgs, RecordPeakArgs,
    ReleaseGrantArgs, RequestDiskArgs, ReserveDiskArgs, StartRunArgs, TakeHostGrantArgs, ViewArgs,
    WithdrawDiskRequestArgs, WithdrawRequestArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// The ledger's lock, in the host directory.
const LOCK: &str = "host.lock";

/// The ledger's generation, in the host directory: a counter every write of a grant bumps, written
/// whole, and whose age is the time since a pass last swept the ledger (`story:build-queue-convoy`).
const GENERATION: &str = "ledger.gen";

/// The ledger's generation in the host directory `host`, read without the lock: what a run waiting
/// in the queue compares with what it read at its last pass. `0` before any write counted one;
/// `None` when the file does not read.
#[must_use]
pub fn generation(host: &Path) -> Option<String> {
    match fs::read_to_string(host.join(GENERATION)) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some("0\n".to_owned()),
        Err(_) => None,
    }
}

/// How long ago a pass swept the ledger in `host`, or a write changed it: the age of `ledger.gen`.
/// `None` when it is not there or does not read.
#[must_use]
pub fn swept_age(host: &Path) -> Option<std::time::Duration> {
    let modified = fs::metadata(host.join(GENERATION)).ok()?.modified().ok()?;
    Some(
        std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or(std::time::Duration::MAX),
    )
}

/// Bumps the ledger's generation in `host`: the counter plus one, with this writer's mark, so two
/// writers that read one counter without the lock still write two generations; written to a file
/// of its own and renamed over the old one, so a reader reads one whole.
fn bump(host: &Path) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = generation(host)
        .and_then(|text| text.split_whitespace().next()?.parse::<u64>().ok())
        .unwrap_or(0);
    let writer = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let pid = std::process::id();
    let path = host.join(GENERATION);
    let next = host.join(format!("{GENERATION}.{pid}.{writer}.next"));
    let text = format!("{} {pid}.{writer}\n", counter.saturating_add(1));
    if fs::write(&next, text)
        .and_then(|()| fs::rename(&next, &path))
        .is_err()
    {
        let _ = fs::remove_file(&next);
    }
}

/// Renews the age of `ledger.gen` in `host`: a pass swept the ledger. The counter stays.
fn swept(host: &Path) {
    let path = host.join(GENERATION);
    match OpenOptions::new().write(true).open(&path) {
        Ok(file) => {
            let _ = file.set_modified(std::time::SystemTime::now());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Ok(mut file) = OpenOptions::new().write(true).create_new(true).open(&path) {
                let _ = file.write_all(b"0\n");
            }
        }
        Err(_) => {}
    }
}

/// The shared marker of the open fall an older binary (0.16.x) kept in the host directory: the
/// item's id and its instance, on two lines. Read once ([`adopt`]), then moved to [`FALLS`].
const FALL: &str = "host.fall";

/// The directory of the open falls in the host directory: `<prefix>.json` holds one instance's
/// operator item (`story:host-fall-per-instance`).
const FALLS: &str = "falls";

/// One instance's open fall of free disk: its operator item and the instance it is in.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Fall {
    /// The file's name without `.json`: the instance's session prefix, its name without one.
    #[serde(skip)]
    pub key: String,
    /// The instance whose store holds the item.
    pub instance: String,
    /// The operator item's request id.
    pub item: String,
}

/// The key of `instance`'s fall: its session prefix, its name without one.
#[must_use]
pub fn fall_key(instance: &conductor_model::config::Instance) -> String {
    instance
        .session_prefix
        .as_ref()
        .map_or_else(|| instance.name.0.clone(), |prefix| prefix.0.clone())
}

/// The file `key`'s fall is kept in: any character but an ASCII letter or number, `-`, `_` or `.`
/// is written `_`, and no name starts with a dot.
fn fall_file(host: &Path, key: &str) -> PathBuf {
    let mut name: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name.starts_with('.') {
        name.insert(0, '_');
    }
    host.join(FALLS).join(format!("{name}.json"))
}

fn read_fall(path: &Path) -> Option<Fall> {
    let text = fs::read_to_string(path).ok()?;
    let mut fall: Fall = serde_json::from_str(&text).ok()?;
    fall.key = path.file_stem()?.to_str()?.to_owned();
    Some(fall)
}

fn write_fall(host: &Path, key: &str, instance: &str, item: &str) -> Result<()> {
    let path = fall_file(host, key);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let text = serde_json::to_string(&Fall {
        key: key.to_owned(),
        instance: instance.to_owned(),
        item: item.to_owned(),
    })?;
    fs::write(&path, format!("{text}\n")).with_context(|| format!("write {}", path.display()))
}

/// Every open fall in the host directory `host`, by key: what the dashboard shows. A `host.fall`
/// of 0.16.x not moved yet is listed under the name of the instance it names.
#[must_use]
pub fn falls(host: &Path) -> Vec<Fall> {
    let mut out: Vec<Fall> = fs::read_dir(host.join(FALLS))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| read_fall(&entry.path()))
        .collect();
    if let Some((item, instance)) = legacy(host) {
        out.push(Fall {
            key: instance.clone(),
            instance,
            item,
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

/// The item and the instance of an unmoved `host.fall`; a marker of 0.16.x names both.
fn legacy(host: &Path) -> Option<(String, String)> {
    let text = fs::read_to_string(host.join(FALL)).ok()?;
    let mut lines = text.lines().map(str::trim);
    let item = lines.next().filter(|line| !line.is_empty())?;
    let instance = lines.next().filter(|line| !line.is_empty())?;
    Some((item.to_owned(), instance.to_owned()))
}

/// Moves a `host.fall` of 0.16.x that names `instance` to the instance's own file under `key`,
/// once. One that names another instance stays for it. A file of the instance's own wins.
fn adopt(host: &Path, key: &str, instance: &str) -> Result<()> {
    let Some((item, owner)) = legacy(host) else {
        return Ok(());
    };
    if owner != instance {
        return Ok(());
    }
    if !fall_file(host, key).exists() {
        write_fall(host, key, instance, &item)?;
    }
    let old = host.join(FALL);
    match fs::remove_file(&old) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", old.display())),
    }
}

/// What `build run` keeps of the last reclaim run, in the host directory.
const RECLAIM_RECORD: &str = "host.reclaim";

/// The lock one reclaim run holds while it walks the disk, in the host directory.
const RECLAIM_LOCK: &str = "host.reclaim.lock";

/// In an instance's state directory: its ledger was moved to the host directory ([`migrate`]).
const MIGRATED: &str = "host.migrated";

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
    /// The host directory.
    dir: PathBuf,
    /// The grants this handle has read for its passes ([`Ledger::grants_read`]).
    read: std::cell::Cell<usize>,
    _lock: File,
}

/// The ledger's store, kept open between two passes without the ledger's lock
/// (`story:build-queue-convoy`): [`Ledger::unlock`] makes one, [`Idle::lock`] takes the lock again.
pub struct Idle {
    generated: Generated<Ports>,
    dir: PathBuf,
    read: std::cell::Cell<usize>,
}

/// How [`Idle::try_lock`] answered.
pub enum Taken {
    /// The lock is taken.
    Locked(Ledger),
    /// Another holder has it: the handle, still without it.
    Held(Idle),
}

/// The ledger passes this process has made: each [`Ledger::sweep`].
static PASSES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

thread_local! {
    /// The ledger passes this thread has made.
    static PASSES_HERE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many ledger passes this process has made so far: each sweep under `host.lock`
/// (`story:build-queue-convoy` counts them while runs wait).
#[must_use]
pub fn passes() -> usize {
    PASSES.load(std::sync::atomic::Ordering::SeqCst)
}

/// [`passes`] of this thread alone.
#[must_use]
pub fn passes_on_this_thread() -> usize {
    PASSES_HERE.with(std::cell::Cell::get)
}

/// The host directory ([`crate::config::host_dir`]), once the ledger the state directory
/// `--state-dir` or the instance names held before is moved there ([`migrate`]).
///
/// # Errors
///
/// The directories cannot be found, or the move fails.
pub fn dir(state: Option<&Path>) -> Result<PathBuf> {
    let host = crate::config::host_dir()?;
    let state = crate::state::dir(state)?;
    migrate(&state, &host, &crate::config::active().instance.name.0)?;
    Ok(host)
}

/// Moves the host ledger an older binary kept in the instance's state directory `state` to the
/// host directory `host`, once (`story:instance-isolation`): under the ledger's lock, every grant
/// of the instance's store the host's does not hold yet, then deleted from the instance's; the run
/// locks, renamed, so a run of the older binary still holds its own; the reclaim record, unless
/// the host has one; and the fall's marker, naming `instance`, unless the host has one. The old
/// locks are removed, and `host.migrated` in `state` records the move: nothing is moved again. A
/// state directory that is not there holds nothing to move, and nothing is written.
///
/// # Errors
///
/// The ledger, either store, or a file does not move.
pub fn migrate(state: &Path, host: &Path, instance: &str) -> Result<()> {
    let marker = state.join(MIGRATED);
    if marker.exists() || !state.is_dir() || state == host {
        return Ok(());
    }
    let mut ledger = Ledger::lock(host)?;
    if let Ok(mut old) = crate::store::open_existing(state) {
        let grants = HostGrantStorage::list(&old);
        kept("move the host ledger", &old)?;
        for grant in grants {
            let id = grant.data.grant_id.clone();
            if HostGrantStorage::get(&ledger.generated.ports, &id).is_none() {
                HostGrantStorage::put(&mut ledger.generated.ports, grant);
            }
            kept("move the host ledger", &ledger.generated.ports.store)?;
            HostGrantStorage::delete(&mut old, &id);
            kept("move the host ledger", &old)?;
        }
    }
    let runs = state.join(RUNS);
    if let Ok(entries) = fs::read_dir(&runs) {
        let to = host.join(RUNS);
        fs::create_dir_all(&to).with_context(|| format!("create {}", to.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("read {}", runs.display()))?;
            let target = to.join(entry.file_name());
            fs::rename(entry.path(), &target).with_context(|| {
                format!("move {} to {}", entry.path().display(), target.display())
            })?;
        }
        fs::remove_dir(&runs).with_context(|| format!("remove {}", runs.display()))?;
    }
    let record = state.join(RECLAIM_RECORD);
    if record.exists() {
        let target = host.join(RECLAIM_RECORD);
        if target.exists() {
            fs::remove_file(&record).with_context(|| format!("remove {}", record.display()))?;
        } else {
            fs::rename(&record, &target)
                .with_context(|| format!("move {} to {}", record.display(), target.display()))?;
        }
    }
    let fall = state.join(FALL);
    if let Ok(text) = fs::read_to_string(&fall) {
        let target = host.join(FALL);
        let item = text.lines().next().unwrap_or_default().trim();
        if !target.exists() && !item.is_empty() {
            fs::write(&target, format!("{item}\n{instance}\n"))
                .with_context(|| format!("write {}", target.display()))?;
        }
        fs::remove_file(&fall).with_context(|| format!("remove {}", fall.display()))?;
    }
    for lock in [LOCK, RECLAIM_LOCK] {
        let path = state.join(lock);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).with_context(|| format!("remove {}", path.display())),
        }
    }
    fs::write(&marker, format!("{}\n", host.display()))
        .with_context(|| format!("write {}", marker.display()))?;
    Ok(())
}

/// The directory of the run locks: each `build run` holds `<host>/host-runs/<grant>.lock` while it
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

/// What the run lock of a grant says about its `build run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunLock {
    /// A live run holds it.
    Held,
    /// The file is there and nobody holds it: the run that made it is gone.
    Free,
    /// There is no file: a run of a binary before 0.10.6, which locked only from its start, or a
    /// run that ended and removed it.
    Absent,
}

fn run_lock(state: &Path, grant: &GrantId) -> RunLock {
    let path = state.join(RUNS).join(format!("{}.lock", grant.0));
    let Ok(file) = OpenOptions::new().write(true).open(&path) else {
        return RunLock::Absent;
    };
    if file.try_lock().is_err() {
        RunLock::Held
    } else {
        RunLock::Free
    }
}

/// How old a request or grant with no run lock file must be before a sweep takes its requester for
/// gone: a `build run` of a binary before 0.10.6 holds no lock while it waits, and waits at most an
/// hour.
const LEGACY_AGE: time::Duration = time::Duration::hours(3);

/// Whether `requested_at` is older than [`LEGACY_AGE`] at `at`; an instant that does not parse is
/// not old.
fn older_than_legacy_age(requested_at: &Timestamp, at: &Timestamp) -> bool {
    use time::format_description::well_known::Rfc3339;
    match (
        time::OffsetDateTime::parse(&requested_at.0, &Rfc3339),
        time::OffsetDateTime::parse(&at.0, &Rfc3339),
    ) {
        (Ok(requested), Ok(now)) => now - requested > LEGACY_AGE,
        _ => false,
    }
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

/// The ledger's lock file in `state`, opened, and created when it is not there; not locked.
fn lock_file(state: &Path) -> Result<File> {
    let path = state.join(LOCK);
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open the host ledger's lock {}", path.display()))
}

/// Whether another writer holds the writer lock of the store in `state`.
fn tree_held(state: &Path) -> Result<bool> {
    crate::store::tree_held(state)
        .with_context(|| format!("read the store's writer lock in {}", state.display()))
}

impl Ledger {
    /// Takes the ledger's lock in the host directory `state`, waiting for another holder, and
    /// opens the store there.
    ///
    /// `host.lock` is never held while the store's writer lock is waited for
    /// (`story:store-readers-hold-no-writer-lock`): every run of the host queues on `host.lock`, so
    /// one run waiting under it on a store another writer holds would stop them all. The store is
    /// opened first, which takes and lets go the tree's writer lock, without `host.lock`; once
    /// `host.lock` is taken, a tree lock another writer holds makes the run let `host.lock` go
    /// again and wait for the tree lock without it.
    ///
    /// # Errors
    ///
    /// The directory or its lock file cannot be made or locked, or the store does not open.
    pub fn lock(state: &Path) -> Result<Self> {
        Idle::open(state)?.lock()
    }

    /// Lets the ledger's lock go and keeps its store open, for the next pass to take the lock
    /// again without opening the store again ([`Idle::lock`]).
    #[must_use]
    pub fn unlock(self) -> Idle {
        Idle {
            generated: self.generated,
            dir: self.dir,
            read: self.read,
        }
    }

    /// [`Ledger::lock`] without waiting: `None` while another holder has the lock, or another
    /// writer the store's writer lock. The store is opened before `host.lock` is asked for, as in
    /// [`Ledger::lock`].
    ///
    /// # Errors
    ///
    /// The directory or its lock file cannot be made, or the store does not open.
    pub fn try_lock(state: &Path) -> Result<Option<Self>> {
        fs::create_dir_all(state).with_context(|| format!("create {}", state.display()))?;
        if tree_held(state)? {
            return Ok(None);
        }
        let store = crate::store::open(state)?;
        let lock = lock_file(state)?;
        if lock.try_lock().is_err() || tree_held(state)? {
            return Ok(None);
        }
        Ok(Some(Self {
            generated: Generated::new(Ports { store }),
            dir: state.to_owned(),
            read: std::cell::Cell::new(0),
            _lock: lock,
        }))
    }

    /// `StartRun` under this ledger's lock ([`start_run`]).
    ///
    /// # Errors
    ///
    /// The specification refuses it, or the store does not keep it.
    pub fn start_run(&mut self, grant: &GrantId, group: i64, at: &Timestamp) -> Result<()> {
        const COMMAND: &str = "host start-run";
        let generated = &mut self.generated;
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

    /// `ReleaseGrant` under this ledger's lock ([`release`]).
    ///
    /// # Errors
    ///
    /// The specification refuses it, or the store does not keep it.
    pub fn release(
        &mut self,
        grant: &GrantId,
        reason: Option<ReleaseReason>,
        at: &Timestamp,
    ) -> Result<()> {
        release_in(&mut self.generated, grant, reason, at)
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
        PASSES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        PASSES_HERE.with(|count| count.set(count.get() + 1));
        let state = self.dir.clone();
        let grants = self.listed()?;
        let mut requests = Vec::new();
        let mut gone = Vec::new();
        for grant in grants {
            let id = grant.data.grant_id;
            let lock = run_lock(&state, &id);
            if lock == RunLock::Held {
                continue;
            }
            // A request or grant is its requester's until the run is known gone: its lock file is
            // free, or it has none and is older than any waiter of an older binary would wait.
            let requester_gone =
                lock == RunLock::Free || older_than_legacy_age(&grant.data.requested_at, at);
            match grant.state {
                HostGrantState::Requested if requester_gone => {
                    requests.push((id, grant.data.bytes.unwrap_or(1)));
                }
                HostGrantState::Granted if requester_gone => gone.push(id),
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
        swept(&state);
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
        let total = live(&self.listed()?)
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

    /// How many `build run` commands hold a grant on the host, but `except`: the disk grants
    /// `Granted` or `Running`, every instance's, which `thresholds.build_slots` bounds
    /// (`story:build-queue-convoy`).
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn building(&self, except: Option<&GrantId>) -> Result<u64> {
        let count = live(&self.listed()?)
            .filter(|grant| grant.data.kind == GrantKind::Disk)
            .filter(|grant| except != Some(&grant.data.grant_id))
            .count();
        Ok(u64::try_from(count).unwrap_or(u64::MAX))
    }

    /// The largest peak the ledger recorded for a disk reservation in `repository` whose command
    /// `covers`, by the instant its run ended (its request, when it has not), at or after
    /// `since` (`story:reserve-from-peak`). A grant whose peak is unknown, or 0, is no
    /// measurement and is not counted. Answers the peak and how many grants recorded one.
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn largest_peak(
        &self,
        repository: Option<&str>,
        covers: &dyn Fn(&str) -> bool,
        since: time::OffsetDateTime,
    ) -> Result<Option<(u64, usize)>> {
        use time::format_description::well_known::Rfc3339;
        let mut found: Option<(u64, usize)> = None;
        for grant in &HostGrantStorage::list(&self.generated.ports) {
            let data = &grant.data;
            let peak = data
                .peak_bytes
                .and_then(|peak| u64::try_from(peak).ok())
                .filter(|peak| *peak > 0);
            let (Some(peak), Some(command)) = (peak, data.command.as_deref()) else {
                continue;
            };
            let at = data.released_at.as_ref().unwrap_or(&data.requested_at);
            let recent = time::OffsetDateTime::parse(&at.0, &Rfc3339).is_ok_and(|at| at >= since);
            if data.kind == GrantKind::Disk
                && data.repository.as_ref().map(|name| name.0.as_str()) == repository
                && recent
                && covers(command)
            {
                found = Some(found.map_or((peak, 1), |(most, count)| (most.max(peak), count + 1)));
            }
        }
        kept("host ledger", &self.generated.ports.store)?;
        Ok(found)
    }

    /// The open grants a pass reads (`story:build-queue-convoy`), from `ledger.open` beside the
    /// store where the store's files bear it out; each grant
    /// read for them is counted in [`Ledger::grants_read`].
    ///
    /// # Errors
    ///
    /// The store does not read.
    fn listed(&self) -> Result<Vec<HostGrantSnapshot>> {
        let (grants, read) = self.generated.ports.store.open_grants();
        kept("host ledger", &self.generated.ports.store)?;
        self.read.set(self.read.get() + read);
        Ok(grants)
    }

    /// How many grants this handle has read for its passes ([`Ledger::sweep`],
    /// [`Ledger::reserved`], [`Ledger::ahead_bytes`], [`Ledger::session_bound`]) since it was
    /// opened (`story:build-queue-convoy`).
    #[must_use]
    pub fn grants_read(&self) -> usize {
        self.read.get()
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
        self.request_of(instance, repository, session_name, None, bytes, command, at)
    }

    /// [`Ledger::request`], made by the harness session `session_id` when one is named
    /// (`story:build-run-session-bound`): a ledger pass withdraws the request while it waits once
    /// that session is no longer running.
    ///
    /// # Errors
    ///
    /// The specification refuses it (`InvalidSize`), or the store does not keep it.
    #[allow(clippy::too_many_arguments)]
    pub fn request_of(
        &mut self,
        instance: &str,
        repository: Option<&str>,
        session_name: Option<&str>,
        session_id: Option<&str>,
        bytes: u64,
        command: &str,
        at: &Timestamp,
    ) -> Result<GrantId> {
        const COMMAND: &str = "host request-disk";
        let input = RequestDisk {
            instance: InstanceName(instance.to_owned()),
            repository: repository.map(|name| RepositoryName(name.to_owned())),
            session_name: session_name.map(|name| SessionName(name.to_owned())),
            session_id: session_id.map(|id| SessionId(id.to_owned())),
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

    /// How many requests of the host's queue are ahead of `grant` (`story:build-run-queue`): disk
    /// requests still `Requested` whose `build run` holds its run lock ([`hold_run`]), made before
    /// it, by `requested_at` and then by `grant_id`. The head of the queue has none ahead. A
    /// request no run holds is not waiting: the next sweep closes it, or it is a waiter of a binary
    /// before 0.10.6, which does not queue.
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn ahead(&self, grant: &GrantId) -> Result<usize> {
        Ok(self.ahead_bytes(grant)?.len())
    }

    /// The disk requests still `Requested` that name the harness session which made them, with
    /// that session's id (`story:build-run-session-bound`): the ones a ledger pass withdraws once
    /// their session is no longer running.
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn session_bound(&self) -> Result<Vec<(GrantId, String)>> {
        let grants = self.listed()?;
        Ok(grants
            .into_iter()
            .filter(|row| {
                row.state == HostGrantState::Requested && row.data.kind == GrantKind::Disk
            })
            .filter_map(|row| Some((row.data.grant_id, row.data.session_id?.0)))
            .collect())
    }

    /// The session of `grant` when the grant was withdrawn because that session stopped
    /// (`SessionStopped`); `None` for any other grant, state or reason.
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn stopped_session(&self, grant: &GrantId) -> Result<Option<String>> {
        let row = HostGrantStorage::get(&self.generated.ports, grant);
        kept("host ledger", &self.generated.ports.store)?;
        Ok(row.and_then(|row| {
            (row.state == HostGrantState::Withdrawn
                && row.data.release_reason == Some(ReleaseReason::SessionStopped))
            .then_some(row.data.session_id?.0)
        }))
    }

    /// Withdraws `grant`, `SessionStopped`, when it still waits in the queue: a request answered or
    /// withdrawn since the session list was read is left as it is.
    ///
    /// # Errors
    ///
    /// The store does not read, or the specification refuses the move, or the store does not keep
    /// it.
    pub fn withdraw_stopped(&mut self, grant: &GrantId, at: &Timestamp) -> Result<bool> {
        let row = HostGrantStorage::get(&self.generated.ports, grant);
        kept("host ledger", &self.generated.ports.store)?;
        if row.is_none_or(|row| row.state != HostGrantState::Requested) {
            return Ok(false);
        }
        self.withdraw_because(grant, at, Some(ReleaseReason::SessionStopped))?;
        Ok(true)
    }

    /// The bytes each request ahead of `grant` asks ([`Ledger::ahead`]), in arrival order: what
    /// `build run` counts to know whether every one of them can only be refused, so that `grant`
    /// may be granted past them (`story:build-run-backfill`).
    ///
    /// # Errors
    ///
    /// The store does not read.
    pub fn ahead_bytes(&self, grant: &GrantId) -> Result<Vec<u64>> {
        let grants = self.listed()?;
        // A request no longer open is not among the open grants: it is read from the store.
        let own = match grants.iter().find(|row| row.data.grant_id == *grant) {
            Some(own) => own.clone(),
            None => {
                let own = HostGrantStorage::get(&self.generated.ports, grant);
                kept("host ledger", &self.generated.ports.store)?;
                let Some(own) = own else {
                    return Ok(Vec::new());
                };
                own
            }
        };
        let mine = arrival(&own);
        let mut ahead: Vec<&HostGrantSnapshot> = grants
            .iter()
            .filter(|row| {
                row.state == HostGrantState::Requested
                    && row.data.kind == GrantKind::Disk
                    && row.data.grant_id != *grant
                    && arrival(row) < mine
                    && run_lock(&self.dir, &row.data.grant_id) == RunLock::Held
            })
            .collect();
        ahead.sort_by(|a, b| arrival(a).cmp(&arrival(b)));
        Ok(ahead
            .into_iter()
            .map(|row| {
                row.data
                    .bytes
                    .map_or(0, |bytes| u64::try_from(bytes).unwrap_or(0))
            })
            .collect())
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

impl Idle {
    /// Opens the store in the host directory `state` without the ledger's lock, creating both
    /// when they are not there.
    ///
    /// # Errors
    ///
    /// The directory cannot be made, or the store does not open.
    pub fn open(state: &Path) -> Result<Self> {
        fs::create_dir_all(state).with_context(|| format!("create {}", state.display()))?;
        Ok(Self {
            generated: Generated::new(Ports {
                store: crate::store::open(state)?,
            }),
            dir: state.to_owned(),
            read: std::cell::Cell::new(0),
        })
    }

    /// Takes the ledger's lock again, waiting for another holder, over the store this handle kept
    /// open: nothing is opened. As [`Ledger::lock`], `host.lock` is never held while the store's
    /// writer lock is waited for.
    ///
    /// # Errors
    ///
    /// The lock file cannot be made or locked.
    pub fn lock(self) -> Result<Ledger> {
        loop {
            let lock = lock_file(&self.dir)?;
            lock.lock().with_context(|| {
                format!("lock the host ledger {}", self.dir.join(LOCK).display())
            })?;
            if !tree_held(&self.dir)? {
                return Ok(self.locked(lock));
            }
            drop(lock);
            crate::store::wait_for_tree(&self.dir).with_context(|| {
                format!("wait for the store's writer lock in {}", self.dir.display())
            })?;
        }
    }

    /// [`Idle::lock`] without waiting: the handle back, unlocked, while another holder has the lock
    /// or another writer the store's writer lock.
    ///
    /// # Errors
    ///
    /// The lock file cannot be made, or the store's writer lock does not read.
    pub fn try_lock(self) -> Result<Taken> {
        let lock = lock_file(&self.dir)?;
        if lock.try_lock().is_err() || tree_held(&self.dir)? {
            return Ok(Taken::Held(self));
        }
        Ok(Taken::Locked(self.locked(lock)))
    }

    fn locked(self, lock: File) -> Ledger {
        Ledger {
            generated: self.generated,
            dir: self.dir,
            read: self.read,
            _lock: lock,
        }
    }
}

/// Where `grant` stands in arrival order: its `requested_at` as an instant (an instant that does
/// not parse stands first), then its id, a UUIDv7, which orders requests made in one millisecond.
fn arrival(grant: &HostGrantSnapshot) -> (Option<OffsetDateTime>, &str) {
    (
        OffsetDateTime::parse(&grant.data.requested_at.0, &Rfc3339).ok(),
        &grant.data.grant_id.0,
    )
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
    Ledger::lock(state)?.start_run(grant, group, at)
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
    record_run_of(state, grant, peak, 0)
}

/// [`record_peak_of`] with the number of times the watchdog paused the run.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn record_run_of(state: &Path, grant: &GrantId, peak: u64, pauses: u32) -> Result<()> {
    let mut ledger = Ledger::lock(state)?;
    record_peak_in(&mut ledger, grant, peak, pauses)
}

/// [`record_peak_of`] while the run goes on: skipped, answering `false`, when another holder has
/// the ledger's lock, so the watchdog never waits on a reclaim another run holds the lock for.
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn record_peak_now(state: &Path, grant: &GrantId, peak: u64, pauses: u32) -> Result<bool> {
    let Some(mut ledger) = Ledger::try_lock(state)? else {
        return Ok(false);
    };
    record_peak_in(&mut ledger, grant, peak, pauses)?;
    Ok(true)
}

fn record_peak_in(ledger: &mut Ledger, grant: &GrantId, peak: u64, pauses: u32) -> Result<()> {
    const COMMAND: &str = "host record-peak";
    let generated = &mut ledger.generated;
    let outcome = generated
        .record_peak(RecordPeak {
            grant_id: grant.clone(),
            peak_bytes: i64::try_from(peak).unwrap_or(i64::MAX),
            pauses: Some(i64::from(pauses)),
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

/// How a bounded [`finish_run`] ended.
#[derive(Debug)]
pub enum Finish {
    /// The ledger was taken: the grant is released, and `peak` is why recording the run's growth
    /// failed, if it did.
    Released {
        /// The error of recording the peak, if any.
        peak: Option<anyhow::Error>,
    },
    /// The ledger was not taken within the bound, or `stop` answered `true`: the grant stays
    /// Running, for [`Ledger::sweep`] to release once its process group is gone
    /// (`ProcessGroupGone`).
    GaveUp,
}

/// How long [`finish_run`] pauses between two tries of the ledger's lock.
const FINISH_EVERY: std::time::Duration = std::time::Duration::from_millis(25);

/// What a `build run` does once its command stopped: records the run's growth (`peak`, with its
/// pauses) and releases `grant` for `reason`, taking the ledger with a bounded try-lock loop
/// instead of waiting for it (`story:guard-no-pattern-kill`). A holder that never lets go, or a
/// stopping signal (`stop`), must not keep a stopped run from exiting: it gives up after `bound`
/// or when `stop` answers `true`, and leaves the grant to [`Ledger::sweep`].
///
/// # Errors
///
/// The store does not open, or the specification refuses the release.
pub fn finish_run(
    state: &Path,
    grant: &GrantId,
    peak: Option<(u64, u32)>,
    reason: Option<ReleaseReason>,
    clock: impl Fn() -> Timestamp,
    bound: std::time::Duration,
    stop: impl Fn() -> bool,
) -> Result<Finish> {
    let until = std::time::Instant::now() + bound;
    // The store is opened once, while no writer holds its writer lock, and kept across the tries
    // of the lock (`story:build-queue-convoy`): a try that finds the lock held opens nothing.
    let mut idle: Option<Idle> = None;
    loop {
        let handle = match idle.take() {
            Some(handle) => Some(handle),
            None if tree_held(state)? => None,
            None => Some(Idle::open(state)?),
        };
        match handle.map(Idle::try_lock).transpose()? {
            Some(Taken::Locked(mut ledger)) => {
                let peak = peak.and_then(|(bytes, pauses)| {
                    record_peak_in(&mut ledger, grant, bytes, pauses).err()
                });
                release_in(&mut ledger.generated, grant, reason, &clock())?;
                return Ok(Finish::Released { peak });
            }
            Some(Taken::Held(back)) => idle = Some(back),
            None => {}
        }
        if stop() || std::time::Instant::now() >= until {
            return Ok(Finish::GaveUp);
        }
        std::thread::sleep(FINISH_EVERY);
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

/// `WithdrawDiskRequest`: the request `grant`, still waiting in the queue, leaves it at `at`
/// (`story:build-run-cancel-group`).
///
/// # Errors
///
/// The specification refuses it, or the store does not open or keep it.
pub fn withdraw(state: &Path, grant: &GrantId, at: &Timestamp) -> Result<()> {
    let mut ledger = Ledger::lock(state)?;
    ledger.withdraw(grant, at)
}

impl Ledger {
    /// [`withdraw`] under this ledger's lock.
    ///
    /// # Errors
    ///
    /// The specification refuses it, or the store does not keep it.
    pub fn withdraw(&mut self, grant: &GrantId, at: &Timestamp) -> Result<()> {
        self.withdraw_because(grant, at, None)
    }

    /// [`Ledger::withdraw`], recording `reason` (`SessionStopped`,
    /// `story:build-run-session-bound`); a signal's withdrawal records none.
    ///
    /// # Errors
    ///
    /// The specification refuses it, or the store does not keep it.
    pub fn withdraw_because(
        &mut self,
        grant: &GrantId,
        at: &Timestamp,
        reason: Option<ReleaseReason>,
    ) -> Result<()> {
        const COMMAND: &str = "host withdraw-disk-request";
        let outcome = self
            .generated
            .withdraw_disk_request(WithdrawDiskRequest {
                grant_id: grant.clone(),
                withdrawn_at: at.clone(),
                reason,
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        kept(COMMAND, &self.generated.ports.store)?;
        match outcome {
            WithdrawDiskRequestOutcome::Withdrawn { .. } => Ok(()),
            WithdrawDiskRequestOutcome::NoSuchGrant { error } => {
                bail!(
                    "{COMMAND}: HostGrantNotFound: {}",
                    no_grant(&error.grant_id)
                )
            }
            WithdrawDiskRequestOutcome::WrongState { error } => bail!(
                "{COMMAND}: HostGrantStateConflict: grant {} is no request still waiting: it was \
                 granted, refused or withdrawn (see `conductor host grants`)",
                error.grant_id.0
            ),
        }
    }
}

/// The rows of the `Grants` view over the existing store under `state`, newest first. The store
/// is opened as a reader ([`crate::store::open_reader`]): a run that holds `host.lock` while a
/// reader held the store's writer lock would let `host.lock` go and wait ([`Ledger::lock`]), so
/// a reader polling the ledger would hold back the host's queue
/// (`story:store-readers-hold-no-writer-lock`).
///
/// # Errors
///
/// There is no store, it does not open, or a grant does not read.
pub fn rows(state: &Path) -> Result<Vec<Grants>> {
    let generated = Generated::new(Ports {
        store: crate::store::open_reader(state)?,
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
    /// The key of its fall: the instance's session prefix, its name without one.
    pub key: &'a str,
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
    /// The instance's own scratch, `<cache>/scratch`, a root of reclaim's step 3.
    pub scratch: Option<&'a Path>,
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

/// What `shortfall` left as a refused `build run` names it on standard error
/// (`story:build-run-stderr-redacted`), largest first, each with its size: each path outside
/// `/var/tmp` with the home directory written `~`, and the entries of `/var/tmp` as one row,
/// `/var/tmp: 4 entries outside the instance`, as the operator item names them.
#[must_use]
pub fn left_redacted(shortfall: &Shortfall<'_>) -> Vec<(String, u64)> {
    Left::of(shortfall).by_size(shortfall)
}

/// Whether only a person may free what `shortfall` left, so it becomes an operator item: reclaim
/// left nothing (the disk must be freed by hand), or it left something that is no scratch or
/// cache, neither a root [`crate::gc::reclaim::scratch_roots`] names nor under one. Freeing
/// scratch and caches is class C (`story:reclaim-scratch-class-c`): reclaim deletes each entry
/// once it is stale and nothing holds it, and no person is asked to decide it.
#[must_use]
pub fn needs_person(shortfall: &Shortfall<'_>) -> bool {
    let roots =
        crate::gc::reclaim::scratch_roots(shortfall.home, shortfall.var_tmp, shortfall.scratch);
    shortfall.left.is_empty()
        || shortfall
            .left
            .iter()
            .any(|(path, _)| !roots.iter().any(|root| Path::new(path).starts_with(root)))
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
/// raised and escalated as class H in the store under the instance's state directory `state`,
/// which `decision hands-todo` lists. The fall's marker, in the host directory `host`, names the
/// item and its instance. Answers the item's id, or none when the fall had one already.
///
/// # Errors
///
/// The decision store does not keep the item, or the fall's marker cannot be written.
pub fn escalate(host: &Path, state: &Path, shortfall: &Shortfall<'_>) -> Result<Option<String>> {
    adopt(host, shortfall.key, shortfall.instance)?;
    if fall_file(host, shortfall.key).exists() {
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
    // The item's dry run is what conductor measured with no credential: the shortfall, the ask,
    // and what reclaim left (`story:hand-item-dry-run`).
    let dry_run = format!(
        "measured: free disk is {short}G short of the floor; the refused build asked {}G; \
         reclaim left {}",
        gib(shortfall.bytes),
        left_measured(shortfall, &left)
    );
    crate::decide::escalate_request_measured(
        Some(state),
        EscalateRequestArgs {
            request_id: Some(request_id.clone()),
            escalation_class: Some("H".to_owned()),
            reason: Some(format!(
                "free disk is {short}G short of the floor and only a person may remove what is \
                 left"
            )),
            dry_run: None,
            no_dry_run: None,
            kind: None,
            lettered_options: None,
            recommended_letters: None,
            default_letters: None,
            deadline: None,
            input_json: None,
        },
        dry_run,
    )?;
    write_fall(host, shortfall.key, shortfall.instance, &request_id)?;
    Ok(Some(request_id))
}

/// What reclaim left, as a disk item's dry run names it: each row with its size, largest first,
/// or `nothing` (`story:hand-item-dry-run`).
fn left_measured(shortfall: &Shortfall<'_>, left: &Left) -> String {
    let rows: Vec<String> = left
        .by_size(shortfall)
        .into_iter()
        .map(|(path, bytes)| format!("{path} ({}G)", gib(bytes)))
        .collect();
    if rows.is_empty() {
        "nothing".to_owned()
    } else {
        rows.join("; ")
    }
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
/// (`thresholds.disk_clear`): withdraws its operator item, if still open (`WithdrawRequest`), in
/// the store under `state`, the state directory of `instance`, and removes the marker from the
/// host directory `host`, so the next shortfall records a new item. A fall whose item another
/// instance raised is left to that instance. Answers the id of the item withdrawn. The watch's
/// pass calls it with what it reads; it takes the ledger's lock, and leaves the fall to the next
/// pass while a `build run` holds it.
///
/// # Errors
///
/// The ledger or the decision store does not open, or the withdrawal or the marker's removal is
/// not kept.
pub fn clear_fall(
    host: &Path,
    state: &Path,
    key: &str,
    instance: &str,
    free: u64,
    clear: u64,
) -> Result<Option<String>> {
    adopt(host, key, instance)?;
    if free < clear || !fall_file(host, key).exists() {
        return Ok(None);
    }
    match Ledger::try_lock(host)? {
        Some(ledger) => ledger.clear_fall(state, key, instance, free, clear),
        None => Ok(None),
    }
}

impl Ledger {
    /// [`clear_fall`] under this ledger's lock.
    ///
    /// # Errors
    ///
    /// As [`clear_fall`].
    pub fn clear_fall(
        &self,
        state: &Path,
        key: &str,
        instance: &str,
        free: u64,
        clear: u64,
    ) -> Result<Option<String>> {
        adopt(&self.dir, key, instance)?;
        if free < clear {
            return Ok(None);
        }
        // Only this instance's own file is read: another instance's fall is never in reach.
        let marker = fall_file(&self.dir, key);
        let Some(fall) = read_fall(&marker) else {
            return Ok(None);
        };
        let item = fall.item;
        // The item is in the instance's store; under `host.lock` its writer lock is never waited
        // for (`story:store-readers-hold-no-writer-lock`): while another writer holds it, the fall
        // is left to the next run or pass.
        if !item.is_empty() && crate::store::tree_held(state).unwrap_or(true) {
            return Ok(None);
        }
        let open = !item.is_empty()
            && DecisionRequestStorage::get(&crate::store::open(state)?, &RequestId(item.clone()))
                .is_some_and(|request| {
                    matches!(
                        request.state,
                        DecisionRequestState::Open | DecisionRequestState::Escalated
                    )
                });
        if open {
            crate::decide::withdraw_request(
                Some(state),
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
        store: crate::store::open(&dir(state)?)?,
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
    let mut ledger = Ledger::lock(&dir(state)?)?;
    let bytes = u64::try_from(bytes).map_err(|_| {
        anyhow!(
            "{COMMAND}: InvalidSize: a reservation asks for a positive size, not {bytes} bytes; \
             nothing was requested"
        )
    })?;
    let grant = ledger.request_of(
        &instance,
        args.repository.as_deref(),
        args.session_name.as_deref(),
        args.session_id.as_deref(),
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
        store: crate::store::open(&dir(state)?)?,
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
    start_run(&dir(state)?, &grant, group, &at)?;
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
        pauses: args.pauses,
    };
    let mut generated = Generated::new(Ports {
        store: crate::store::open(&dir(state)?)?,
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
    release(&dir(state)?, &grant, reason, &at)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor host withdraw-disk-request`: the command `conductor.host.WithdrawDiskRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep it, or the
/// specification refuses it: `HostGrantNotFound`, `HostGrantStateConflict`.
pub fn withdraw_disk_request(
    state: Option<&Path>,
    args: WithdrawDiskRequestArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "host withdraw-disk-request";
    flags_only(args.input_json, "host withdraw-disk-request --input-json")?;
    let grant = GrantId(flag(COMMAND, "--grant-id", args.grant_id)?);
    let at = Timestamp(flag(COMMAND, "--withdrawn-at", args.withdrawn_at)?);
    let reason = match args.reason {
        Some(reason) => Some(
            reason_named(&reason)
                .ok_or_else(|| anyhow!("{COMMAND}: --reason {reason:?} is no ReleaseReason"))?,
        ),
        None => None,
    };
    Ledger::lock(&dir(state)?)?.withdraw_because(&grant, &at, reason)?;
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
    const COLUMNS: [&str; 18] = [
        "grant_id",
        "state",
        "instance",
        "repository",
        "kind",
        "bytes",
        "peak_bytes",
        "pauses",
        "command",
        "process_group",
        "session_name",
        "session_id",
        "reclaimed_bytes",
        "shortfall_bytes",
        "requested_at",
        "started_at",
        "released_at",
        "release_reason",
    ];
    // A reader, as [`rows`]: it never takes the store's writer lock.
    let generated = Generated::new(Ports {
        store: crate::store::open_reader(&dir(state)?)?,
    });
    let mut held = generated
        .grants()
        .map_err(|unmet| anyhow!("host grants: {unmet}"))?;
    held.reverse();
    let unknown = matches!(view.format, Format::Text | Format::Markdown);
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
                match row.peak_bytes {
                    Some(peak) => Value::from(peak),
                    // A table says what null means for a finished run: it was not measured.
                    None if unknown
                        && row.kind == GrantKind::Disk
                        && row.state == HostGrantState::Released =>
                    {
                        Value::from("unknown")
                    }
                    None => Value::Null,
                },
                Value::from(row.pauses),
                Value::from(row.command),
                Value::from(row.process_group),
                Value::from(row.session_name.map(|name| name.0)),
                Value::from(row.session_id.map(|id| id.0)),
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
        HostGrantState::Withdrawn => "Withdrawn",
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
        HostGrantState::Withdrawn,
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
        ReleaseReason::SessionStopped => "SessionStopped",
    }
}

/// The reason named `name`, as [`reason_name`] writes it.
#[must_use]
pub fn reason_named(name: &str) -> Option<ReleaseReason> {
    [
        ReleaseReason::Exited,
        ReleaseReason::Killed,
        ReleaseReason::ProcessGroupGone,
        ReleaseReason::SessionStopped,
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

    /// Keeps the grant, then `ledger.open` and the ledger's generation (`story:build-queue-convoy`),
    /// in that order: a reader that sees the new generation reads the new open grants.
    fn put(&mut self, snapshot: HostGrantSnapshot) {
        let grant = snapshot.clone();
        HostGrantStorage::put(&mut self.store, snapshot);
        if self.store.check().is_ok() {
            self.store.opened_put(&grant);
            bump(self.store.state_dir());
        }
    }

    fn delete(&mut self, identity: &GrantId) {
        HostGrantStorage::delete(&mut self.store, identity);
        if self.store.check().is_ok() {
            self.store.opened_delete(identity);
            bump(self.store.state_dir());
        }
    }

    fn list(&self) -> Vec<HostGrantSnapshot> {
        HostGrantStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_tool_tool_call_id(
        &mut self,
    ) -> Result<conductor_model::tool::ToolCallId, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.tool.ToolCallId",
        ))
    }

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
