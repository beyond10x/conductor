//! A store of the size `story:status-current-snapshot` was measured on: an instance whose
//! `<state>/tree` holds 50,000 files and whose observations hold 14 snapshots, with the host
//! ledger an older binary kept in the instance's store, thousands of grants long. [`plant`] builds
//! it under a state directory; `tests/status.rs` and `tests/dashboard_fast.rs` read it.
//!
//! - **The tree.** `tree/` holds the eventlog tree the builds before the database kept
//!   (`store.json` and [`TREE_FILES`] files under `tenants/conductor/streams/`), and the database
//!   holds its migration (`conductor.store.Migration`), as after `conductor store migrate`: every
//!   command opens the database, and none needs to read the tree.
//! - **The grants.** [`GRANTS`] `conductor.host.HostGrant` streams in the instance's tenant, each
//!   asked, granted, running, released and then deleted, as the move of the host ledger to the
//!   host directory leaves them (`story:instance-isolation`); `host.migrated` marks that move done.
//!   Each body is the one the store writes, varied by its id and state.
//! - **The snapshots.** [`SNAPSHOTS`] complete snapshots, an hour apart, each of [`REPOSITORIES`]
//!   repositories with [`PER`] pull requests and `main` workflow runs, a blocker and a
//!   specification, taken through the snapshot driver after the grants were written, as the
//!   instance's runtime takes them, so the store's index is as of the newest one. Retention keeps
//!   the newest 12; the observations of the two it dropped are put back, so the store holds 14
//!   snapshots'.
//! - **The older records.** Each snapshot but the newest also holds one repository observation
//!   this build cannot read ([`UNREADABLE`]): a read of an older snapshot names it.
//!
//! The bulk is written to the database through Eventlog's SQLite adapter, a thousand streams to a
//! transaction, so the store is built in seconds rather than in one transaction per record.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store::{self, observations};
use conductor_model::behaviour::{HostGrantStorage, MigrationStorage};
use conductor_model::config::InstanceName;
use conductor_model::host::{GrantId, GrantKind, HostGrantData, HostGrantSnapshot, HostGrantState};
use conductor_model::observation::{
    CommitSha, Mergeability, RecordBlocker, RecordPullRequest, RecordRepository,
    RecordSpecification, RecordWorkflowRun, RepositoryName, RunConclusion, SnapshotState,
    SpecificationPresence, StartSnapshot, ValidationResult, Visibility,
};
use conductor_model::primitives::Timestamp;
use conductor_model::store::{MigrationData, MigrationSnapshot, MigrationState, StoreUrl};
use eventlog_core::{
    AppendGroup, AtomicEventStore as _, CommandMeta, EventStore as _, Expected, MAX_READ_LIMIT,
    NewEvent, RecordedEvent, StreamAppend, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_sqlite::SqliteEventStore;
use serde_json::json;
use time::OffsetDateTime;
use tokio::runtime::Runtime;

/// The files under `tree/`.
pub const TREE_FILES: usize = 50_000;

/// The complete snapshots whose observations the store holds.
pub const SNAPSHOTS: usize = 14;

/// The grants of the host ledger an older binary kept in the instance's store.
pub const GRANTS: usize = 7_000;

/// The repositories each snapshot observes.
pub const REPOSITORIES: usize = 40;

/// The pull requests and workflow runs each snapshot observes of each repository.
pub const PER: usize = 6;

/// The repository each older snapshot holds an unreadable observation of.
pub const UNREADABLE: &str = "unreadable-older";

/// The instance's tenant: a `--state-dir` store's.
const TENANT: &str = "conductor";

/// The streams of one transaction.
const BATCH: usize = 1_000;

/// When the snapshot `n` (0 the oldest) started.
fn started_at(n: usize) -> String {
    format!("2026-11-08T{n:02}:00:00Z")
}

/// Holds the one lock of the cases that read this store, across their test binaries, until the
/// answer is dropped: a case's timed calls never share the host with another case planting its
/// store, which writes tens of thousands of files and records at full speed.
pub fn alone() -> fs::File {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sized.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .expect("open the lock of the sized cases");
    file.lock().expect("take the lock of the sized cases");
    file
}

/// Builds the store under `state`, as the module says; the id of the newest snapshot.
pub fn plant(state: &Path) -> String {
    super::active::isolate();
    migrated(state);
    let runtime = runtime();
    let log = runtime
        .block_on(SqliteEventStore::open(
            state
                .join(store::STORE_FILE)
                .to_str()
                .expect("a UTF-8 path"),
            store::PREFIX,
        ))
        .expect("open the database");
    grants(state, &runtime, &log);
    let mut ids = Vec::new();
    for n in 0..SNAPSHOTS {
        ids.push(take(state, &started_at(n)));
    }
    let newest = ids.last().expect("a snapshot").clone();
    let full = observation_tenant(&newest);
    let template = feed(&runtime, &log, &full);
    for id in &ids[..SNAPSHOTS - 1] {
        let tenant = observation_tenant(id);
        if feed(&runtime, &log, &tenant).is_empty() {
            copy(&runtime, &log, &template, &tenant, id);
        }
        unreadable(&runtime, &log, &tenant, id);
    }
    tree(state);
    newest
}

/// Takes one complete snapshot of the fixture, started at `started_at`; its id.
fn take(state: &Path, started_at: &str) -> String {
    let store = store::open(state).expect("open the store");
    let start = StartSnapshot {
        actor: conductor_cli::actor::daemon(),
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let taken = snapshot::take_from(store, start, &[collector()]).expect("the driver ends");
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    taken.snapshot_id.0.0
}

/// A collector of [`REPOSITORIES`] repositories `repo-00`, `repo-01`, …, each with [`PER`] open
/// pull requests (every other one with a failing check) and `main` workflow runs, one blocker and
/// one specification.
fn collector() -> Collector {
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let at = |day: usize| Timestamp(format!("2026-11-{:02}T09:00:00Z", 1 + day % 7));
        for index in 0..REPOSITORIES {
            let repository = RepositoryName(format!("repo-{index:02}"));
            record.repository(RecordRepository {
                actor: conductor_cli::actor::daemon(),
                snapshot_id: snapshot.clone(),
                repository: repository.clone(),
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: CommitSha(format!("head-of-{index}")),
                main_committed_at: at(index),
                main_commits_7d: 2,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: 1,
                oldest_open_issue_at: None,
                latest_release: Some("v1.0.0".to_owned()),
                latest_release_at: Some(at(index)),
                unreleased_commits: Some(0),
                planning_store_version: None,
                in_catalog: None,
                gates_hooks_version: None,
                gates_version: None,
                wave_width: None,
                wave_ready: None,
                wave_reason: None,
            })?;
            for number in 0..PER {
                let number = i64::try_from(number + 1).expect("a small number");
                record.pull_request(RecordPullRequest {
                    actor: conductor_cli::actor::daemon(),
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    number,
                    title: format!("Change {number}"),
                    draft: false,
                    mergeability: Mergeability::Mergeable,
                    failing_checks: number % 2,
                    failing_check_names: Vec::new(),
                    opened_at: at(index),
                    updated_at: at(index),
                    url: None,
                })?;
                record.workflow_run(RecordWorkflowRun {
                    actor: conductor_cli::actor::daemon(),
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    workflow: format!("workflow-{number}"),
                    conclusion: RunConclusion::Success,
                    run_at: at(index),
                })?;
            }
            record.blocker(RecordBlocker {
                actor: conductor_cli::actor::daemon(),
                snapshot_id: snapshot.clone(),
                repository: repository.clone(),
                reference: format!("blocker:{index}"),
                blocker_kind: "decision".to_owned(),
                title: "Which token".to_owned(),
            })?;
            record.specification(RecordSpecification {
                actor: conductor_cli::actor::daemon(),
                snapshot_id: snapshot.clone(),
                repository,
                presence: SpecificationPresence::Present,
                path: Some("spec".to_owned()),
                format: Some("ess/22".to_owned()),
                required_ess: None,
                validation: ValidationResult::Valid,
                validation_refusals: 0,
                scenarios: Some(10),
                synthesis_refusals: None,
                conformance_status: None,
            })?;
        }
        Ok(())
    })
}

fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime")
}

fn meta(events: &impl serde::Serialize) -> CommandMeta {
    let id = new_event_id();
    CommandMeta {
        idempotency_key: id.clone(),
        request_hash: request_hash(events).expect("a hash"),
        subject: TENANT.to_owned(),
        actor: TENANT.to_owned(),
        request_id: id.clone(),
        trace_id: id,
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
        claim: None,
    }
}

/// The tenant of the observations of the snapshot `id` of a `--state-dir` store.
fn observation_tenant(id: &str) -> TenantId {
    observations::tenant(TENANT, id).expect("a tenant")
}

/// Every event of `tenant`, in feed order.
fn feed(runtime: &Runtime, log: &SqliteEventStore, tenant: &TenantId) -> Vec<RecordedEvent> {
    let mut events = Vec::new();
    let mut after = 0;
    loop {
        let page = runtime
            .block_on(log.read_feed(tenant, after, MAX_READ_LIMIT))
            .expect("read the feed");
        let more = page.has_more && !page.events.is_empty();
        after = page.next_position;
        events.extend(page.events);
        if !more {
            return events;
        }
    }
}

/// Appends `appends` to `tenant`, [`BATCH`] streams to a transaction.
fn append(
    runtime: &Runtime,
    log: &SqliteEventStore,
    tenant: &TenantId,
    appends: Vec<StreamAppend>,
) {
    let mut appends = appends.into_iter().peekable();
    while appends.peek().is_some() {
        let batch: Vec<StreamAppend> = appends.by_ref().take(BATCH).collect();
        let group = AppendGroup {
            tenant: tenant.clone(),
            meta: meta(&batch.iter().map(|one| &one.events).collect::<Vec<_>>()),
            appends: batch,
        };
        runtime
            .block_on(log.append_group(&group))
            .expect("plant a batch");
    }
}

/// Puts the observations `template` (another snapshot's) into `tenant` as the snapshot `id`'s:
/// what retention dropped of an older snapshot.
fn copy(
    runtime: &Runtime,
    log: &SqliteEventStore,
    template: &[RecordedEvent],
    tenant: &TenantId,
    id: &str,
) {
    let mut streams: BTreeMap<(String, String), Vec<NewEvent>> = BTreeMap::new();
    for event in template {
        let mut data = event.data.clone();
        if data.get("snapshot_id").is_some() {
            data["snapshot_id"] = json!(id);
        }
        streams
            .entry((event.stream_type.clone(), event.stream_id.clone()))
            .or_default()
            .push(NewEvent::new(event.name.clone(), event.schema_version, data).expect("an event"));
    }
    let appends = streams
        .into_iter()
        .map(|((stream_type, stream_id), events)| StreamAppend {
            stream: StreamId::new(tenant.clone(), stream_type, stream_id).expect("a stream"),
            expected: Expected::NoStream,
            events,
        })
        .collect();
    append(runtime, log, tenant, appends);
}

/// One repository observation of the snapshot `id`, in its `tenant`, that this build cannot read:
/// its visibility is none the specification names.
fn unreadable(runtime: &Runtime, log: &SqliteEventStore, tenant: &TenantId, id: &str) {
    let template = feed(runtime, log, tenant)
        .into_iter()
        .find(|event| event.stream_type == "conductor.observation.RepositoryObservation")
        .expect("a repository observation");
    let mut data = template.data;
    data["observation_id"] = json!(format!("{UNREADABLE}-{id}"));
    data["repository"] = json!(UNREADABLE);
    data["visibility"] = json!("Translucent");
    let appends = vec![StreamAppend {
        stream: StreamId::new(
            tenant.clone(),
            "conductor.observation.RepositoryObservation",
            format!("{UNREADABLE}-{id}"),
        )
        .expect("a stream"),
        expected: Expected::NoStream,
        events: vec![NewEvent::new("stored", 1, data).expect("an event")],
    }];
    append(runtime, log, tenant, appends);
}

/// Records the migration of the instance's tree, as `conductor store migrate` does, and the move
/// of the host ledger to the host directory.
fn migrated(state: &Path) {
    let mut store = store::open(state).expect("open the store");
    let instance = InstanceName(store.instance().to_owned());
    MigrationStorage::put(
        &mut store,
        MigrationSnapshot {
            state: MigrationState::Migrated,
            data: MigrationData {
                instance: instance.clone(),
                source: state.join("tree").display().to_string(),
                store: StoreUrl(format!(
                    "sqlite:{}",
                    state.join(store::STORE_FILE).display()
                )),
                events_copied: 50_000,
                migrated_at: Timestamp("2026-11-08T03:30:00Z".to_owned()),
                actor: conductor_cli::actor::daemon(),
            },
        },
    );
    store.check().expect("record the migration");
    fs::write(state.join("host.migrated"), "").expect("mark the host ledger moved");
}

/// The [`GRANTS`] grants of the host ledger an older binary kept in the instance's store, each
/// asked, granted, running, released and deleted. The body is the one the store writes for a
/// grant, put once through its port and varied by id and state.
fn grants(state: &Path, runtime: &Runtime, log: &SqliteEventStore) {
    const TYPE: &str = "conductor.host.HostGrant";
    let mut store = store::open(state).expect("open the store");
    let instance = InstanceName(store.instance().to_owned());
    HostGrantStorage::put(
        &mut store,
        HostGrantSnapshot {
            state: HostGrantState::Released,
            data: HostGrantData {
                grant_id: GrantId("grant-template".to_owned()),
                instance: instance.clone(),
                repository: Some(RepositoryName("repo-00".to_owned())),
                kind: GrantKind::Disk,
                bytes: Some(8_000_000_000),
                peak_bytes: Some(6_000_000_000),
                pauses: Some(0),
                command: Some("cargo test -p conductor-cli".to_owned()),
                process_group: Some(4242),
                session_name: None,
                session_id: None,
                reclaimed_bytes: None,
                shortfall_bytes: None,
                requested_at: Timestamp("2026-11-01T09:00:00Z".to_owned()),
                started_at: Some(Timestamp("2026-11-01T09:00:05Z".to_owned())),
                released_at: Some(Timestamp("2026-11-01T09:04:00Z".to_owned())),
                release_reason: None,
                cache_hits: Some(10),
                cache_misses: Some(2),
                lane: None,
                lane_overrun: None,
                actor: conductor_cli::actor::daemon(),
            },
        },
    );
    store.check().expect("put the template grant");
    drop(store);
    let tenant = TenantId::new(TENANT).expect("the tenant");
    let template = feed(runtime, log, &tenant)
        .into_iter()
        .find(|event| event.stream_type == TYPE)
        .expect("the template grant")
        .data;
    let appends = (0..GRANTS)
        .map(|n| {
            let id = format!("grant-{n:05}");
            let events = ["Requested", "Granted", "Running", "Released"]
                .into_iter()
                .map(|state| {
                    let mut data = template.clone();
                    data["grant_id"] = json!(id);
                    data["state"] = json!(state);
                    NewEvent::new("stored", 1, data).expect("an event")
                })
                .chain([NewEvent::new("deleted", 1, json!({})).expect("an event")])
                .collect();
            StreamAppend {
                stream: StreamId::new(tenant.clone(), TYPE, id).expect("a stream"),
                expected: Expected::NoStream,
                events,
            }
        })
        .collect();
    append(runtime, log, &tenant, appends);
}

/// The tree the builds before the database kept: its `store.json` and [`TREE_FILES`] files.
fn tree(state: &Path) {
    let streams = state.join("tree/tenants/conductor/streams");
    for n in 0..TREE_FILES {
        let dir = streams.join(format!("conductor.host.HostGrant/grant-{:05}", n / 5));
        if n % 5 == 0 {
            fs::create_dir_all(&dir).expect("create a stream's directory");
        }
        fs::write(dir.join(format!("{:020}.json", n % 5 + 1)), b"{}").expect("write a tree file");
    }
    fs::write(state.join("tree/store.json"), b"{}").expect("write the tree's store.json");
}
