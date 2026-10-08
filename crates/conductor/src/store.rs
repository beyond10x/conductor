//! Conductor's local store under the state directory: every accepted command's outcome, so a
//! restarted conductor resumes from records rather than from context.
//!
//! Built on eventlog's tree provider (`story:store-on-eventlog-tree` records the choice): one
//! immutable file per event, read through an in-memory index built when the store opens. Each
//! stored instance is one stream, typed by its entity's qualified name and keyed by its identity.
//! Every write a generated behaviour makes through a storage port appends the instance's new
//! snapshot to that stream, and every read folds the stream back, so a reopened store answers the
//! views from records.
//!
//! The records are kept in two tree stores (`story:observation-retention`). `state/tree/` keeps
//! every record but the observations, in one tenant, `conductor`. `state/observations/` keeps every
//! `*Observation` entity of the observation domain ([`observations`]), each snapshot's in a tenant
//! of its own, named after the snapshot's id; the `Snapshot` itself is in `tree/`. A tree store
//! re-reads every file when it opens, so a handle opens `tree/` when it is opened and
//! `observations/` only at its first read or write of an observation: a command that touches no
//! observation, the guard's hook among them, never opens it, and `tree/` does not grow with the
//! observations. [`Store::retain`] drops the tenants of the snapshots retention does not keep.
//!
//! The store before it was eventlog's File provider under `state/store/`. Where that store is and
//! no tree store is, every command refuses and names `conductor store migrate` ([`migrate`]), which
//! copies it; nothing reads or writes it otherwise. The same command splits a `tree/` that holds
//! observations, as the build before this one wrote it.
//!
//! The tree keeps each stream in a directory named after its id, so an id whose name would be
//! longer than a directory entry may be is kept under the digest of it instead ([`held`]): an
//! identity of up to 51 bytes always keeps its stream id, and one of only ASCII letters and
//! digits up to 255 bytes does.
//!
//! A tree store serves reads from the index it built, and rebuilds it only when it writes. So a
//! handle takes the tree's [`Mark`] (its commit files and its tenants) before each read, and before
//! a write to an instance it has not read, and reopens the tree when another writer has committed
//! since or a tenant was dropped: a long-lived handle sees another handle's writes, as it did on
//! the File provider.
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
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::fmt::{self, Write as _};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use anyhow::{Context as _, Result, bail};
use conductor_model::behaviour::{GoalStorage, GuardDecisionStorage};
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use conductor_model::dispatch::{
    GuardDecisionData, GuardDecisionId, GuardDecisionSnapshot, GuardDecisionState, Verdict,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::{Timestamp, Uuid};
use eventlog_core::{
    CommandMeta, EventLogError, EventStore, Expected, MAX_READ_LIMIT, NewEvent, RecordedEvent,
    StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::runtime::{Builder, Handle, Runtime};

// The storage ports of the later stories, one child module each, so that units running at once
// never edit this file. A child module reaches the private helpers below (`read`, `write`, `fail`).
mod controller;
mod decision;
mod dispatch;
mod marks;
pub mod migrate;
pub mod observations;
mod serving;
mod snapshot;

/// The directory under `state/` the tree store of every record but the observations lives in.
const TREE_DIR: &str = "tree";

/// The directory under `state/` the observations live in, one tenant per snapshot.
const OBSERVATIONS_DIR: &str = "observations";

/// The directory under `state/` the File provider's store lived in, before `conductor store
/// migrate`.
const FILE_STORE_DIR: &str = "store";

/// The File provider's commit point: a store directory holding it holds that provider's store.
const FILE_STORE_MANIFEST: &str = "manifest.json";

/// The tree store's own description, written when it is created: a directory holding it holds a
/// tree store.
const TREE_MANIFEST: &str = "store.json";

/// The tree store's writer lock, one writer at a time (eventlog-tree's layout).
const TREE_LOCK: &str = ".lock";

/// The one tenant of conductor's log, and the opaque id every record is attributed to.
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

/// Opens the store kept under `state`, at `state/tree/` (and `state/observations/` at its first
/// use), creating `tree/` when it is not there.
///
/// # Errors
///
/// `state/store/` holds a File provider's store and `state/tree/` holds no tree store, or
/// `state/tree.next/` is there and `state/tree/` holds no tree store: nothing is written, and the
/// one line names `conductor store migrate`. Or the directory cannot be created, or the tree in it
/// does not open: eventlog verifies every file it replays.
pub fn open(state: &Path) -> Result<Store> {
    let root = state.join(TREE_DIR);
    unmigrated(state, &root)?;
    fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;
    // Two first commands at once would both write the tree's description through one staging
    // name; the tree's own writer lock makes the second wait and then find it written.
    let _creating = if is_file(&root.join(TREE_MANIFEST)) {
        None
    } else {
        Some(hold(&root.join(TREE_LOCK)).with_context(|| format!("create {}", root.display()))?)
    };
    Store::at(state)
}

/// Opens the store kept under `state`, at `state/tree/`, only when it is there: nothing is
/// created, so a read never plants a store, and a read of observations where `state/observations/`
/// holds none reads nothing.
///
/// # Errors
///
/// As [`open`] for a store `conductor store migrate` is to move; or `state/tree/` holds no tree
/// store (one line naming it), or the tree in it does not open.
pub fn open_existing(state: &Path) -> Result<Store> {
    let root = state.join(TREE_DIR);
    unmigrated(state, &root)?;
    if !is_file(&root.join(TREE_MANIFEST)) {
        bail!("no store at {}", root.display());
    }
    Store::at(state)
}

/// Refuses a state directory whose `tree/` holds no tree store while `store/` holds a File
/// provider's store, whose records would otherwise be left behind unread; or while `tree.next/`
/// is there, which `conductor store migrate` renames to `tree/` and which a new `tree/` would
/// stand in the way of.
fn unmigrated(state: &Path, tree: &Path) -> Result<()> {
    if is_file(&tree.join(TREE_MANIFEST)) {
        return Ok(());
    }
    let file_store = state.join(FILE_STORE_DIR);
    if fs::symlink_metadata(file_store.join(FILE_STORE_MANIFEST)).is_ok() {
        bail!(
            "{} holds an eventlog-file store and {} holds no tree store: run `conductor store \
             migrate`",
            file_store.display(),
            tree.display()
        );
    }
    let next = state.join(migrate::NEXT_DIR);
    if fs::symlink_metadata(&next).is_ok() {
        bail!(
            "{} holds a tree `conductor store migrate` has not finished and {} holds no tree \
             store: run `conductor store migrate`",
            next.display(),
            tree.display()
        );
    }
    Ok(())
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

/// What a tree holds, as far as a handle needs it to know whether another writer changed the tree
/// since: how many commit files, one per command any writer committed
/// (`tenants/<tenant>/groups/…/<digest>.json` in eventlog-tree's layout), and the directory of
/// each tenant. A commit adds exactly one file. Nothing conductor runs removes one but
/// [`Store::retain`], which erases whole tenants, and a dropped snapshot's tenant is never written
/// again (its snapshot is no longer collecting). So two equal marks mean nobody committed or
/// erased in between.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Mark {
    /// The commit files.
    commits: usize,
    /// Each tenant's directory name.
    tenants: BTreeSet<OsString>,
}

impl Mark {
    /// Whether `after` is this mark and exactly one more commit, in `tenant`, which the commit may
    /// have created.
    fn one_more(&self, after: &Self, tenant: &TenantId) -> bool {
        let mut tenants = self.tenants.clone();
        tenants.insert(OsString::from(observations::segment(tenant.as_str())));
        after.commits == self.commits + 1 && after.tenants == tenants
    }
}

/// The [`Mark`] of the tree under `root`; an empty one where nothing is there.
fn mark(root: &Path) -> io::Result<Mark> {
    fn walk(directory: &Path, found: &mut usize) -> io::Result<()> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            let name = entry.file_name();
            if kind.is_dir() {
                walk(&entry.path(), found)?;
            } else if kind.is_file()
                && !name.to_string_lossy().starts_with('.')
                && Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext == "json")
            {
                *found += 1;
            }
        }
        Ok(())
    }
    let mut mark = Mark::default();
    let tenants = match fs::read_dir(root.join("tenants")) {
        Ok(tenants) => tenants,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(mark),
        Err(error) => return Err(error),
    };
    for tenant in tenants {
        let tenant = tenant?;
        if tenant.file_type()?.is_dir() {
            walk(&tenant.path().join("groups"), &mut mark.commits)?;
            mark.tenants.insert(tenant.file_name());
        }
    }
    Ok(mark)
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
    runtime: Engine,
    /// `state/tree/`: every record but the observations, in the tenant [`Store::tenant`].
    tree: Tree,
    /// `state/observations/`: the observations, one tenant per snapshot; opened at its first use.
    observations: Tree,
    /// The one tenant of `tree`.
    tenant: TenantId,
    /// The stream version this handle last read or wrote, per `(entity, identity)`; `0` is a
    /// stream that did not exist. What the next write to it expects.
    versions: RefCell<HashMap<(&'static str, String), u64>>,
    /// Every read or write that failed on this handle, in the order they failed; a record that
    /// does not decode is kept once however often it is read.
    failures: RefCell<Vec<StoreError>>,
}

/// One of a handle's tree stores.
struct Tree {
    /// Its directory.
    root: PathBuf,
    /// The tree, as last opened: reopened when another writer has committed since. `None` until
    /// the first use of a tree the handle opens at its first use.
    log: RefCell<Option<TreeEventStore>>,
    /// The [`Mark`] of what `log` is known to serve, when that is known: when the tree is at this
    /// mark, `log` serves all of it. `None` after a write that cannot tell whether another writer
    /// committed beside it.
    seen: RefCell<Option<Mark>>,
}

impl Tree {
    /// A tree at `root`, opened at its first use.
    fn unopened(root: PathBuf) -> Self {
        Self {
            root,
            log: RefCell::new(None),
            seen: RefCell::new(None),
        }
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
    /// Opens the stores under `state`: `tree/`, which holds one or is to be created, now, and
    /// `observations/` at its first use.
    fn at(state: &Path) -> Result<Self> {
        let root = state.join(TREE_DIR);
        let runtime = Engine::start()?;
        let seen = mark(&root).with_context(|| format!("read {}", root.display()))?;
        let log = runtime
            .block_on(TreeEventStore::open(&root))
            .with_context(|| format!("open the store at {}", root.display()))?;
        Ok(Self {
            runtime,
            tree: Tree {
                root,
                log: RefCell::new(Some(log)),
                seen: RefCell::new(Some(seen)),
            },
            observations: Tree::unopened(state.join(OBSERVATIONS_DIR)),
            tenant: TenantId::new(CONDUCTOR)?,
            versions: RefCell::default(),
            failures: RefCell::default(),
        })
    }

    /// Brings `tree`'s log up to its files, and answers whether the tree holds a store. At the
    /// tree's first use it opens it, creating it when `create` says so; where it is not there and
    /// is not to be created, it answers `false`, a store of no records, and opens nothing. Later
    /// it reopens the tree when the tree's [`Mark`] is not the one its log is known to serve:
    /// another writer committed or erased since, so the log's index is behind the files.
    fn fresh(&self, tree: &Tree, create: bool) -> Result<bool, EventLogError> {
        if tree.log.borrow().is_none() {
            if !create && !is_file(&tree.root.join(TREE_MANIFEST)) {
                return Ok(false);
            }
            fs::create_dir_all(&tree.root).map_err(|error| backend(&error))?;
            // As in `open`: the first writer of a new tree holds its lock while it writes the
            // tree's description.
            let _creating = if is_file(&tree.root.join(TREE_MANIFEST)) {
                None
            } else {
                Some(hold(&tree.root.join(TREE_LOCK)).map_err(|error| backend(&error))?)
            };
            let seen = mark(&tree.root).map_err(|error| backend(&error))?;
            let log = self.reopened(tree)?;
            *tree.log.borrow_mut() = Some(log);
            *tree.seen.borrow_mut() = Some(seen);
            return Ok(true);
        }
        let now = mark(&tree.root).map_err(|error| backend(&error))?;
        if tree.seen.borrow().as_ref() == Some(&now) {
            return Ok(true);
        }
        let log = self.reopened(tree)?;
        *tree.log.borrow_mut() = Some(log);
        *tree.seen.borrow_mut() = Some(now);
        Ok(true)
    }

    /// `tree` opened afresh from its files; a failure names its directory, since a handle opens
    /// two.
    fn reopened(&self, tree: &Tree) -> Result<TreeEventStore, EventLogError> {
        self.runtime
            .block_on(TreeEventStore::open(&tree.root))
            .map_err(|error| {
                EventLogError::Backend(format!(
                    "open the store at {}: {error}",
                    tree.root.display()
                ))
            })
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

    /// Keeps `failure`, unless it is already kept: an unreadable record is the same failure
    /// whatever read met it.
    fn fail(&self, failure: StoreError) {
        let mut failures = self.failures.borrow_mut();
        if !failures.iter().any(|kept| kept.same(&failure)) {
            failures.push(failure);
        }
    }

    /// The stream of `entity`'s instance `identity` in `tenant`. An identity whose escaped form
    /// eventlog refuses as a stream id names no stream, whatever form the tree keeps it under.
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
            self.fresh(&self.tree, false)
                .map_err(|source| log_error(entity, Some(identity), source))
                .and_then(|_| self.latest(&self.tree, entity, identity, &stream))
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

    /// The tenant of the observation store that holds `entity`'s instance `identity`, with its
    /// stream's version and newest body; `None` when no tenant holds it, or the observation store
    /// is not there.
    fn locate(
        &self,
        entity: &'static str,
        identity: &str,
    ) -> Result<Option<(TenantId, u64, Option<Value>)>, StoreError> {
        let failed = |source| log_error(entity, Some(identity), source);
        if !self.fresh(&self.observations, false).map_err(failed)? {
            return Ok(None);
        }
        for tenant in observations::tenants(&self.observations.root).map_err(failed)? {
            let Ok(stream) = Self::stream(&tenant, entity, identity) else {
                return Ok(None);
            };
            let (version, body) = self.latest(&self.observations, entity, identity, &stream)?;
            if version > 0 {
                return Ok(Some((tenant, version, body)));
            }
        }
        Ok(None)
    }

    /// The version and newest body of `stream` in `tree`, whose log is opened: `(0, None)` for a
    /// stream that does not exist, or a tree that is not there.
    fn latest(
        &self,
        tree: &Tree,
        entity: &'static str,
        identity: &str,
        stream: &StreamId,
    ) -> Result<(u64, Option<Value>), StoreError> {
        let failed = |source| log_error(entity, Some(identity), source);
        let log = tree.log.borrow();
        let Some(log) = log.as_ref() else {
            return Ok((0, None));
        };
        let Some(head) = self
            .runtime
            .block_on(log.stream_version(stream))
            .map_err(failed)?
        else {
            return Ok((0, None));
        };
        let slice = self
            .runtime
            .block_on(log.read_stream(stream, head - 1, 1))
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
            self.fresh(&self.tree, false)
                .map_err(|source| log_error(entity, None, source))
                .and_then(|_| self.fold(&self.tree, &self.tenant, entity))
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

    /// [`Store::fold`] over every tenant of the observation store, in the order the streams were
    /// first recorded: a tenant is replayed after the one before it in name order whatever the
    /// instants, so the feed positions alone do not order two snapshots' observations.
    fn fold_observations(&self, entity: &'static str) -> Result<Vec<Folded>, StoreError> {
        let failed = |source| log_error(entity, None, source);
        if !self.fresh(&self.observations, false).map_err(failed)? {
            return Ok(Vec::new());
        }
        let mut streams = Vec::new();
        for tenant in observations::tenants(&self.observations.root).map_err(failed)? {
            streams.extend(self.fold(&self.observations, &tenant, entity)?);
        }
        streams.sort_by_key(|folded| folded.first);
        Ok(streams)
    }

    /// Every stream of `entity` in `tenant` of `tree`, whose log is opened, in the order they were
    /// first stored, each with its newest body.
    fn fold(
        &self,
        tree: &Tree,
        tenant: &TenantId,
        entity: &'static str,
    ) -> Result<Vec<Folded>, StoreError> {
        let mut order: Vec<(String, (OffsetDateTime, u64))> = Vec::new();
        let mut newest: HashMap<String, Result<Option<Value>, StoreError>> = HashMap::new();
        let mut after = 0;
        let log = tree.log.borrow();
        let Some(log) = log.as_ref() else {
            return Ok(Vec::new());
        };
        loop {
            let page = self
                .runtime
                .block_on(log.read_feed(tenant, after, MAX_READ_LIMIT))
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

    /// Appends one event to an instance's stream, unless an earlier failure stopped this handle.
    fn write(&self, entity: &'static str, identity: &str, name: &'static str, body: Value) {
        if !self.failures.borrow().is_empty() {
            return;
        }
        let written = if observations::holds(entity) {
            self.append_observation(entity, identity, name, body)
        } else {
            self.append(&self.tree, &self.tenant, entity, identity, name, body)
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
            Some(snapshot) => observations::tenant(snapshot)
                .map_err(|source| log_error(entity, Some(identity), source))?,
            None => match self.locate(entity, identity)? {
                Some((tenant, _, _)) => tenant,
                None => return Ok(()),
            },
        };
        self.append(&self.observations, &tenant, entity, identity, name, body)
    }

    fn append(
        &self,
        tree: &Tree,
        tenant: &TenantId,
        entity: &'static str,
        identity: &str,
        name: &'static str,
        body: Value,
    ) -> Result<(), StoreError> {
        let stream = Self::stream(tenant, entity, identity)?;
        let failed = |source| log_error(entity, Some(identity), source);
        let key = (entity, identity.to_owned());
        let read = self.versions.borrow().get(&key).copied();
        let read = match read {
            Some(version) => {
                if tree.log.borrow().is_none() {
                    self.fresh(tree, true).map_err(failed)?;
                }
                version
            }
            // Never read by this handle: the port replaces what is held, so it reads what is held
            // now.
            None => {
                self.fresh(tree, true).map_err(failed)?;
                let log = tree.log.borrow();
                let log = log.as_ref().ok_or_else(|| failed(EventLogError::Closed))?;
                self.runtime
                    .block_on(log.stream_version(&stream))
                    .map_err(failed)?
                    .unwrap_or(0)
            }
        };
        let opened = tree.log.borrow();
        let log = opened
            .as_ref()
            .ok_or_else(|| failed(EventLogError::Closed))?;
        let expected = if read == 0 {
            Expected::NoStream
        } else {
            Expected::Exact(read)
        };
        let events = [NewEvent::new(name, SCHEMA_VERSION, body).map_err(failed)?];
        let meta = meta(&events).map_err(failed)?;
        let before = tree.seen.borrow_mut().take();
        let appended = self
            .runtime
            .block_on(log.append(&stream, expected, &events, &meta));
        drop(opened);
        // The append added exactly one commit file. When the tree gained only that one, the log
        // serves all of it; when it gained more, another writer committed beside this one, and the
        // next read reopens the tree.
        if appended.is_ok()
            && let Some(before) = before
            && let Ok(after) = mark(&tree.root)
            && before.one_more(&after, tenant)
        {
            *tree.seen.borrow_mut() = Some(after);
        }
        match appended {
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
        "state": goal_state_name(goal.state),
        "goal_id": goal.data.goal_id.0,
        "title": goal.data.title,
        "exit_evidence": goal.data.exit_evidence,
    })
}

fn decode_goal(body: &Value) -> Result<GoalSnapshot, String> {
    let state = text(body, "state")?;
    Ok(GoalSnapshot {
        state: goal_state_named(&state).ok_or_else(|| format!("unknown Goal state {state:?}"))?,
        data: GoalData {
            goal_id: GoalId(text(body, "goal_id")?),
            title: text(body, "title")?,
            exit_evidence: text(body, "exit_evidence")?,
        },
    })
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

/// Who wrote a record: conductor, under a fresh key per write.
fn meta(events: &[NewEvent]) -> Result<CommandMeta, EventLogError> {
    let id = new_event_id();
    Ok(CommandMeta {
        idempotency_key: id.clone(),
        request_hash: request_hash(&events)?,
        subject: CONDUCTOR.to_owned(),
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
