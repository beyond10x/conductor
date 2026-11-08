//! `conductor store migrate` (`story:store-shared`, `conductor.store.MigrateStore`): reads the
//! eventlog tree under the state directory once into the store's database, records the migration
//! there (`conductor.store.Migration`, keyed by the instance) and is the only reader of the tree
//! afterwards. The tree is left where it is; every other command refuses a state directory whose
//! tree the database holds no migration of.
//!
//! The tree is the one the builds before the database kept: `tree/` (every record but the
//! observations, in the tenant `conductor`), `observations/` (each snapshot's observations in a
//! tenant named after it) and `series/` (the daily rollups). Every event is appended anew, in
//! feed order, to its stream in the instance's tenant, or, for an observation, in the tenant of
//! its snapshot (`<instance>/observations/<snapshot>`), at the version it held in the tree, with
//! its name, schema version, body and instant; the database assigns its own ids and positions,
//! and every event names the instance. Each stream's event count and digest is then read back from
//! the database and compared with the tree's before the migration is recorded.
//!
//! An older state directory is first brought to that tree as the build before this one did: an
//! eventlog-file store at `store/` is moved into `tree/` and `observations/` (kept beside them as
//! `store.eventlog-file-<UTC stamp>/`), and a `tree/` that holds observations is split.
//!
//! It refuses, copying nothing, a non-human actor (`ActorNotHuman`), an instance whose tree the
//! database holds already (`AlreadyMigrated`), and a state directory that holds no tree
//! (`SourceMissing`).
//!
//! The steps that bring an older state directory to the tree, as the build before wrote them:
//!
//! From the File store, the run holds the File store's writer lock from before it reads the store
//! until it has renamed it, so no writer of that store appends in between. The history is read
//! from a copy made under the lock, `store.migrating/`, through eventlog's own inspection of a
//! File store, and the copy is removed once read. An observation is appended to `observations/`,
//! in the tenant of the snapshot the newest record of its stream names; every other record to
//! `tree.next/`. Each stream's event count and digest is then read back and compared with the
//! source's. Only then is `tree.next/` renamed `tree/`, and `store/` renamed
//! `store.eventlog-file-<UTC stamp>/`, which nothing deletes. A split holds `tree/`'s writer lock,
//! moves its records as above, then renames `tree/` `tree.split-<UTC stamp>/`, which nothing
//! deletes, and `tree.next/` `tree/`; a split that stopped between its renames is finished by the
//! next run.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use conductor_model::behaviour::MigrationStorage;
use conductor_model::config::InstanceName;
use conductor_model::operator::ActorKind;
use conductor_model::primitives::Timestamp;
use conductor_model::store::{MigrationData, MigrationSnapshot, MigrationState, StoreUrl};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, InspectHistory as _, InspectionLimits, MAX_READ_LIMIT,
    NewEvent, RecordedEvent, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_file::{FileEventStore, FileHistoryInspector};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::observations::{self, segment};
use super::{
    CONDUCTOR, DELETED, Engine, FILE_STORE_DIR, FILE_STORE_MANIFEST, OBSERVATIONS_DIR, SERIES_DIR,
    STORED, Store, StoreError, TREE_DIR, TREE_LOCK, TREE_MANIFEST, held, hold, is_file, text,
    undecodable,
};
use crate::cli::StoreMigrateArgs;

/// `conductor.store.Migration`.
const MIGRATION: &str = "conductor.store.Migration";

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

/// What a step that brought an older state directory to the tree did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Moved the File provider's store.
    EventlogFile,
    /// Split a `tree/` that held observations.
    Split,
    /// Finished a split that stopped between its two renames: renamed `tree.next/` `tree/`.
    Finished,
}

/// What a step that brought an older state directory to the tree moved, and where it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrated {
    /// What the step did.
    pub kind: Kind,
    /// Per stream type, what was moved; nothing for [`Kind::Finished`].
    pub types: BTreeMap<String, Moved>,
    /// What the records were moved from.
    pub source: PathBuf,
    /// The tree store, `state/tree/`.
    pub tree: PathBuf,
    /// The observation store, `state/observations/`.
    pub observations: PathBuf,
    /// Where what the records were moved from is kept.
    pub kept: PathBuf,
}

/// What `store migrate` read into the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// The step that brought an older state directory to the tree first, if one ran.
    pub before: Option<Migrated>,
    /// Per stream type (an entity's qualified name), what was read.
    pub types: BTreeMap<String, Moved>,
    /// The tree read: `state/tree/`.
    pub source: PathBuf,
    /// The database it was read into.
    pub store: String,
    /// The instance whose records they are.
    pub instance: String,
}

impl Read {
    /// Every stream read.
    #[must_use]
    pub fn streams(&self) -> usize {
        self.types.values().map(|moved| moved.streams).sum()
    }

    /// Every event read.
    #[must_use]
    pub fn events(&self) -> usize {
        self.types.values().map(|moved| moved.events).sum()
    }
}

/// `conductor store migrate`: [`migrate`] on the state directory (`--state-dir`, else the
/// instance's), then the counts it read on standard output, as text or, with `--format json`, one
/// JSON object. A refusal is one line on standard error naming the specification's error.
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
    let read = migrate(&state).context(COMMAND)?;
    let rendered = if as_json {
        json_report(&read)
    } else {
        text_report(&read)
    };
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(rendered.as_bytes())
        .and_then(|()| stdout.flush())
        .context("write the counts read to standard output")?;
    Ok(ExitCode::SUCCESS)
}

fn text_report(read: &Read) -> String {
    let mut text = String::new();
    if let Some(before) = &read.before {
        let kept = match before.kind {
            Kind::EventlogFile => "the eventlog-file store",
            Kind::Split | Kind::Finished => "the tree before the split",
        };
        text.push_str(&format!(
            "brought {} to the tree at {}; {kept} is kept at {}\n",
            before.source.display(),
            before.tree.display(),
            before.kept.display()
        ));
    }
    for (stream_type, moved) in &read.types {
        text.push_str(&format!(
            "{stream_type}: {} streams, {} events\n",
            moved.streams, moved.events
        ));
    }
    text.push_str(&format!(
        "read {} events in {} streams from {} into {} as instance {}; the tree is not read again\n",
        read.events(),
        read.streams(),
        read.source.display(),
        read.store,
        read.instance
    ));
    text
}

fn json_report(read: &Read) -> String {
    let types: Vec<_> = read
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
    let before = read.before.as_ref().map(|before| {
        json!({
            "kind": match before.kind {
                Kind::EventlogFile => "eventlog-file",
                Kind::Split => "split",
                Kind::Finished => "finished-split",
            },
            "source": before.source.display().to_string(),
            "kept": before.kept.display().to_string(),
        })
    });
    format!(
        "{}\n",
        json!({
            "instance": read.instance,
            "streams": read.streams(),
            "events": read.events(),
            "types": types,
            "source": read.source.display().to_string(),
            "store": read.store,
            "before": before,
        })
    )
}

/// Reads the tree under `state` into the database, as the module says, and answers what it read.
///
/// # Errors
///
/// The actor is not a person (`ActorNotHuman`), the database holds the instance's migration
/// already (`AlreadyMigrated`), or `state` holds no tree (`SourceMissing`): nothing is copied.
/// Or the tree does not read, an event is refused by the database, or the database read back does
/// not hold what the tree held (then no migration is recorded).
pub fn migrate(state: &Path) -> Result<Read> {
    let actor = crate::actor::chosen();
    if actor.kind != ActorKind::Human {
        bail!("ActorNotHuman: {}", crate::actor::not_human(&actor));
    }
    let source = state.join(TREE_DIR);
    let missing = || {
        format!(
            "SourceMissing: no eventlog tree at {}, so nothing was copied",
            source.display()
        )
    };
    // What `to_tree` reads from, decided before anything is opened: a tree, a File store with no
    // `tree/` in its way, or a split a run left between its renames.
    let tree_there = fs::symlink_metadata(&source).is_ok();
    let readable =
        is_file(&source.join(TREE_MANIFEST)) || (!tree_there && super::holds_tree(state));
    if !readable {
        bail!(missing());
    }
    let mut store = super::open_for_migration(state)?;
    let instance = store.instance().to_owned();
    if store.migrated() {
        bail!(
            "AlreadyMigrated: the tree of instance {instance} is in the store already; it is read \
             once and not opened again, so nothing was copied"
        );
    }
    let before = to_tree(state)?;
    if !is_file(&source.join(TREE_MANIFEST)) {
        bail!(missing());
    }
    let types = into_database(&store, state)?;
    let read = Read {
        before,
        types,
        source: source.clone(),
        store: store.location().shown(),
        instance: instance.clone(),
    };
    let copied = i64::try_from(read.events()).unwrap_or(i64::MAX);
    let url = StoreUrl(store.location().url());
    MigrationStorage::put(
        &mut store,
        MigrationSnapshot {
            state: MigrationState::Migrated,
            data: MigrationData {
                instance: InstanceName(instance),
                source: source.display().to_string(),
                store: url,
                events_copied: copied,
                migrated_at: Timestamp(
                    OffsetDateTime::now_utc()
                        .format(&Rfc3339)
                        .context("write the instant")?,
                ),
                actor,
            },
        },
    );
    store
        .check()
        .context("the store did not keep the migration")?;
    Ok(read)
}

/// Brings an older state directory to the tree: moves a File store, splits a tree that holds
/// observations, or finishes a split; `None` where the tree is as this build reads it, or no store
/// is there at all.
fn to_tree(state: &Path) -> Result<Option<Migrated>> {
    let tree = state.join(TREE_DIR);
    if fs::symlink_metadata(&tree).is_ok() {
        if is_file(&tree.join(TREE_MANIFEST)) && holds_observations(&tree) {
            return split(state).map(Some);
        }
        return Ok(None);
    }
    if let Some(finished) = finish_split(state)? {
        return Ok(Some(finished));
    }
    if fs::symlink_metadata(state.join(FILE_STORE_DIR).join(FILE_STORE_MANIFEST)).is_ok() {
        return from_file(state).map(Some);
    }
    Ok(None)
}

/// Copies every event of the tree under `state` into `store`, then reads each stream back and
/// compares it with the tree's; answers what it copied, per stream type.
fn into_database(store: &Store, state: &Path) -> Result<BTreeMap<String, Moved>> {
    let runtime = &store.runtime;
    let conductor = TenantId::new(CONDUCTOR)?;
    let mut pairs: Vec<(TenantId, RecordedEvent)> = Vec::new();
    for dir in [TREE_DIR, SERIES_DIR] {
        let root = state.join(dir);
        if !is_file(&root.join(TREE_MANIFEST)) {
            continue;
        }
        for event in read_all(runtime, &root, &conductor)? {
            pairs.push((store.tenant.clone(), event));
        }
    }
    let observed = state.join(OBSERVATIONS_DIR);
    if is_file(&observed.join(TREE_MANIFEST)) {
        for tenant in observations::tenants(&observed)
            .with_context(|| format!("read the tenants of {}", observed.display()))?
        {
            let into = observations::tenant_of_stream(store.instance(), tenant.as_str())?;
            for event in read_all(runtime, &observed, &tenant)? {
                pairs.push((into.clone(), event));
            }
        }
    }
    let held = summary(pairs.iter().map(|(tenant, event)| (tenant.clone(), event)))?;
    for (tenant, event) in &pairs {
        let what = || {
            format!(
                "copy version {} of {} {:?} into the store",
                event.version, event.stream_type, event.stream_id
            )
        };
        let stream = StreamId::new(
            tenant.clone(),
            event.stream_type.clone(),
            held_id(&event.stream_id).with_context(what)?,
        )
        .with_context(what)?;
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
            subject: store.instance().to_owned(),
            actor: event.actor.clone(),
            request_id: id.clone(),
            trace_id: id,
            causation_id: None,
            causation_depth: 0,
            occurred_at: event.occurred_at,
            claim: None,
        };
        runtime
            .block_on(store.log.append(&stream, expected, &appended, &meta))
            .with_context(what)?;
    }
    let tenants: BTreeSet<String> = held.keys().map(|(tenant, _, _)| tenant.clone()).collect();
    let mut written = Vec::new();
    for tenant in tenants {
        let tenant = TenantId::new(tenant)?;
        let mut after = 0;
        loop {
            let page = runtime
                .block_on(store.log.read_feed(&tenant, after, MAX_READ_LIMIT))
                .context("read the store back")?;
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            written.extend(page.events);
            if !more {
                break;
            }
        }
    }
    let written = summary(written.iter().map(|event| (event.tenant.clone(), event)))?;
    if let Some(difference) = first_difference(&held, &written, true) {
        bail!(
            "the store does not hold what {} holds ({difference}); no migration is recorded",
            state.join(TREE_DIR).display()
        );
    }
    let mut types: BTreeMap<String, Moved> = BTreeMap::new();
    for ((_, stream_type, _), stream) in &held {
        let moved = types.entry(stream_type.clone()).or_default();
        moved.streams += 1;
        moved.events += stream.events;
    }
    Ok(types)
}

/// The stream id the store keeps the stream `id` under: [`held`].
fn held_id(id: &str) -> Result<String> {
    held(id).map_err(Into::into)
}

impl MigrationStorage for Store {
    fn get(&self, identity: &InstanceName) -> Option<MigrationSnapshot> {
        let body = self.read(MIGRATION, &identity.0)?;
        match decode_migration(&body) {
            Ok(migration) if migration.data.instance == *identity => Some(migration),
            Ok(migration) => {
                self.fail(undecodable(
                    MIGRATION,
                    &identity.0,
                    format!("holds instance {:?}", migration.data.instance.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(MIGRATION, &identity.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: MigrationSnapshot) {
        let body = encode_migration(&snapshot);
        self.write(MIGRATION, &snapshot.data.instance.0, STORED, body);
    }

    fn delete(&mut self, identity: &InstanceName) {
        self.write(MIGRATION, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<MigrationSnapshot> {
        self.rows(MIGRATION)
            .into_iter()
            .filter_map(|(stream, body)| match decode_migration(&body) {
                Ok(migration) => Some(migration),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: MIGRATION,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_migration(migration: &MigrationSnapshot) -> Value {
    json!({
        "state": "Migrated",
        "instance": migration.data.instance.0,
        "source": migration.data.source,
        "store": migration.data.store.0,
        "events_copied": migration.data.events_copied,
        "migrated_at": migration.data.migrated_at.0,
        "actor": crate::actor::to_json(&migration.data.actor),
    })
}

fn decode_migration(body: &Value) -> Result<MigrationSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Migrated" {
        return Err(format!("unknown Migration state {state:?}"));
    }
    Ok(MigrationSnapshot {
        state: MigrationState::Migrated,
        data: MigrationData {
            instance: InstanceName(text(body, "instance")?),
            source: text(body, "source")?,
            store: StoreUrl(text(body, "store")?),
            events_copied: body
                .get("events_copied")
                .and_then(Value::as_i64)
                .ok_or_else(|| "events_copied is not a whole number".to_owned())?,
            migrated_at: Timestamp(text(body, "migrated_at")?),
            actor: crate::actor::stored(body)?,
        },
    })
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
            *tenant = Some(observations::tree_tenant(snapshot).with_context(|| {
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
