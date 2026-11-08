//! Conductor's store: every accepted command's outcome, so a restarted conductor resumes from
//! records rather than from context.
//!
//! One shared database per deployment (`story:store-shared`, design architecture-next § 8),
//! opened through Eventlog's port with one of two adapters: `eventlog-sqlite` for a deployment
//! that is one host, `eventlog-postgres` for everything else. A URL names it ([`Location`]):
//! `sqlite:<path>` or `postgres://…`. The daemon's `daemon.store` is the one URL; the daemon hands
//! it to every instance runtime it starts in [`STORE_VARIABLE`]. A handle of the instance this
//! process runs, on that instance's state directory, opens the URL the variable names, else the
//! one the config file writes as `daemon.store`, else `<state>/store.sqlite` ([`STORE_FILE`]); a
//! handle on any other state directory (a `--state-dir` of its own, the host's directory) opens
//! `<state>/store.sqlite`.
//!
//! Every event names its instance: on the deployment's database its tenant is the instance's
//! name, and so is its subject; a state directory's own `store.sqlite` holds one instance's
//! records, as `conductor`, so `--state-dir` reads them as written. One database holds every
//! instance's records and the daemon's own, each instance's in one ordered log. Each stored
//! instance is one stream, typed by its entity's qualified name and keyed by its identity. Every
//! write a generated behaviour makes through a storage port appends the
//! instance's new snapshot to that stream, and every read folds the stream back, so a reopened
//! store answers the views from records.
//!
//! The observations (`story:observation-retention`) are kept, each snapshot's, in a tenant of
//! their own, `<instance>/observations/<snapshot>` ([`observations`]); [`Store::retain`] drops the
//! tenants of the snapshots retention does not keep. The `Snapshot` itself is in the instance's
//! tenant, as every other record is.
//!
//! The store before it was eventlog's tree provider under `state/tree/` (and before that its File
//! provider under `state/store/`). `conductor store migrate` reads that tree once into the
//! database ([`migrate`]) and is its only reader afterwards; where a state directory holds a tree
//! and the database holds no migration of it, every command refuses and names that command.
//!
//! The generated storage ports cannot report a failure: `put` and `delete` return nothing, and
//! `get` answers `None` for absent. So a handle keeps every read or write that failed, refuses
//! every write after the first, and answers that first one from [`Store::check`]. An outcome a
//! behaviour reports is durable only when `check` is `Ok` after it. A view answers through
//! [`show`]: every row it could read, then one line for each record it could not.
//!
//! An identity no stream can carry names no stored instance, so a `get` of one answers absent and
//! keeps nothing; a write of one fails.
//!
//! A write expects the stream to be where this handle last read it. A creation decided on an
//! instance read as absent is refused as [`StoreError::Exists`] when another writer created it in
//! the meantime, and an update decided on a stale read as [`StoreError::Moved`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fmt::{self, Write as _};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;
use std::thread;

use anyhow::{Context as _, Result, bail};
use conductor_model::behaviour::{GoalStorage, GuardDecisionStorage, MigrationStorage};
use conductor_model::config::InstanceName;
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use conductor_model::dispatch::{
    GuardDecisionData, GuardDecisionId, GuardDecisionSnapshot, GuardDecisionState, Verdict,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::store::Backend;
use eventlog_core::{
    AppendGroup, AtomicEventStore, CommandMeta, EventLogError, Expected, MAX_READ_LIMIT, NewEvent,
    RecordedEvent, StreamAppend, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_postgres::PostgresEventStore;
use eventlog_sqlite::SqliteEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::runtime::{Builder, Handle, Runtime};

use crate::config;

// The storage ports of the later stories, one child module each, so that units running at once
// never edit this file. A child module reaches the private helpers below (`read`, `write`, `fail`).
mod board;
mod config_revision;
pub use config_revision::{level_name, level_named};
mod controller;
mod decision;
pub(crate) mod dispatch;
mod host;
mod improvement;
mod marks;
pub mod migrate;
pub mod observations;
mod placement;
pub mod proposal;
mod reclaim;
mod reconciliation;
mod secret;
pub use secret::source_name;
pub(crate) mod run;
pub mod series;
mod serving;
mod session;
mod snapshot;
mod token;
mod tool;
pub use token::{grant_name, token_state_name, usage_name};

/// The variable the daemon hands every instance runtime it starts the store's URL in
/// (`daemon.store`).
pub const STORE_VARIABLE: &str = "CONDUCTOR_STORE";

/// The SQLite file under a state directory a handle opens when no URL names the store.
pub const STORE_FILE: &str = "store.sqlite";

/// The table prefix of conductor's tables in the database: one owner, every instance in it.
pub const PREFIX: &str = "conductor";

/// The directory under `state/` the tree store of every record but the observations lived in,
/// before `conductor store migrate`.
const TREE_DIR: &str = "tree";

/// The directory under `state/` the observations lived in, one tenant per snapshot.
const OBSERVATIONS_DIR: &str = "observations";

/// The directory under `state/` the daily rollups lived in ([`series`]).
const SERIES_DIR: &str = "series";

/// The directory under `state/` the File provider's store lived in, before the tree.
const FILE_STORE_DIR: &str = "store";

/// The File provider's commit point: a store directory holding it holds that provider's store.
const FILE_STORE_MANIFEST: &str = "manifest.json";

/// The tree store's own description: a directory holding it holds a tree store.
const TREE_MANIFEST: &str = "store.json";

/// The tree store's writer lock, one writer at a time (eventlog-tree's layout).
const TREE_LOCK: &str = ".lock";

/// The tenant, subject and actor of a store on a state directory of no instance, and the one
/// tenant of the tree before the database.
const CONDUCTOR: &str = "conductor";

/// The event that holds an instance's snapshot after a write.
const STORED: &str = "stored";

/// The event that ends an instance: the stream reads as absent after it.
const DELETED: &str = "deleted";

/// The version of every event body this module writes.
const SCHEMA_VERSION: u32 = 1;

/// `conductor.direction.Goal`.
const GOAL: &str = "conductor.direction.Goal";

/// `conductor.dispatch.GuardDecision`.
const GUARD_DECISION: &str = "conductor.dispatch.GuardDecision";

/// Where the store is: the database a URL names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// `sqlite:<path>`: one file, on one host.
    Sqlite(PathBuf),
    /// `postgres://…`: a server every box reaches.
    Postgres(String),
}

impl Location {
    /// The location `url` names: `sqlite:<path>` (a path under `~/` is under the home directory)
    /// or `postgres://…`.
    ///
    /// # Errors
    ///
    /// `url` is neither, names no path, or writes its password as `secret:<name>`: the daemon
    /// resolves the secret and hands the runtime the URL in [`STORE_VARIABLE`].
    pub fn parse(url: &str) -> Result<Self> {
        if let Some(path) = url.strip_prefix("sqlite:") {
            let path = path
                .strip_prefix("//")
                .filter(|rest| rest.starts_with('/'))
                .unwrap_or(path);
            if path.is_empty() {
                bail!("the store URL {url:?} names no file: write sqlite:<path>");
            }
            let path = match path.strip_prefix("~/") {
                Some(rest) => PathBuf::from(
                    env::var_os("HOME").context("the store URL is under ~/ and HOME is not set")?,
                )
                .join(rest),
                None => PathBuf::from(path),
            };
            return Ok(Self::Sqlite(path));
        }
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            if url.contains(":secret:") {
                bail!(
                    "the store URL {} writes its password as secret:<name>, which only the daemon \
                     resolves: it hands the runtime the URL in {STORE_VARIABLE}",
                    redacted(url)
                );
            }
            return Ok(Self::Postgres(url.to_owned()));
        }
        bail!(
            "{:?} is not a store URL: write sqlite:<path> or postgres://…, its password a \
             secret:<name>",
            redacted(url)
        )
    }

    /// The backend: `Sqlite` or `Postgres`.
    #[must_use]
    pub fn backend(&self) -> Backend {
        match self {
            Self::Sqlite(_) => Backend::Sqlite,
            Self::Postgres(_) => Backend::Postgres,
        }
    }

    /// The backend's name as `conductor doctor` writes it: `sqlite` or `postgres`.
    #[must_use]
    pub fn backend_name(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }

    /// Where it is, as a person may read it: the SQLite file, or the Postgres URL without its
    /// password.
    #[must_use]
    pub fn shown(&self) -> String {
        match self {
            Self::Sqlite(path) => path.display().to_string(),
            Self::Postgres(url) => redacted(url),
        }
    }

    /// The URL, as `daemon.store` writes it.
    #[must_use]
    pub fn url(&self) -> String {
        match self {
            Self::Sqlite(path) => format!("sqlite:{}", path.display()),
            Self::Postgres(url) => url.clone(),
        }
    }
}

/// `url` with the password of its user, if any, written `***`.
fn redacted(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let Some((user, host)) = rest.split_once('@') else {
        return url.to_owned();
    };
    match user.split_once(':') {
        Some((name, _)) => format!("{scheme}://{name}:***@{host}"),
        None => url.to_owned(),
    }
}

/// Whether the store at `location` answers, without creating anything in a SQLite file that is
/// not there: `Err` with why it does not.
///
/// # Errors
///
/// Why it does not answer, on one line.
pub fn answers(location: &Location) -> Result<(), String> {
    let runtime = Engine::start().map_err(|error| format!("{error:#}"))?;
    let opened = match location {
        Location::Sqlite(path) => {
            if !is_file(path) {
                return Err(format!("no database at {}", path.display()));
            }
            let path = path.to_string_lossy().into_owned();
            runtime
                .block_on(SqliteEventStore::open_existing(&path, PREFIX))
                .map(|_| ())
        }
        Location::Postgres(url) => runtime
            .block_on(PostgresEventStore::connect(url, PREFIX))
            .map(|_| ()),
    };
    opened.map_err(|error| one_line(&error.to_string()))
}

/// `text` on one line.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// How a handle opens its store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Creates the database and its tables where they are not there.
    Create,
    /// Opens only a database that is there.
    Existing,
    /// As [`Mode::Existing`], for a handle that never writes.
    Reader,
}

/// The store a handle on `state` opens, and the instance it writes as: the instance this process
/// runs, on that instance's own state directory, at the URL [`STORE_VARIABLE`] names, else the
/// config file's `daemon.store`. Otherwise the state directory's own `<state>/store.sqlite`, as
/// [`CONDUCTOR`]: a file only that state directory writes, so `--state-dir` reads it as written.
///
/// # Errors
///
/// The URL is no store URL.
pub fn located(state: &Path) -> Result<(Location, String)> {
    if let Some(active) = config::active_if_set()
        && active.from_file
        && same_dir(state, Path::new(&active.instance.state))
    {
        let instance = active.instance.name.0.clone();
        if let Some(url) = handed_or_written() {
            return Ok((Location::parse(&url)?, instance));
        }
    }
    Ok((
        Location::Sqlite(state.join(STORE_FILE)),
        CONDUCTOR.to_owned(),
    ))
}

/// Whether `one` and `other` name one directory.
fn same_dir(one: &Path, other: &Path) -> bool {
    one == other || config::physical(one) == config::physical(other)
}

/// The URL [`STORE_VARIABLE`] names, else the `daemon.store` the config file this process read
/// writes; none when neither does. A file that writes no `daemon.store` keeps each instance's
/// store under its state directory.
fn handed_or_written() -> Option<String> {
    if let Some(url) = env::var(STORE_VARIABLE).ok().filter(|url| !url.is_empty()) {
        return Some(url);
    }
    static WRITTEN: OnceLock<Option<String>> = OnceLock::new();
    WRITTEN
        .get_or_init(|| {
            let text = fs::read_to_string(config::config_file()?).ok()?;
            let file: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
            file.get("daemon")?
                .get("store")?
                .as_str()
                .map(str::to_owned)
        })
        .clone()
}

/// Opens the store of `state` ([`located`]), creating the database and its tables when they are
/// not there.
///
/// # Errors
///
/// `state` holds an eventlog tree (or File store) the database holds no migration of: nothing is
/// written, and the one line names `conductor store migrate`. Or the URL is no store URL, or the
/// database does not open.
pub fn open(state: &Path) -> Result<Store> {
    opened_on(state, Mode::Create)
}

/// Opens the store of `state` ([`located`]) only when its database is there: nothing is created,
/// so a read never plants a store.
///
/// # Errors
///
/// As [`open`]; or the database is not there (one line naming it).
pub fn open_existing(state: &Path) -> Result<Store> {
    opened_on(state, Mode::Existing)
}

/// [`open_existing`] for a handle that only reads, as the dashboard's does. It never writes.
///
/// # Errors
///
/// As [`open_existing`].
pub fn open_reader(state: &Path) -> Result<Store> {
    opened_on(state, Mode::Reader)
}

/// Opens the store at `url` as `instance`, creating the database and its tables when they are not
/// there: what the daemon opens, and what it hands every runtime.
///
/// # Errors
///
/// `url` is no store URL, `instance` is no tenant eventlog holds, or the database does not open.
pub fn open_url(url: &str, instance: &str) -> Result<Store> {
    let location = Location::parse(url)?;
    let state = match &location {
        Location::Sqlite(path) => path.parent().map_or_else(PathBuf::new, Path::to_owned),
        Location::Postgres(_) => env::current_dir().unwrap_or_default(),
    };
    Store::opened(&state, location, instance, Mode::Create)
}

/// [`open`], [`open_existing`] or [`open_reader`], by `mode`.
fn opened_on(state: &Path, mode: Mode) -> Result<Store> {
    let (location, instance) = located(state)?;
    let tree = holds_tree(state);
    if tree
        && let Location::Sqlite(path) = &location
        && !is_file(path)
    {
        bail!(unmigrated_line(state));
    }
    let store = Store::opened(state, location, &instance, mode)?;
    if tree && !store.migrated() {
        bail!(unmigrated_line(state));
    }
    Ok(store)
}

/// [`open`] for `conductor store migrate`: on a state directory whose tree is not yet read.
fn open_for_migration(state: &Path) -> Result<Store> {
    let (location, instance) = located(state)?;
    Store::opened(state, location, &instance, Mode::Create)
}

/// Whether `state` holds an eventlog tree (or a split of one stopped between its renames), or an
/// eventlog-file store, of the builds before the database.
fn holds_tree(state: &Path) -> bool {
    is_file(&state.join(TREE_DIR).join(TREE_MANIFEST))
        || is_file(&state.join(migrate::NEXT_DIR).join(TREE_MANIFEST))
        || fs::symlink_metadata(state.join(FILE_STORE_DIR).join(FILE_STORE_MANIFEST)).is_ok()
}

/// The one line every command but `store migrate` refuses with on a state directory whose tree
/// is not read into the database yet.
fn unmigrated_line(state: &Path) -> String {
    format!(
        "{} holds an eventlog tree the store has not read: run `conductor store migrate`",
        state.display()
    )
}

/// Whether `path` is a regular file, not following a final link.
fn is_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Holds the lock file at `path`, creating it when it is not there, until the answer is dropped.
fn hold(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

/// Whether another writer holds the store's writer lock right now: never, since the database
/// serializes its writers itself. Kept for the callers that waited on the tree's lock.
///
/// # Errors
///
/// None.
pub fn tree_held(_state: &Path) -> io::Result<bool> {
    Ok(false)
}

/// Waits until no other writer holds the store's writer lock: at once, since the database
/// serializes its writers itself.
///
/// # Errors
///
/// None.
pub fn wait_for_tree(_state: &Path) -> io::Result<()> {
    Ok(())
}

/// The files whose length and modification time move when the store of `state` is written: the
/// SQLite file and its write-ahead log; none for a Postgres store. What the dashboard
/// fingerprints its answers by.
#[must_use]
pub fn written_files(state: &Path) -> Vec<PathBuf> {
    match located(state) {
        Ok((Location::Sqlite(path), _)) => {
            let mut wal = path.clone().into_os_string();
            wal.push("-wal");
            vec![path, PathBuf::from(wal)]
        }
        _ => Vec::new(),
    }
}

/// Whether the store of `state` is there: its SQLite file, or any Postgres store.
#[must_use]
pub fn present(state: &Path) -> bool {
    match located(state) {
        Ok((Location::Sqlite(path), _)) => is_file(&path),
        Ok((Location::Postgres(_), _)) => true,
        Err(_) => false,
    }
}

/// An I/O failure as the log reports one.
fn backend(error: &io::Error) -> EventLogError {
    EventLogError::Backend(error.to_string())
}

/// Answers a view whose query has run over `store`: `rendered`, the rows it could read, on
/// standard output, then one line on standard error for each record it could not read, naming
/// the record's stream. Exits 0 when every record was read, and 1 when one was not. `what` names
/// the rows in the messages ("the goals").
///
/// # Errors
///
/// A read failed for a reason other than a record this build cannot read; nothing is written
/// then. Or standard output cannot be written.
pub fn show(store: &Store, what: &str, rendered: &str) -> Result<ExitCode> {
    let unreadable = store
        .unreadable()
        .with_context(|| format!("read the {what}"))?;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(rendered.as_bytes())
        .and_then(|()| stdout.flush())
        .with_context(|| format!("write the {what} to standard output"))?;
    if unreadable.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let mut stderr = io::stderr().lock();
    for record in &unreadable {
        writeln!(stderr, "conductor: read the {what}: {record}")
            .with_context(|| format!("name an unreadable record among the {what}"))?;
    }
    Ok(ExitCode::FAILURE)
}

/// The store's own runtime, safe on a thread that is already inside a tokio runtime (an async
/// handler, a `#[tokio::main]` binary), where tokio panics on a second runtime's `block_on` and on
/// dropping one.
struct Engine(Option<Runtime>);

impl Engine {
    fn start() -> Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("start the store's runtime")?;
        Ok(Self(Some(runtime)))
    }

    /// Runs `future` to completion on the store's runtime: on this thread, or on a thread of its
    /// own while this one is inside another runtime.
    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future + Send,
        F::Output: Send,
    {
        let runtime = self.0.as_ref().expect("the runtime is taken only on drop");
        if Handle::try_current().is_err() {
            return runtime.block_on(future);
        }
        thread::scope(|scope| {
            scope
                .spawn(|| runtime.block_on(future))
                .join()
                .unwrap_or_else(|payload| panic::resume_unwind(payload))
        })
    }
}

impl Drop for Engine {
    /// Inside another runtime the store's runtime may not wait for its threads, so it lets them
    /// finish on their own; nothing is in flight once every `block_on` has returned.
    fn drop(&mut self) {
        let Some(runtime) = self.0.take() else {
            return;
        };
        if Handle::try_current().is_ok() {
            runtime.shutdown_background();
        }
    }
}

/// Conductor's records, behind the storage ports the generated behaviours and queries use.
pub struct Store {
    /// The log, dropped before the runtime it was opened on.
    log: Box<dyn AtomicEventStore>,
    runtime: Engine,
    /// The state directory the handle was opened on.
    state: PathBuf,
    /// The database.
    location: Location,
    /// The instance every event of this handle names.
    instance: String,
    /// The instance's tenant: every record but the observations.
    tenant: TenantId,
    /// The stream version this handle last read or wrote, per `(entity, identity)`; `0` is a
    /// stream that did not exist. What the next write to it expects.
    versions: RefCell<HashMap<(&'static str, String), u64>>,
    /// Every read or write that failed on this handle, in the order they failed; a record that
    /// does not decode is kept once however often it is read.
    failures: RefCell<Vec<StoreError>>,
    /// The observation records this handle holds back, while [`Store::hold_observations`] is on,
    /// to commit them together ([`Store::commit_observations`]); `None` while it writes each at
    /// once.
    held_back: RefCell<Option<Vec<HeldBack>>>,
    /// The feed position of the instance's tenant this handle read snapshots up to, and the
    /// tenant of each snapshot it met there, in the order met ([`Store::observation_tenants`]).
    snapshot_tenants: RefCell<(u64, Vec<TenantId>)>,
}

/// One observation stream's records held back, in the order written, to commit with the others.
struct HeldBack {
    /// The entity and identity, as [`Store::versions`] keys them.
    key: (&'static str, String),
    /// The stream, in the tenant of the observation's snapshot.
    stream: StreamId,
    /// The stream version the first of them expects.
    read: u64,
    /// The records.
    events: Vec<NewEvent>,
}

/// What a handle holds of its store in memory (`story:dashboard-memory-bound`), as
/// [`Store::held`] reports it: one open database, whose rows stay in it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Replays {
    /// The stores held open now.
    pub live: usize,
    /// The most open at once since the handle opened.
    pub peak: usize,
    /// The bytes of records held in memory: none, the database keeps them.
    pub bytes: u64,
}

thread_local! {
    /// The stores this thread opened.
    static REPLAYS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How long each open waits on a slow disk ([`slow_replays`]), in nanoseconds; 0 for none.
static SLOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The one open a slow disk serves at a time ([`slow_replays`]).
static DISK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// How many stores this thread has opened so far (`story:build-queue-convoy` counts a run's).
#[must_use]
pub fn replays_on_this_thread() -> usize {
    REPLAYS.with(std::cell::Cell::get)
}

/// A slow disk, for a case that measures what opens cost: from now on every open of a store in
/// this process takes `delay` more, one at a time. `Duration::ZERO` ends it. Nothing but a case
/// calls it.
#[doc(hidden)]
pub fn slow_replays(delay: std::time::Duration) {
    SLOW.store(
        u64::try_from(delay.as_nanos()).unwrap_or(u64::MAX),
        std::sync::atomic::Ordering::SeqCst,
    );
}

thread_local! {
    /// Whether the log refuses every write of this thread ([`refuse_writes`]).
    static REFUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A log that refuses writes, for a case that checks how a failed write is reported: from now on
/// every write a handle makes on this thread fails as the database would fail it, until `false`.
/// Nothing but a case calls it.
#[doc(hidden)]
pub fn refuse_writes(refuse: bool) {
    REFUSED.with(|refused| refused.set(refuse));
}

/// The failure of a write this thread refuses ([`refuse_writes`]), if it does.
fn refused_write() -> Option<EventLogError> {
    REFUSED
        .with(std::cell::Cell::get)
        .then(|| EventLogError::Backend("the database refused the write".to_owned()))
}

/// Counts one open on this thread, and waits as the slow disk of [`slow_replays`] makes it.
fn replayed() {
    REPLAYS.with(|count| count.set(count.get() + 1));
    let slow = SLOW.load(std::sync::atomic::Ordering::SeqCst);
    if slow > 0 {
        let _disk = DISK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        thread::sleep(std::time::Duration::from_nanos(slow));
    }
}

/// Why the store could not keep or read a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// A creation found the instance already stored: another writer created it after this handle
    /// read it as absent.
    Exists {
        /// The entity's qualified name.
        entity: &'static str,
        /// The instance's identity.
        identity: String,
    },
    /// The instance changed after this handle read it.
    Moved {
        /// The entity's qualified name.
        entity: &'static str,
        /// The instance's identity.
        identity: String,
        /// The stream version this handle read.
        read: u64,
        /// The stream version the store holds.
        held: u64,
    },
    /// A stored record does not decode into the generated type.
    Undecodable {
        /// The entity's qualified name.
        entity: &'static str,
        /// The stream the record is in.
        stream: String,
        /// What did not decode.
        detail: String,
    },
    /// The log refused the call or failed.
    Log {
        /// The entity's qualified name.
        entity: &'static str,
        /// The instance's identity; `None` for a read of every instance.
        identity: Option<String>,
        /// What the log answered.
        source: EventLogError,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exists { entity, identity } => {
                write!(f, "{entity} {identity:?} already exists in the store")
            }
            Self::Moved {
                entity,
                identity,
                read,
                held,
            } => write!(
                f,
                "{entity} {identity:?} moved to version {held} after this handle read version {read}"
            ),
            Self::Undecodable {
                entity,
                stream,
                detail,
            } => write!(
                f,
                "{entity} record in stream {stream:?} does not decode: {detail}"
            ),
            Self::Log {
                entity,
                identity: Some(identity),
                source,
            } => write!(f, "the store failed on {entity} {identity:?}: {source}"),
            Self::Log {
                entity,
                identity: None,
                source,
            } => write!(f, "the store failed listing {entity}: {source}"),
        }
    }
}

impl StoreError {
    /// Whether `self` and `other` are one failure: the same record that does not decode, whatever
    /// read met it and said why, or otherwise equal.
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Undecodable { entity, stream, .. },
                Self::Undecodable {
                    entity: other_entity,
                    stream: other_stream,
                    ..
                },
            ) => entity == other_entity && stream == other_stream,
            _ => self == other,
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Log { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Store {
    /// The store at `location`, as `instance`, on the state directory `state`.
    fn opened(state: &Path, location: Location, instance: &str, mode: Mode) -> Result<Self> {
        let runtime = Engine::start()?;
        let tenant = TenantId::new(instance)
            .with_context(|| format!("the instance {instance:?} names no tenant of the store"))?;
        let shown = location.shown();
        replayed();
        let log: Box<dyn AtomicEventStore> = match &location {
            Location::Sqlite(path) => {
                let text = path.to_string_lossy().into_owned();
                if mode != Mode::Create && !is_file(path) {
                    bail!("no store at {shown}");
                }
                if let Some(parent) = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create {}", parent.display()))?;
                }
                // Opening sets the database's journal mode and creates its tables, which a second
                // opener at the same instant finds locked: the openers of one file take turns.
                let mut lock = path.clone().into_os_string();
                lock.push(".open-lock");
                let _turn = hold(Path::new(&lock))
                    .with_context(|| format!("take the opening turn of {shown}"))?;
                if mode == Mode::Create {
                    Box::new(
                        runtime
                            .block_on(SqliteEventStore::open(&text, PREFIX))
                            .with_context(|| format!("open the store at {shown}"))?,
                    )
                } else {
                    Box::new(
                        runtime
                            .block_on(SqliteEventStore::open_existing(&text, PREFIX))
                            .with_context(|| format!("open the store at {shown}"))?,
                    )
                }
            }
            Location::Postgres(url) => Box::new(
                runtime
                    .block_on(PostgresEventStore::connect(url, PREFIX))
                    .with_context(|| format!("open the store at {shown}"))?,
            ),
        };
        Ok(Self {
            log,
            runtime,
            state: state.to_owned(),
            location,
            instance: instance.to_owned(),
            tenant,
            versions: RefCell::default(),
            failures: RefCell::default(),
            held_back: RefCell::default(),
            snapshot_tenants: RefCell::default(),
        })
    }

    /// Whether the database holds the migration of this handle's instance's tree.
    fn migrated(&self) -> bool {
        MigrationStorage::get(self, &InstanceName(self.instance.clone())).is_some()
    }

    /// What this handle holds of its store in memory: one open database, none of its records.
    #[must_use]
    pub fn held(&self) -> Replays {
        Replays {
            live: 1,
            peak: 1,
            bytes: 0,
        }
    }

    /// The state directory this handle was opened on.
    #[must_use]
    pub fn state_dir(&self) -> &Path {
        &self.state
    }

    /// The database this handle opened.
    #[must_use]
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// The instance every event this handle writes names.
    #[must_use]
    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// Whether every read and write on this handle since it was opened succeeded.
    ///
    /// # Errors
    ///
    /// The first [`StoreError`] on this handle. Every write after it was refused, so nothing a
    /// behaviour reported since then was kept.
    pub fn check(&self) -> Result<(), StoreError> {
        match self.failures.borrow().first() {
            Some(failure) => Err(failure.clone()),
            None => Ok(()),
        }
    }

    /// Every record this handle met and could not read ([`StoreError::Undecodable`]), each once,
    /// in the order met, when nothing else failed on it: what a view names beside its rows.
    ///
    /// # Errors
    ///
    /// The first failure on this handle that is not a record this build cannot read.
    pub fn unreadable(&self) -> Result<Vec<StoreError>, StoreError> {
        let failures = self.failures.borrow();
        match failures
            .iter()
            .find(|failure| !matches!(failure, StoreError::Undecodable { .. }))
        {
            Some(failure) => Err(failure.clone()),
            None => Ok(failures.clone()),
        }
    }

    /// Runs `read` over this handle and takes back every failure it kept, answering them beside
    /// its result: the handle is left as it was, so a read beside a sense run whose records do
    /// not decode neither stops the run's writes nor fails its [`Store::check`]. What a second
    /// handle kept apart, without opening the store a second time (`story:snapshot-store-cost`).
    pub fn apart<T>(&self, read: impl FnOnce(&Self) -> T) -> (T, Vec<StoreError>) {
        let before = self.failures.borrow().len();
        let answer = read(self);
        let met = self.failures.borrow_mut().split_off(before);
        (answer, met)
    }

    /// Keeps `failure`, unless it is already kept: an unreadable record is the same failure
    /// whatever read met it.
    fn fail(&self, failure: StoreError) {
        let mut failures = self.failures.borrow_mut();
        if !failures.iter().any(|kept| kept.same(&failure)) {
            failures.push(failure);
        }
    }

    /// The stream of `entity`'s instance `identity` in `tenant`. An identity whose escaped form
    /// eventlog refuses as a stream id names no stream.
    fn stream(
        tenant: &TenantId,
        entity: &'static str,
        identity: &str,
    ) -> Result<StreamId, StoreError> {
        let failed = |source| log_error(entity, Some(identity), source);
        let escaped = escaped(identity);
        StreamId::new(tenant.clone(), entity, escaped.clone()).map_err(failed)?;
        StreamId::new(tenant.clone(), entity, held(&escaped).map_err(failed)?).map_err(failed)
    }

    /// The newest snapshot body of one instance, or `None` when it is absent, deleted or the read
    /// failed (kept for [`Store::check`]). An identity no stream can carry names no stored
    /// instance: it reads as absent, and nothing is kept. An observation is read from the tenant
    /// that holds it.
    fn read(&self, entity: &'static str, identity: &str) -> Option<Value> {
        let held = if observations::holds(entity) {
            self.locate(entity, identity)
                .map(|found| found.map_or((0, None), |(_, version, body)| (version, body)))
        } else {
            let stream = Self::stream(&self.tenant, entity, identity).ok()?;
            self.latest(entity, identity, &stream)
        };
        match held {
            Ok((version, body)) => {
                self.versions
                    .borrow_mut()
                    .insert((entity, identity.to_owned()), version);
                body
            }
            Err(failure) => {
                self.fail(failure);
                None
            }
        }
    }

    /// The tenant of the observations of the snapshot `snapshot`.
    fn observation_tenant(&self, snapshot: &str) -> Result<TenantId, EventLogError> {
        observations::tenant(&self.instance, snapshot)
    }

    /// The tenants that may hold observations: one per snapshot the instance's tenant holds a
    /// stream of, the newest first. The feed is read from where this handle last read it, so a
    /// read of an observation does not read the whole store again.
    fn observation_tenants(&self, entity: &'static str) -> Result<Vec<TenantId>, StoreError> {
        let failed = |source| log_error(entity, None, source);
        let mut seen = self.snapshot_tenants.borrow_mut();
        loop {
            let page = self
                .runtime
                .block_on(self.log.read_feed(&self.tenant, seen.0, MAX_READ_LIMIT))
                .map_err(failed)?;
            let more = page.has_more && !page.events.is_empty();
            if !page.events.is_empty() {
                seen.0 = page.next_position;
            }
            for event in page.events {
                if event.stream_type == snapshot::SNAPSHOT {
                    let tenant = observations::tenant_of_stream(&self.instance, &event.stream_id)
                        .map_err(failed)?;
                    if !seen.1.contains(&tenant) {
                        seen.1.push(tenant);
                    }
                }
            }
            if !more {
                break;
            }
        }
        Ok(seen.1.iter().rev().cloned().collect())
    }

    /// The tenant of the observations that holds `entity`'s instance `identity`, with its
    /// stream's version and newest body; `None` when no tenant holds it.
    fn locate(
        &self,
        entity: &'static str,
        identity: &str,
    ) -> Result<Option<(TenantId, u64, Option<Value>)>, StoreError> {
        self.commit_observations()?;
        for tenant in self.observation_tenants(entity)? {
            let Ok(stream) = Self::stream(&tenant, entity, identity) else {
                return Ok(None);
            };
            let (version, body) = self.latest(entity, identity, &stream)?;
            if version > 0 {
                return Ok(Some((tenant, version, body)));
            }
        }
        Ok(None)
    }

    /// The version and newest body of `stream`: `(0, None)` for a stream that does not exist.
    fn latest(
        &self,
        entity: &'static str,
        identity: &str,
        stream: &StreamId,
    ) -> Result<(u64, Option<Value>), StoreError> {
        let failed = |source| log_error(entity, Some(identity), source);
        let Some(head) = self
            .runtime
            .block_on(self.log.stream_version(stream))
            .map_err(failed)?
        else {
            return Ok((0, None));
        };
        let slice = self
            .runtime
            .block_on(self.log.read_stream(stream, head - 1, 1))
            .map_err(failed)?;
        let event = slice
            .events
            .into_iter()
            .next()
            .ok_or_else(|| failed(EventLogError::NotFound))?;
        Ok((head, body(entity, event)?))
    }

    /// Every instance of `entity` the store holds, as `(stream, body)`, in the order they were
    /// first stored. An instance whose newest record this build cannot read is left out, and each
    /// such record is kept for [`Store::check`] and [`Store::unreadable`]; every other instance is
    /// still answered.
    fn rows(&self, entity: &'static str) -> Vec<(String, Value)> {
        let folded = if observations::holds(entity) {
            self.fold_observations(entity)
        } else {
            self.fold(&self.tenant, entity)
        };
        match folded {
            Ok(streams) => streams
                .into_iter()
                .filter_map(|folded| match folded.newest {
                    Ok(body) => Some((folded.stream, body?)),
                    Err(unreadable) => {
                        self.fail(unreadable);
                        None
                    }
                })
                .collect(),
            Err(failure) => {
                self.fail(failure);
                Vec::new()
            }
        }
    }

    /// [`Store::fold`] over every tenant of the observations, in the order the streams were first
    /// recorded.
    fn fold_observations(&self, entity: &'static str) -> Result<Vec<Folded>, StoreError> {
        self.commit_observations()?;
        let mut streams = Vec::new();
        for tenant in self.observation_tenants(entity)? {
            streams.extend(self.fold(&tenant, entity)?);
        }
        streams.sort_by_key(|folded| folded.first);
        Ok(streams)
    }

    /// Every stream of `entity` in `tenant`, in the order they were first stored, each with its
    /// newest body.
    fn fold(&self, tenant: &TenantId, entity: &'static str) -> Result<Vec<Folded>, StoreError> {
        let mut order: Vec<(String, (OffsetDateTime, u64))> = Vec::new();
        let mut newest: HashMap<String, Result<Option<Value>, StoreError>> = HashMap::new();
        let mut after = 0;
        loop {
            let page = self
                .runtime
                .block_on(self.log.read_feed(tenant, after, MAX_READ_LIMIT))
                .map_err(|source| log_error(entity, None, source))?;
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            for event in page.events {
                if event.stream_type != entity {
                    continue;
                }
                let stream = event.stream_id.clone();
                let first = (event.recorded_at, event.global_seq);
                let snapshot = body(entity, event);
                if newest.insert(stream.clone(), snapshot).is_none() {
                    order.push((stream, first));
                }
            }
            if !more {
                break;
            }
        }
        Ok(order
            .into_iter()
            .filter_map(|(stream, first)| {
                let newest = newest.remove(&stream)?;
                Some(Folded {
                    stream,
                    first,
                    newest,
                })
            })
            .collect())
    }

    /// The streams of `entity` in the instance's tenant, deleted ones too, and each one's events:
    /// what the host's ledger counts.
    fn stream_lengths(&self, entity: &'static str) -> Result<HashMap<String, u64>, EventLogError> {
        let mut lengths: HashMap<String, u64> = HashMap::new();
        let mut after = 0;
        loop {
            let page =
                self.runtime
                    .block_on(self.log.read_feed(&self.tenant, after, MAX_READ_LIMIT))?;
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            for event in page.events {
                if event.stream_type == entity {
                    let length = lengths.entry(event.stream_id).or_default();
                    *length = (*length).max(event.version);
                }
            }
            if !more {
                break;
            }
        }
        Ok(lengths)
    }

    /// Appends one event to an instance's stream, unless an earlier failure stopped this handle.
    fn write(&self, entity: &'static str, identity: &str, name: &'static str, body: Value) {
        if !self.failures.borrow().is_empty() {
            return;
        }
        let written = if observations::holds(entity) {
            self.append_observation(entity, identity, name, body)
        } else {
            self.append(&self.tenant, entity, identity, name, body)
        };
        if let Err(failure) = written {
            self.fail(failure);
        }
    }

    /// Appends one event to an observation's stream, in the tenant of the snapshot its body
    /// names. A deletion names none: it goes to the tenant that holds the observation, and where
    /// none does there is nothing to delete.
    fn append_observation(
        &self,
        entity: &'static str,
        identity: &str,
        name: &'static str,
        body: Value,
    ) -> Result<(), StoreError> {
        let tenant = match body.get("snapshot_id").and_then(Value::as_str) {
            Some(snapshot) => self
                .observation_tenant(snapshot)
                .map_err(|source| log_error(entity, Some(identity), source))?,
            None => match self.locate(entity, identity)? {
                Some((tenant, _, _)) => tenant,
                None => return Ok(()),
            },
        };
        if self.held_back.borrow().is_some() {
            return self.hold_back(&tenant, entity, identity, name, body);
        }
        self.append(&tenant, entity, identity, name, body)
    }

    /// Holds every observation record written from now on back, until
    /// [`Store::commit_observations`] commits them together, as one transaction per snapshot
    /// (`story:snapshot-store-cost`). Any read of an observation commits what is held first, so a
    /// read answers what was written.
    pub fn hold_observations(&self) {
        let mut held = self.held_back.borrow_mut();
        if held.is_none() {
            *held = Some(Vec::new());
        }
    }

    /// Commits the observation records held back since [`Store::hold_observations`], one
    /// transaction per snapshot tenant, and goes on holding. A failure is kept for
    /// [`Store::check`], as a failed write is; none is committed after it.
    ///
    /// # Errors
    ///
    /// The commit failed: a stream another writer created or moved meanwhile, or the log.
    pub fn commit_observations(&self) -> Result<(), StoreError> {
        let held = match self.held_back.borrow_mut().as_mut() {
            Some(held) if !held.is_empty() => std::mem::take(held),
            _ => return Ok(()),
        };
        if !self.failures.borrow().is_empty() {
            return Ok(());
        }
        let mut start = 0;
        while start < held.len() {
            let tenant = held[start].stream.tenant().clone();
            let end = held[start..]
                .iter()
                .position(|back| back.stream.tenant() != &tenant)
                .map_or(held.len(), |offset| start + offset);
            if let Err(failure) = self.commit_group(&tenant, &held[start..end]) {
                self.fail(failure.clone());
                return Err(failure);
            }
            start = end;
        }
        Ok(())
    }

    /// Writes nothing more back: commits what is held ([`Store::commit_observations`]) and writes
    /// each observation record at once from now on.
    ///
    /// # Errors
    ///
    /// The commit of what was held failed.
    pub fn release_observations(&self) -> Result<(), StoreError> {
        let committed = self.commit_observations();
        *self.held_back.borrow_mut() = None;
        committed
    }

    /// Holds one observation record back, after any held for its stream; the first of a stream
    /// expects the version this handle last read or wrote of it, or the one the log holds.
    fn hold_back(
        &self,
        tenant: &TenantId,
        entity: &'static str,
        identity: &str,
        name: &'static str,
        body: Value,
    ) -> Result<(), StoreError> {
        let stream = Self::stream(tenant, entity, identity)?;
        let failed = |source| log_error(entity, Some(identity), source);
        let event = NewEvent::new(name, SCHEMA_VERSION, body).map_err(failed)?;
        let key = (entity, identity.to_owned());
        {
            let mut held = self.held_back.borrow_mut();
            if let Some(back) = held
                .as_mut()
                .and_then(|held| held.iter_mut().find(|back| back.stream == stream))
            {
                back.events.push(event);
                return Ok(());
            }
        }
        let known = self.versions.borrow().get(&key).copied();
        let read = match known {
            Some(version) => version,
            None => self
                .runtime
                .block_on(self.log.stream_version(&stream))
                .map_err(failed)?
                .unwrap_or(0),
        };
        if let Some(held) = self.held_back.borrow_mut().as_mut() {
            held.push(HeldBack {
                key,
                stream,
                read,
                events: vec![event],
            });
        }
        Ok(())
    }

    /// Commits `held`, every one in `tenant`, as one transaction.
    fn commit_group(&self, tenant: &TenantId, held: &[HeldBack]) -> Result<(), StoreError> {
        let Some(first) = held.first() else {
            return Ok(());
        };
        let failed = |source| log_error(first.key.0, Some(&first.key.1), source);
        let appends: Vec<StreamAppend> = held
            .iter()
            .map(|back| StreamAppend {
                stream: back.stream.clone(),
                expected: if back.read == 0 {
                    Expected::NoStream
                } else {
                    Expected::Exact(back.read)
                },
                events: back.events.clone(),
            })
            .collect();
        let events: Vec<NewEvent> = held
            .iter()
            .flat_map(|back| back.events.iter().cloned())
            .collect();
        let group = AppendGroup {
            tenant: tenant.clone(),
            appends,
            meta: meta(&events, &self.instance).map_err(failed)?,
        };
        if let Some(refused) = refused_write() {
            return Err(failed(refused));
        }
        match self.runtime.block_on(self.log.append_group(&group)) {
            Ok(committed) => {
                let mut versions = self.versions.borrow_mut();
                for (back, appended) in held.iter().zip(committed.appends) {
                    versions.insert(back.key.clone(), appended.last_version);
                }
                Ok(())
            }
            Err(EventLogError::Conflict { actual, .. }) => Err(StoreError::Moved {
                entity: first.key.0,
                identity: first.key.1.clone(),
                read: first.read,
                held: actual,
            }),
            Err(source) => Err(failed(source)),
        }
    }

    fn append(
        &self,
        tenant: &TenantId,
        entity: &'static str,
        identity: &str,
        name: &'static str,
        body: Value,
    ) -> Result<(), StoreError> {
        let stream = Self::stream(tenant, entity, identity)?;
        let failed = |source| log_error(entity, Some(identity), source);
        let key = (entity, identity.to_owned());
        let known = self.versions.borrow().get(&key).copied();
        let read = match known {
            Some(version) => version,
            // Never read by this handle: the port replaces what is held, so it reads what is held
            // now.
            None => self
                .runtime
                .block_on(self.log.stream_version(&stream))
                .map_err(failed)?
                .unwrap_or(0),
        };
        let expected = if read == 0 {
            Expected::NoStream
        } else {
            Expected::Exact(read)
        };
        let events = [NewEvent::new(name, SCHEMA_VERSION, body).map_err(failed)?];
        let meta = meta(&events, &self.instance).map_err(failed)?;
        if let Some(refused) = refused_write() {
            return Err(failed(refused));
        }
        match self
            .runtime
            .block_on(self.log.append(&stream, expected, &events, &meta))
        {
            Ok(appended) => {
                self.versions
                    .borrow_mut()
                    .insert(key, appended.last_version);
                Ok(())
            }
            Err(EventLogError::Conflict { .. }) if read == 0 => Err(StoreError::Exists {
                entity,
                identity: identity.to_owned(),
            }),
            Err(EventLogError::Conflict { actual, .. }) => Err(StoreError::Moved {
                entity,
                identity: identity.to_owned(),
                read,
                held: actual,
            }),
            Err(source) => Err(failed(source)),
        }
    }
}

/// One stream a fold met: its id, when its first event was recorded and at which feed position,
/// and its newest body (`None` after a deletion), or why that does not read.
struct Folded {
    stream: String,
    first: (OffsetDateTime, u64),
    newest: Result<Option<Value>, StoreError>,
}

impl GoalStorage for Store {
    fn get(&self, identity: &GoalId) -> Option<GoalSnapshot> {
        let body = self.read(GOAL, &identity.0)?;
        match decode_goal(&body) {
            Ok(goal) if goal.data.goal_id == *identity => Some(goal),
            Ok(goal) => {
                self.fail(undecodable(
                    GOAL,
                    &identity.0,
                    format!("holds goal {:?}", goal.data.goal_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(GOAL, &identity.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: GoalSnapshot) {
        let body = encode_goal(&snapshot);
        self.write(GOAL, &snapshot.data.goal_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &GoalId) {
        self.write(GOAL, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<GoalSnapshot> {
        self.rows(GOAL)
            .into_iter()
            .filter_map(|(stream, body)| match decode_goal(&body) {
                Ok(goal) => Some(goal),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: GOAL,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_goal(goal: &GoalSnapshot) -> Value {
    json!({
        "actor": crate::actor::to_json(&goal.data.actor),
        "state": goal_state_name(goal.state),
        "goal_id": goal.data.goal_id.0,
        "title": goal.data.title,
        "exit_evidence": goal.data.exit_evidence,
        "evidence": goal.data.evidence,
        "reason": goal.data.reason,
    })
}

fn decode_goal(body: &Value) -> Result<GoalSnapshot, String> {
    let state = text(body, "state")?;
    Ok(GoalSnapshot {
        state: goal_state_named(&state).ok_or_else(|| format!("unknown Goal state {state:?}"))?,
        data: GoalData {
            actor: crate::actor::stored(body)?,
            goal_id: GoalId(text(body, "goal_id")?),
            title: text(body, "title")?,
            exit_evidence: text(body, "exit_evidence")?,
            evidence: goal_note(body, "evidence")?,
            reason: goal_note(body, "reason")?,
        },
    })
}

/// A goal's `evidence` or `reason`: absent or null when the goal was not met or dropped, as every
/// goal recorded before the store kept them is.
fn goal_note(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(note)) => Ok(Some(note.clone())),
        Some(other) => Err(format!("{field} is {other}, not a string")),
    }
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn goal_state_name(state: GoalState) -> &'static str {
    match state {
        GoalState::Confirmed => "Confirmed",
        GoalState::Draft => "Draft",
        GoalState::Dropped => "Dropped",
        GoalState::Met => "Met",
    }
}

fn goal_state_named(name: &str) -> Option<GoalState> {
    [
        GoalState::Confirmed,
        GoalState::Draft,
        GoalState::Dropped,
        GoalState::Met,
    ]
    .into_iter()
    .find(|state| goal_state_name(*state) == name)
}

impl GuardDecisionStorage for Store {
    fn get(&self, identity: &GuardDecisionId) -> Option<GuardDecisionSnapshot> {
        let key = &identity.0.0;
        let body = self.read(GUARD_DECISION, key)?;
        match decode_guard_decision(&body) {
            Ok(decision) if decision.data.guard_decision_id == *identity => Some(decision),
            Ok(decision) => {
                self.fail(undecodable(
                    GUARD_DECISION,
                    key,
                    format!(
                        "holds guard decision {:?}",
                        decision.data.guard_decision_id.0.0
                    ),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(GUARD_DECISION, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: GuardDecisionSnapshot) {
        let body = encode_guard_decision(&snapshot);
        self.write(
            GUARD_DECISION,
            &snapshot.data.guard_decision_id.0.0,
            STORED,
            body,
        );
    }

    fn delete(&mut self, identity: &GuardDecisionId) {
        self.write(GUARD_DECISION, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<GuardDecisionSnapshot> {
        self.rows(GUARD_DECISION)
            .into_iter()
            .filter_map(|(stream, body)| match decode_guard_decision(&body) {
                Ok(decision) => Some(decision),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: GUARD_DECISION,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_guard_decision(decision: &GuardDecisionSnapshot) -> Value {
    let data = &decision.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": guard_decision_state_name(decision.state),
        "guard_decision_id": data.guard_decision_id.0.0,
        "repository": data.repository.0,
        "session_ref": data.session_ref,
        "tool": data.tool,
        "target": data.target,
        "verdict": verdict_name(data.verdict),
        "reason": data.reason,
        "decided_at": data.decided_at.0,
    })
}

fn decode_guard_decision(body: &Value) -> Result<GuardDecisionSnapshot, String> {
    let state = text(body, "state")?;
    let verdict = text(body, "verdict")?;
    Ok(GuardDecisionSnapshot {
        state: guard_decision_state_named(&state)
            .ok_or_else(|| format!("unknown GuardDecision state {state:?}"))?,
        data: GuardDecisionData {
            actor: crate::actor::stored(body)?,
            guard_decision_id: GuardDecisionId(Uuid(text(body, "guard_decision_id")?)),
            repository: RepositoryName(text(body, "repository")?),
            session_ref: text(body, "session_ref")?,
            tool: text(body, "tool")?,
            target: text(body, "target")?,
            verdict: verdict_named(&verdict)
                .ok_or_else(|| format!("unknown Verdict {verdict:?}"))?,
            reason: text(body, "reason")?,
            decided_at: Timestamp(text(body, "decided_at")?),
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn guard_decision_state_name(state: GuardDecisionState) -> &'static str {
    match state {
        GuardDecisionState::Recorded => "Recorded",
    }
}

fn guard_decision_state_named(name: &str) -> Option<GuardDecisionState> {
    [GuardDecisionState::Recorded]
        .into_iter()
        .find(|state| guard_decision_state_name(*state) == name)
}

/// The verdict's name as the specification spells it. Exhaustive, so a verdict the specification
/// adds fails to compile here rather than being written under a guessed name.
fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "Allow",
        Verdict::Deny => "Deny",
    }
}

fn verdict_named(name: &str) -> Option<Verdict> {
    [Verdict::Allow, Verdict::Deny]
        .into_iter()
        .find(|verdict| verdict_name(*verdict) == name)
}

fn text(body: &Value, field: &str) -> Result<String, String> {
    body.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("no text field {field:?}"))
}

/// The snapshot body an event leaves its instance with: `None` after a deletion. Every event is
/// judged at its schema version: one this build does not write, a redacted snapshot, or a name it
/// does not know is a record it cannot read.
fn body(entity: &'static str, event: RecordedEvent) -> Result<Option<Value>, StoreError> {
    match event.name.as_str() {
        STORED if event.schema_version == SCHEMA_VERSION && !event.is_redacted() => {
            Ok(Some(event.data))
        }
        DELETED if event.schema_version == SCHEMA_VERSION => Ok(None),
        _ => {
            let redacted = if event.is_redacted() {
                ", redacted"
            } else {
                ""
            };
            Err(StoreError::Undecodable {
                entity,
                detail: format!("event {:?} v{}{redacted}", event.name, event.schema_version),
                stream: event.stream_id,
            })
        }
    }
}

/// The stream id of an identity: the identity itself, with every byte eventlog cannot hold
/// verbatim (and `%`) written `%XX`, so two identities never share a stream; or, where the tree
/// cannot name a directory after that, its [`held`] form.
fn stream_id(identity: &str) -> String {
    let escaped = escaped(identity);
    held(&escaped).unwrap_or(escaped)
}

/// The longest name of a directory entry (`NAME_MAX` on Linux).
const NAME_MAX: usize = 255;

/// The stream id the tree keeps the stream `id` under. The tree names a stream's directory after
/// its id, writing every byte outside `[A-Za-z0-9._-]`, and a leading `.`, as three. Where that
/// name fits in [`NAME_MAX`] the id is kept as it is; where it does not, the stream is
/// `%sha256:` followed by the SHA-256 of `id`, which no escaped identity spells, since a `%` in
/// one is always followed by two upper-case hex digits.
///
/// # Errors
///
/// `id` cannot be digested; it is a string, so it always can.
fn held(id: &str) -> Result<String, EventLogError> {
    let named: usize = id
        .bytes()
        .enumerate()
        .map(|(at, byte)| {
            if (byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
                && !(at == 0 && byte == b'.')
            {
                1
            } else {
                3
            }
        })
        .sum();
    if named <= NAME_MAX {
        return Ok(id.to_owned());
    }
    Ok(format!("%sha256:{}", request_hash(&id)?))
}

/// `identity` with every byte eventlog cannot hold verbatim (and `%`) written `%XX`.
fn escaped(identity: &str) -> String {
    let mut id = String::with_capacity(identity.len());
    for byte in identity.bytes() {
        if byte.is_ascii_graphic() && byte != b'%' {
            id.push(char::from(byte));
        } else {
            let _ = write!(id, "%{byte:02X}");
        }
    }
    id
}

/// Who wrote a record: the instance `instance`, under a fresh key per write.
fn meta(events: &[NewEvent], instance: &str) -> Result<CommandMeta, EventLogError> {
    let id = new_event_id();
    Ok(CommandMeta {
        idempotency_key: id.clone(),
        request_hash: request_hash(&events)?,
        subject: instance.to_owned(),
        actor: CONDUCTOR.to_owned(),
        request_id: id.clone(),
        trace_id: id,
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
        claim: None,
    })
}

fn log_error(entity: &'static str, identity: Option<&str>, source: EventLogError) -> StoreError {
    StoreError::Log {
        entity,
        identity: identity.map(str::to_owned),
        source,
    }
}

fn undecodable(entity: &'static str, identity: &str, detail: String) -> StoreError {
    StoreError::Undecodable {
        entity,
        stream: stream_id(identity),
        detail,
    }
}
