//! `conductor store migrate` (`story:store-on-eventlog-tree`, `story:observation-retention`):
//! moves the records under the state directory into the two tree stores the store reads,
//! `tree/` and `observations/` (the store module says which records each holds), and keeps what it
//! moved them from beside them. It moves one of two:
//!
//! - the File provider's store, `store/`, where there is no `tree/`;
//! - a `tree/` that holds observations, as the build before `story:observation-retention` wrote
//!   it: the run splits it.
//!
//! From the File store, the run holds the File store's writer lock from before it reads the store
//! until it has renamed it, so no writer of that store appends in between. Every reader of a File
//! store takes that lock too, this one's included, so the history is read from a copy made under
//! the lock, `store.migrating/`, through eventlog's own inspection of a File store, and the copy is
//! removed once read.
//!
//! Every event of the `conductor` tenant is appended anew, in feed order, with its stream, type,
//! name, schema version and body, and with the subject, actor and instant its writer gave it; the
//! tree assigns its own ids, record instants and feed positions. An observation is appended to
//! `observations/`, in the tenant of the snapshot the newest record of its stream names; every
//! other record to `tree.next/`. A stream whose id the tree cannot name a directory after is kept
//! under the id the store reads it by (`held` in the store module). Each stream's event count and
//! digest is then read back from `tree.next/` and `observations/`, opened afresh, and compared
//! with the source's. Only then is `tree.next/` renamed `tree/`, and `store/` renamed
//! `store.eventlog-file-<UTC stamp>/`, which nothing deletes.
//!
//! `observations/` is written in place: an observation stream it already holds as far as the
//! source does, which a run that stopped left there, is not appended again, and the read-back
//! finds whether it holds it alike.
//!
//! It refuses, before writing anything, where `tree/` exists and holds no observations, and where
//! an observation's stream names no snapshot. A run that stops before its first rename leaves
//! `store/` as it was; the next run removes the `tree.next/` and `store.migrating/` it left, under
//! the lock, and starts again.
//!
//! A split holds `tree/`'s writer lock, which every writer of the tree takes, from before it reads
//! `tree/` until it has renamed it, and reads every event of `tree/` from a handle opened afresh.
//! It moves them as above, then renames `tree/` `tree.split-<UTC stamp>/`, which nothing deletes,
//! and `tree.next/` `tree/`. Between the two renames every other command refuses and names this
//! one (`unmigrated` in the store module). A split that stopped there is finished by the next run,
//! which renames `tree.next/` `tree/` where no `tree/` and no File store are and a
//! `tree.split-*/` is; one that stopped before its first rename leaves `tree/` as it was, and the
//! next run removes the `tree.next/` it left and splits again.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, InspectHistory as _, InspectionLimits, MAX_READ_LIMIT,
    NewEvent, RecordedEvent, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_file::{FileEventStore, FileHistoryInspector};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::observations::{self, segment};
use super::{
    CONDUCTOR, Engine, FILE_STORE_DIR, FILE_STORE_MANIFEST, OBSERVATIONS_DIR, TREE_DIR, TREE_LOCK,
    TREE_MANIFEST, held, hold, is_file,
};
use crate::cli::StoreMigrateArgs;

/// The File store's writer lock, which every writer and reader of it holds.
const FILE_STORE_LOCK: &str = "writer.lock";

/// Where the tree is written before it is complete and checked.
pub(super) const NEXT_DIR: &str = "tree.next";

/// Where the File store is copied under its lock, to be read.
const COPY_DIR: &str = "store.migrating";

/// The name a split keeps the tree it split under, before its UTC stamp.
const SPLIT_PREFIX: &str = "tree.split-";

/// What a migration moved of one stream type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Moved {
    /// Its streams: one per stored instance.
    pub streams: usize,
    /// Its events, across those streams.
    pub events: usize,
}

/// What a run of `store migrate` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Moved the File provider's store.
    EventlogFile,
    /// Split a `tree/` that held observations.
    Split,
    /// Finished a split that stopped between its two renames: renamed `tree.next/` `tree/`.
    Finished,
}

/// What a migration moved, and where the stores are now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrated {
    /// What the run did.
    pub kind: Kind,
    /// Per stream type (an entity's qualified name), what was moved; nothing for
    /// [`Kind::Finished`].
    pub types: BTreeMap<String, Moved>,
    /// What the records were moved from: `state/store/`, or `state/tree/` for a split, or
    /// `state/tree.next/` for a split finished.
    pub source: PathBuf,
    /// The tree store, `state/tree/`.
    pub tree: PathBuf,
    /// The observation store, `state/observations/`.
    pub observations: PathBuf,
    /// Where what the records were moved from is kept: `state/store.eventlog-file-<UTC stamp>/`,
    /// or `state/tree.split-<UTC stamp>/`.
    pub kept: PathBuf,
}

impl Migrated {
    /// Every stream moved.
    #[must_use]
    pub fn streams(&self) -> usize {
        self.types.values().map(|moved| moved.streams).sum()
    }

    /// Every event moved.
    #[must_use]
    pub fn events(&self) -> usize {
        self.types.values().map(|moved| moved.events).sum()
    }
}

/// `conductor store migrate`: [`migrate`] on the state directory (`--state-dir`, else `state/`),
/// then the counts it moved on standard output, as text or, with `--format json`, one JSON object.
///
/// # Errors
///
/// `--format` is neither text nor json, or [`migrate`] refuses or fails.
pub fn store_migrate(state: Option<&Path>, args: &StoreMigrateArgs) -> Result<ExitCode> {
    const COMMAND: &str = "store migrate";
    let as_json = match args.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is neither text nor json"),
    };
    let state = crate::state::dir(state).context(COMMAND)?;
    let migrated = migrate(&state).context(COMMAND)?;
    let rendered = if as_json {
        json_report(&migrated)
    } else {
        text_report(&migrated)
    };
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(rendered.as_bytes())
        .and_then(|()| stdout.flush())
        .context("write the counts moved to standard output")?;
    Ok(ExitCode::SUCCESS)
}

fn text_report(migrated: &Migrated) -> String {
    if migrated.kind == Kind::Finished {
        return format!(
            "finished the split a run left between its renames: renamed {} to {}; the tree before \
             the split is kept at {}\n",
            migrated.source.display(),
            migrated.tree.display(),
            migrated.kept.display()
        );
    }
    let mut text = String::new();
    for (stream_type, moved) in &migrated.types {
        text.push_str(&format!(
            "{stream_type}: {} streams, {} events\n",
            moved.streams, moved.events
        ));
    }
    let kept = match migrated.kind {
        Kind::EventlogFile => "the eventlog-file store",
        Kind::Split | Kind::Finished => "the tree before the split",
    };
    text.push_str(&format!(
        "moved {} events in {} streams from {} to {} and {}; {kept} is kept at {}\n",
        migrated.events(),
        migrated.streams(),
        migrated.source.display(),
        migrated.tree.display(),
        migrated.observations.display(),
        migrated.kept.display()
    ));
    text
}

fn json_report(migrated: &Migrated) -> String {
    let types: Vec<_> = migrated
        .types
        .iter()
        .map(|(stream_type, moved)| {
            json!({
                "stream_type": stream_type,
                "streams": moved.streams,
                "events": moved.events,
            })
        })
        .collect();
    let kind = match migrated.kind {
        Kind::EventlogFile => "eventlog-file",
        Kind::Split => "split",
        Kind::Finished => "finished-split",
    };
    format!(
        "{}\n",
        json!({
            "kind": kind,
            "streams": migrated.streams(),
            "events": migrated.events(),
            "types": types,
            "source": migrated.source.display().to_string(),
            "tree": migrated.tree.display().to_string(),
            "observations": migrated.observations.display().to_string(),
            "kept": migrated.kept.display().to_string(),
        })
    )
}

/// Moves the records under `state` as the module says, and answers what it moved.
///
/// # Errors
///
/// `state/tree/` exists and holds no observations, or neither `state/tree/` nor `state/store/`
/// holds a store, or an observation's stream names no snapshot: nothing is written. The store
/// moved from does not open or does not read, an event is refused by a tree, a tree read back does
/// not hold what the source held (then `tree.next/` is left for inspection and the source is as it
/// was), or a rename fails.
pub fn migrate(state: &Path) -> Result<Migrated> {
    let tree = state.join(TREE_DIR);
    if fs::symlink_metadata(&tree).is_ok() {
        if is_file(&tree.join(TREE_MANIFEST)) && holds_observations(&tree) {
            return split(state);
        }
        refuse_migrated(&tree)?;
    }
    if let Some(finished) = finish_split(state)? {
        return Ok(finished);
    }
    from_file(state)
}

/// Moves the File store at `state/store/`.
fn from_file(state: &Path) -> Result<Migrated> {
    let source = state.join(FILE_STORE_DIR);
    let tree = state.join(TREE_DIR);
    let next = state.join(NEXT_DIR);
    let copy = state.join(COPY_DIR);
    let tenant = TenantId::new(CONDUCTOR)?;
    if fs::symlink_metadata(source.join(FILE_STORE_MANIFEST)).is_err() {
        bail!(
            "no eventlog-file store at {}: nothing to migrate",
            source.display()
        );
    }
    let runtime = Engine::start()?;
    // Opening it finishes a write it left pending and verifies its history, as every opener did.
    drop(
        runtime
            .block_on(FileEventStore::open_existing(&source))
            .with_context(|| format!("open the eventlog-file store at {}", source.display()))?,
    );
    let _lock = hold(&source.join(FILE_STORE_LOCK))
        .with_context(|| format!("hold the writer lock of {}", source.display()))?;
    refuse_migrated(&tree)?;
    for leftover in [&next, &copy] {
        if fs::symlink_metadata(leftover).is_ok() {
            fs::remove_dir_all(leftover).with_context(|| {
                format!(
                    "remove {}, which an earlier run left unfinished",
                    leftover.display()
                )
            })?;
        }
    }

    copy_dir(&source, &copy)
        .with_context(|| format!("copy {} to {}", source.display(), copy.display()))?;
    let limits = InspectionLimits {
        source_bytes: u64::MAX,
        events: u64::MAX,
        envelope_bytes: u64::MAX,
    };
    let history = runtime
        .block_on(FileHistoryInspector::new(&copy).inspect_history(&tenant, limits))
        .with_context(|| format!("read the eventlog-file store at {}", source.display()))?;
    fs::remove_dir_all(&copy).with_context(|| format!("remove {}", copy.display()))?;

    let types = move_records(&runtime, state, &source, &history.events)?;
    fs::rename(&next, &tree)
        .with_context(|| format!("rename {} to {}", next.display(), tree.display()))?;
    let kept = state.join(format!(
        "store.eventlog-file-{}",
        stamp(OffsetDateTime::now_utc())
    ));
    fs::rename(&source, &kept)
        .with_context(|| format!("rename {} to {}", source.display(), kept.display()))?;
    Ok(Migrated {
        kind: Kind::EventlogFile,
        types,
        source,
        tree,
        observations: state.join(OBSERVATIONS_DIR),
        kept,
    })
}

/// Splits `state/tree/`, which holds observations.
fn split(state: &Path) -> Result<Migrated> {
    let tree = state.join(TREE_DIR);
    let next = state.join(NEXT_DIR);
    let tenant = TenantId::new(CONDUCTOR)?;
    let runtime = Engine::start()?;
    let _lock = hold(&tree.join(TREE_LOCK))
        .with_context(|| format!("hold the writer lock of {}", tree.display()))?;
    if fs::symlink_metadata(&next).is_ok() {
        fs::remove_dir_all(&next).with_context(|| {
            format!(
                "remove {}, which an earlier run left unfinished",
                next.display()
            )
        })?;
    }
    let events = read_all(&runtime, &tree, &tenant)?;
    let types = move_records(&runtime, state, &tree, &events)?;
    let kept = state.join(format!(
        "{SPLIT_PREFIX}{}",
        stamp(OffsetDateTime::now_utc())
    ));
    fs::rename(&tree, &kept)
        .with_context(|| format!("rename {} to {}", tree.display(), kept.display()))?;
    fs::rename(&next, &tree)
        .with_context(|| format!("rename {} to {}", next.display(), tree.display()))?;
    Ok(Migrated {
        kind: Kind::Split,
        types,
        source: tree.clone(),
        tree,
        observations: state.join(OBSERVATIONS_DIR),
        kept,
    })
}

/// Finishes a split that stopped between its renames: no `tree/` and no File store are there,
/// `tree.next/` holds a tree store, and a `tree.split-*/` is beside it. Nothing else leaves
/// `tree.next/` without `tree/` and without a File store: a split renames `tree/` away only once
/// `tree.next/` is read back and checked, and a migration from the File store renames `tree.next/`
/// `tree/` before it renames the File store. `None` where that is not so.
fn finish_split(state: &Path) -> Result<Option<Migrated>> {
    let tree = state.join(TREE_DIR);
    let next = state.join(NEXT_DIR);
    if fs::symlink_metadata(&tree).is_ok()
        || !is_file(&next.join(TREE_MANIFEST))
        || fs::symlink_metadata(state.join(FILE_STORE_DIR).join(FILE_STORE_MANIFEST)).is_ok()
    {
        return Ok(None);
    }
    let mut kept: Vec<PathBuf> = fs::read_dir(state)
        .with_context(|| format!("read {}", state.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            (name.to_str()?.starts_with(SPLIT_PREFIX) && entry.file_type().ok()?.is_dir())
                .then(|| entry.path())
        })
        .collect();
    kept.sort();
    let Some(kept) = kept.pop() else {
        return Ok(None);
    };
    fs::rename(&next, &tree)
        .with_context(|| format!("rename {} to {}", next.display(), tree.display()))?;
    Ok(Some(Migrated {
        kind: Kind::Finished,
        types: BTreeMap::new(),
        source: next,
        tree,
        observations: state.join(OBSERVATIONS_DIR),
        kept,
    }))
}

/// Refuses where `tree` exists: the store is migrated already, or something else is there.
fn refuse_migrated(tree: &Path) -> Result<()> {
    if fs::symlink_metadata(tree).is_ok() {
        bail!(
            "{} exists: the store is migrated already, and nothing was changed",
            tree.display()
        );
    }
    Ok(())
}

/// Whether the tree at `tree` keeps a stream of an observation entity in conductor's tenant, by
/// the directories eventlog-tree keeps streams in (`tenants/<tenant>/streams/<type>/`); reading
/// them writes nothing.
fn holds_observations(tree: &Path) -> bool {
    let streams = tree
        .join("tenants")
        .join(segment(CONDUCTOR))
        .join("streams");
    observations::ENTITIES.iter().any(|entity| {
        fs::symlink_metadata(streams.join(segment(entity))).is_ok_and(|found| found.is_dir())
    })
}

/// `instant` as `YYYYMMDDTHHMMSSZ`.
fn stamp(instant: OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        instant.year(),
        u8::from(instant.month()),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second()
    )
}

/// Copies every directory and regular file under `from` to the new directory `to`.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)?;
        } else {
            return Err(io::Error::other(format!(
                "{} is neither a directory nor a regular file",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

/// Every event of `tenant` the tree store at `root` holds, in feed order, read from a handle
/// opened afresh, so from its files.
fn read_all(runtime: &Engine, root: &Path, tenant: &TenantId) -> Result<Vec<RecordedEvent>> {
    let tree = runtime
        .block_on(TreeEventStore::open(root))
        .with_context(|| format!("open the tree store at {}", root.display()))?;
    let mut events = Vec::new();
    let mut after = 0;
    loop {
        let page = runtime
            .block_on(tree.read_feed(tenant, after, MAX_READ_LIMIT))
            .with_context(|| format!("read the tree store at {}", root.display()))?;
        let more = page.has_more && !page.events.is_empty();
        after = page.next_position;
        events.extend(page.events);
        if !more {
            break;
        }
    }
    Ok(events)
}

/// The tenant of each observation stream of `events`, by `(stream type, stream id)`: the tenant of
/// the snapshot the newest of its records names.
///
/// # Errors
///
/// An observation stream none of whose records names a snapshot.
fn snapshot_tenants(
    events: &[RecordedEvent],
) -> Result<BTreeMap<(String, String), Option<TenantId>>> {
    let mut tenants: BTreeMap<(String, String), Option<TenantId>> = BTreeMap::new();
    for event in events {
        if !observations::holds(&event.stream_type) {
            continue;
        }
        let tenant = tenants
            .entry((event.stream_type.clone(), event.stream_id.clone()))
            .or_default();
        if let Some(snapshot) = event.data.get("snapshot_id").and_then(Value::as_str) {
            *tenant = Some(observations::tenant(snapshot).with_context(|| {
                format!(
                    "{} {:?} names snapshot {snapshot:?}",
                    event.stream_type, event.stream_id
                )
            })?);
        }
    }
    if let Some(((stream_type, stream_id), _)) = tenants.iter().find(|(_, tenant)| tenant.is_none())
    {
        bail!("{stream_type} {stream_id:?} names no snapshot: nothing was changed");
    }
    Ok(tenants)
}

/// Moves `events`, read from `source` in feed order: every observation into its snapshot's tenant
/// of `state/observations/`, every other record into a new tree at `state/tree.next/`; then reads
/// both back and compares them with `events`. Answers what it moved, per stream type.
fn move_records(
    runtime: &Engine,
    state: &Path,
    source: &Path,
    events: &[RecordedEvent],
) -> Result<BTreeMap<String, Moved>> {
    let next = state.join(NEXT_DIR);
    let observed = state.join(OBSERVATIONS_DIR);
    let tenant = TenantId::new(CONDUCTOR)?;
    let places = snapshot_tenants(events)?;
    let place = |event: &RecordedEvent| {
        places
            .get(&(event.stream_type.clone(), event.stream_id.clone()))
            .cloned()
            .flatten()
    };
    let (moved, kept): (Vec<&RecordedEvent>, Vec<&RecordedEvent>) =
        events.iter().partition(|event| place(event).is_some());
    let held_tree = summary(kept.iter().map(|event| (tenant.clone(), *event)))?;
    let held_observed = summary(
        moved
            .iter()
            .map(|event| (place(event).expect("partitioned on a place"), *event)),
    )?;

    append_all(
        runtime,
        &next,
        kept.iter().map(|event| (tenant.clone(), *event)),
        false,
    )?;
    // No observation, no observation store: opening one creates it.
    if !moved.is_empty() {
        append_all(
            runtime,
            &observed,
            moved
                .iter()
                .map(|event| (place(event).expect("partitioned on a place"), *event)),
            true,
        )?;
    }

    let written = read_back(runtime, &next, [tenant.clone()])?;
    if let Some(difference) = first_difference(&held_tree, &written, true) {
        bail!(
            "{} does not hold what {} holds ({difference}); {} is left for inspection and {} is \
             unchanged",
            next.display(),
            source.display(),
            next.display(),
            source.display()
        );
    }
    let routed: BTreeSet<String> = held_observed
        .keys()
        .map(|(tenant, _, _)| tenant.clone())
        .collect();
    let routed = routed
        .into_iter()
        .map(TenantId::new)
        .collect::<Result<Vec<_>, _>>()?;
    let written = if routed.is_empty() {
        Streams::new()
    } else {
        read_back(runtime, &observed, routed)?
    };
    if let Some(difference) = first_difference(&held_observed, &written, false) {
        bail!(
            "{} does not hold what {} holds ({difference}); {} is unchanged",
            observed.display(),
            source.display(),
            source.display()
        );
    }

    let mut types: BTreeMap<String, Moved> = BTreeMap::new();
    for ((_, stream_type, _), stream) in held_tree.iter().chain(&held_observed) {
        let moved = types.entry(stream_type.clone()).or_default();
        moved.streams += 1;
        moved.events += stream.events;
    }
    Ok(types)
}

/// Appends `events`, in their order, to the tree store at `root`, each to its stream in the tenant
/// it is paired with, at the version it held there. With `resume`, an event its stream already
/// holds, which a run that stopped appended, is not appended again; without it the tree is new.
fn append_all<'a>(
    runtime: &Engine,
    root: &Path,
    events: impl Iterator<Item = (TenantId, &'a RecordedEvent)>,
    resume: bool,
) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("create {}", root.display()))?;
    // A writer creating the same tree at once would write its description through one staging
    // name; the tree's writer lock makes it wait, as the store does when it creates a tree.
    let creating = if is_file(&root.join(TREE_MANIFEST)) {
        None
    } else {
        Some(hold(&root.join(TREE_LOCK)).with_context(|| format!("create {}", root.display()))?)
    };
    let tree = runtime
        .block_on(TreeEventStore::open(root))
        .with_context(|| format!("create the tree store at {}", root.display()))?;
    // Each append takes the writer lock itself.
    drop(creating);
    for (tenant, event) in events {
        let what = || {
            format!(
                "append version {} of {} {:?} to {}",
                event.version,
                event.stream_type,
                event.stream_id,
                root.display()
            )
        };
        let stream = StreamId::new(
            tenant,
            event.stream_type.clone(),
            held(&event.stream_id).with_context(what)?,
        )
        .with_context(what)?;
        if resume
            && runtime
                .block_on(tree.stream_version(&stream))
                .with_context(what)?
                .is_some_and(|held| held >= event.version)
        {
            continue;
        }
        let expected = match event.version {
            1 => Expected::NoStream,
            version => Expected::Exact(version - 1),
        };
        let appended =
            [
                NewEvent::new(event.name.clone(), event.schema_version, event.data.clone())
                    .with_context(what)?,
            ];
        let id = new_event_id();
        let meta = CommandMeta {
            idempotency_key: id.clone(),
            request_hash: request_hash(&appended).with_context(what)?,
            subject: event.subject.clone(),
            actor: event.actor.clone(),
            request_id: id.clone(),
            trace_id: id,
            causation_id: None,
            causation_depth: 0,
            occurred_at: event.occurred_at,
            claim: None,
        };
        runtime
            .block_on(tree.append(&stream, expected, &appended, &meta))
            .with_context(what)?;
    }
    Ok(())
}

/// Every stream of `tenants` the tree store at `root` holds, read from a handle opened afresh, so
/// from its files.
fn read_back(
    runtime: &Engine,
    root: &Path,
    tenants: impl IntoIterator<Item = TenantId>,
) -> Result<Streams> {
    let tree = runtime
        .block_on(TreeEventStore::open(root))
        .with_context(|| format!("reopen the tree store at {}", root.display()))?;
    let mut events = Vec::new();
    for tenant in tenants {
        let mut after = 0;
        loop {
            let page = runtime
                .block_on(tree.read_feed(&tenant, after, MAX_READ_LIMIT))
                .with_context(|| format!("read the tree store at {}", root.display()))?;
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            events.extend(page.events);
            if !more {
                break;
            }
        }
    }
    let read: Vec<(TenantId, &RecordedEvent)> = events
        .iter()
        .map(|event| (event.tenant.clone(), event))
        .collect();
    summary(read.into_iter())
}

/// One stream's events, as a migration compares them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stream {
    /// How many events it holds.
    events: usize,
    /// The digest of its events' names, schema versions and bodies, in their order.
    digest: String,
}

/// Each stream, by `(tenant, stream type, stream id)`.
type Streams = BTreeMap<(String, String, String), Stream>;

/// Each stream of `events`, which are in feed order, so each stream's in its version order, in the
/// tenant each is paired with, by the id the tree keeps it under.
fn summary<'a>(events: impl Iterator<Item = (TenantId, &'a RecordedEvent)>) -> Result<Streams> {
    let mut streams: BTreeMap<(String, String, String), Vec<NewEvent>> = BTreeMap::new();
    for (tenant, event) in events {
        streams
            .entry((
                tenant.as_str().to_owned(),
                event.stream_type.clone(),
                held(&event.stream_id)?,
            ))
            .or_default()
            .push(NewEvent {
                name: event.name.clone(),
                schema_version: event.schema_version,
                data: event.data.clone(),
            });
    }
    streams
        .into_iter()
        .map(|(key, events)| {
            let digest = request_hash(&events)
                .with_context(|| format!("digest the stream {} {:?}", key.1, key.2))?;
            Ok((
                key,
                Stream {
                    events: events.len(),
                    digest,
                },
            ))
        })
        .collect()
}

/// The first stream the two do not hold alike, named, or `None` when `written` holds every stream
/// of `held` alike and, when `exact`, no other.
fn first_difference(held: &Streams, written: &Streams, exact: bool) -> Option<String> {
    for (key, stream) in held {
        match written.get(key) {
            None => return Some(format!("no stream {} {:?}", key.1, key.2)),
            Some(other) if other.events != stream.events => {
                return Some(format!(
                    "{} events in {} {:?}, not {}",
                    other.events, key.1, key.2, stream.events
                ));
            }
            Some(other) if other.digest != stream.digest => {
                return Some(format!("other events in {} {:?}", key.1, key.2));
            }
            Some(_) => {}
        }
    }
    if !exact {
        return None;
    }
    written
        .keys()
        .find(|key| !held.contains_key(*key))
        .map(|key| format!("a stream {} {:?} the source does not hold", key.1, key.2))
}
