//! `story:snapshot-store-cost`: a snapshot reads the store in proportion to what it records, not
//! all of it, however often other writers commit to `state/tree/` while it runs; and a snapshot
//! whose process is gone never stays `Collecting`.
//!
//! The cost is read from `/proc/self/io` (`rchar`: every byte this process read, from the page
//! cache or the disk), so the cost cases run on Linux only. Another writer, as the guard's hook or
//! a dispatch is while a snapshot collects, is a tree handle of its own on `state/tree/`, opened
//! before the measurement, that commits one record between two observations.
//!
//! A case keeps its state directory in this test target's temporary directory in `target/`, and
//! fixes this process's instance with [`active::isolate`]. The measurement over a copy of a live
//! store is ignored by default; it reads only the directory `SNAPSHOT_COST_STATE` names, which
//! must lie under a `target/`.

mod active;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::{CollectorRunObservationStorage, SnapshotStorage};
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotId, SnapshotState, StartSnapshot,
    Visibility,
};
use conductor_model::primitives::Timestamp;
use eventlog_core::{
    AtomicEventStore as _, CommandMeta, EventStore as _, Expected, NewEvent, StreamAppend,
    StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::json;
use time::OffsetDateTime;
use tokio::runtime::Runtime;

/// `conductor.observation.Snapshot`, which is kept in `state/tree/`.
const SNAPSHOT: &str = "conductor.observation.Snapshot";

/// The stream type the other writer commits: no entity of conductor's, so no view reads it.
const BESIDE: &str = "conductor.test.Beside";

/// The records planted in `state/tree/` of a fixture, so that reading the tree once costs far
/// more than recording one observation.
const PLANTED: usize = 1500;

/// The observations the smaller of the two snapshots records; the larger records ten times as
/// many.
const FEW: usize = 8;

/// A fresh state directory for the case `name`.
fn case(name: &str) -> PathBuf {
    active::isolate();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("snapshot_store_cost")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case's directory");
    }
    fs::create_dir_all(&root).expect("create the case's directory");
    root.join("state")
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
        subject: "conductor".to_owned(),
        actor: "conductor".to_owned(),
        request_id: id.clone(),
        trace_id: id,
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
        claim: None,
    }
}

/// Another writer of `state/tree/`, as the guard's hook or a dispatch is beside a snapshot.
struct Beside {
    runtime: Runtime,
    log: TreeEventStore,
}

impl Beside {
    fn open(state: &Path) -> Self {
        let runtime = runtime();
        let log = runtime
            .block_on(TreeEventStore::open(state.join("tree")))
            .expect("open the tree beside the snapshot");
        Self { runtime, log }
    }

    /// Commits one record of a stream of its own.
    fn commit(&self) {
        let tenant = TenantId::new("conductor").expect("the tenant");
        let stream = StreamId::new(tenant, BESIDE, new_event_id()).expect("a stream");
        let events = [NewEvent::new("stored", 1, json!({"beside": true})).expect("an event")];
        let meta = meta(&events);
        self.runtime
            .block_on(self.log.append(&stream, Expected::NoStream, &events, &meta))
            .expect("commit beside the snapshot");
    }

    /// Commits `count` records in one command: a tree of `count` records more.
    fn plant(&self, count: usize) {
        let tenant = TenantId::new("conductor").expect("the tenant");
        let appends: Vec<StreamAppend> = (0..count)
            .map(|n| StreamAppend {
                stream: StreamId::new(tenant.clone(), BESIDE, format!("planted-{n:05}"))
                    .expect("a stream"),
                expected: Expected::NoStream,
                events: vec![
                    NewEvent::new("stored", 1, json!({"planted": n, "text": "x".repeat(400)}))
                        .expect("an event"),
                ],
            })
            .collect();
        let group = eventlog_core::AppendGroup {
            tenant: tenant.clone(),
            meta: meta(&appends.iter().map(|a| &a.events).collect::<Vec<_>>()),
            appends,
        };
        self.runtime
            .block_on(self.log.append_group(&group))
            .expect("plant the tree");
    }
}

/// Every byte this process has read so far.
fn read_bytes() -> u64 {
    io_field("rchar")
}

/// Every byte this process has written so far.
fn written_bytes() -> u64 {
    io_field("wchar")
}

fn io_field(name: &str) -> u64 {
    let io = fs::read_to_string("/proc/self/io").expect("read /proc/self/io");
    io.lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(": ")?.parse().ok())
        .unwrap_or_else(|| panic!("no {name} in /proc/self/io: {io}"))
}

/// The bytes of every file under `dir`.
fn bytes_under(dir: &Path) -> u64 {
    fs::read_dir(dir).map_or(0, |entries| {
        entries
            .map(|entry| {
                let entry = entry.expect("a directory entry");
                let kind = entry.file_type().expect("its type");
                if kind.is_dir() {
                    bytes_under(&entry.path())
                } else {
                    entry.metadata().map_or(0, |metadata| metadata.len())
                }
            })
            .sum()
    })
}

fn repository(snapshot: &SnapshotId, name: &str) -> RecordRepository {
    RecordRepository {
        snapshot_id: snapshot.clone(),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Public,
        archived: false,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: 1,
        real_commits_7d: 1,
        behind_main: 0,
        dirty_files: 0,
        worktrees: 1,
        open_issues: 0,
        oldest_open_issue_at: None,
        latest_release: None,
        latest_release_at: None,
        unreleased_commits: None,
        planning_store_version: None,
        in_catalog: Some(true),
        gates_hooks_version: None,
        gates_version: None,
        wave_width: None,
        wave_ready: None,
        wave_reason: None,
    }
}

/// A collector `name` that records `count` repository observations, and lets `beside` commit
/// once after every `every` of them.
fn recording(name: &'static str, count: usize, beside: Rc<Beside>, every: usize) -> Collector {
    Collector::new(name, move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        for n in 0..count {
            record.repository(repository(&snapshot, &format!("{name}-{n:05}")))?;
            if (n + 1) % every == 0 {
                beside.commit();
            }
        }
        Ok(())
    })
}

/// Takes one snapshot over the store under `state`, opened inside the measurement, through
/// `collectors`; answers it with the bytes read and written and the time it took.
fn measured(state: &Path, collectors: &[Collector]) -> (Taken, u64, u64, Duration) {
    let (read, written, started) = (read_bytes(), written_bytes(), Instant::now());
    let taken = snapshot::take_in(state, collectors).expect("the driver ends the snapshot");
    (
        taken,
        read_bytes() - read,
        written_bytes() - written,
        started.elapsed(),
    )
}

/// A store under `state` holding one complete snapshot and a tree of [`PLANTED`] records more,
/// and the other writer, opened on it.
fn fixture(state: &Path) -> Rc<Beside> {
    let taken = snapshot::take_in(state, &[]).expect("the first snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let beside = Beside::open(state);
    beside.plant(PLANTED);
    Rc::new(beside)
}

/// A snapshot that records ten times the observations, with another writer committing to
/// `state/tree/` after each one, reads less more than one read of the whole store: what it reads
/// grows with the observations it records, not with the store once per observation.
#[cfg(target_os = "linux")]
#[test]
fn ten_times_the_observations_read_less_than_the_store_once_more() {
    let few = case("few");
    let many = case("many");
    let few_beside = fixture(&few);
    let many_beside = fixture(&many);
    let store_bytes = bytes_under(&few.join("tree")) + bytes_under(&few.join("observations"));

    let (taken, few_read, few_written, few_took) =
        measured(&few, &[recording("repositories", FEW, few_beside, 1)]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let (taken, many_read, many_written, many_took) = measured(
        &many,
        &[recording("repositories", 10 * FEW, many_beside, 1)],
    );
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");

    println!(
        "store {store_bytes} B; {FEW} observations: read {few_read} B, wrote {few_written} B in \
         {few_took:?}; {} observations: read {many_read} B, wrote {many_written} B in \
         {many_took:?}",
        10 * FEW
    );
    let more = many_read.saturating_sub(few_read);
    assert!(
        more < store_bytes,
        "{} more observations, each after another writer's commit, read {more} B more, against \
         a store of {store_bytes} B: {FEW} read {few_read} B, {} read {many_read} B",
        9 * FEW,
        10 * FEW
    );
}

/// Plants a `Snapshot` left collecting, as a run that was killed leaves one.
fn left_collecting(state: &Path, snapshot: &str) {
    let runtime = runtime();
    runtime.block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the tree");
        let tenant = TenantId::new("conductor").expect("the tenant");
        let stream = StreamId::new(tenant, SNAPSHOT, snapshot).expect("the snapshot's stream");
        let events = [NewEvent::new(
            "stored",
            1,
            json!({
                "state": "Collecting",
                "snapshot_id": snapshot,
                "started_at": "2026-10-16T05:18:00Z",
                "disk_free_bytes": 1,
                "failure_reason": null,
            }),
        )
        .expect("an event")];
        let meta = meta(&events);
        log.append(&stream, Expected::NoStream, &events, &meta)
            .await
            .expect("plant the snapshot");
    });
}

/// The state and failure reason of the snapshot `id` in the store under `state`.
fn state_of(state: &Path, id: &str) -> (SnapshotState, Option<String>) {
    let store = store::open_existing(state).expect("open the store");
    let held = SnapshotStorage::list(&store)
        .into_iter()
        .find(|snapshot| snapshot.data.snapshot_id.0.0 == id)
        .unwrap_or_else(|| panic!("no snapshot {id}"));
    (held.state, held.data.failure_reason)
}

/// A snapshot left `Collecting` by a process that is gone is failed by the next snapshot's start,
/// with the reason `process gone`; the next snapshot completes.
#[test]
fn the_next_start_fails_a_snapshot_left_collecting_by_a_process_that_is_gone() {
    let state = case("gone");
    let first = snapshot::take_in(&state, &[]).expect("a first snapshot");
    assert_eq!(first.state, SnapshotState::Complete, "{first:?}");
    let killed = "01a142ac-372b-70cf-b72d-884a29a963f5";
    left_collecting(&state, killed);
    assert_eq!(state_of(&state, killed).0, SnapshotState::Collecting);

    let next = snapshot::take_in(&state, &[]).expect("the next snapshot");
    assert_eq!(next.state, SnapshotState::Complete, "{next:?}");
    assert_eq!(
        state_of(&state, killed),
        (SnapshotState::Failed, Some("process gone".to_owned())),
        "the snapshot whose process is gone is failed by the next start"
    );
    assert_eq!(
        state_of(&state, &first.snapshot_id.0.0).0,
        SnapshotState::Complete,
        "a complete snapshot is left as it is"
    );
}

/// Each collector's run is recorded with the bytes the snapshot read and wrote while it ran, its
/// records kept included: a collector that records 25 times the observations writes more.
#[cfg(target_os = "linux")]
#[test]
fn each_collector_run_records_the_bytes_read_and_written_while_it_ran() {
    let state = case("bytes");
    let beside = fixture(&state);
    let taken = snapshot::take_in(
        &state,
        &[
            recording("few", 2, Rc::clone(&beside), 1000),
            recording("many", 50, beside, 1000),
        ],
    )
    .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");

    let store = store::open_existing(&state).expect("open the store");
    let bytes = |collector: &str| {
        let run = CollectorRunObservationStorage::list(&store)
            .into_iter()
            .find(|run| {
                run.data.snapshot_id == taken.snapshot_id && run.data.collector == collector
            })
            .unwrap_or_else(|| panic!("no run of {collector} in {}", taken.snapshot_id.0.0));
        (run.data.read_bytes, run.data.written_bytes)
    };
    let (few_read, few_written) = bytes("few");
    let (many_read, many_written) = bytes("many");
    println!(
        "few: read {few_read:?} B, wrote {few_written:?} B; many: read {many_read:?} B, wrote {many_written:?} B"
    );
    for (collector, read, written) in [
        ("few", few_read, few_written),
        ("many", many_read, many_written),
    ] {
        assert!(
            read.is_some_and(|read| read >= 0),
            "{collector}'s run records its read bytes: {read:?}"
        );
        assert!(
            written.is_some_and(|written| written > 0),
            "{collector}'s run records its written bytes: {written:?}"
        );
    }
    assert!(
        many_written > few_written,
        "50 observations write more than 2: {many_written:?} against {few_written:?}"
    );
}

/// A collector that takes a snapshot of the store under `state` beside the one it runs in, and
/// keeps the running snapshot's id with what the one beside ended as.
fn starts_another(state: &Path, seen: &Rc<RefCell<Option<(String, Taken)>>>) -> Collector {
    let (state, seen) = (state.to_owned(), Rc::clone(seen));
    Collector::new("starts-another", move |record| {
        let running = record.snapshot().0.0.clone();
        let taken = snapshot::take_in(&state, &[]).expect("a snapshot beside the running one");
        *seen.borrow_mut() = Some((running, taken));
        Ok(())
    })
}

/// Asserts that the snapshot `outer`, which ran [`starts_another`], completed, and so did the one
/// started beside it, which left it collecting rather than failing it.
fn both_complete(state: &Path, outer: &Taken, seen: &Rc<RefCell<Option<(String, Taken)>>>) {
    assert_eq!(outer.state, SnapshotState::Complete, "{outer:?}");
    let (running, taken) = seen.borrow_mut().take().expect("the collector ran");
    assert_eq!(running, outer.snapshot_id.0.0);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(
        state_of(state, &running),
        (SnapshotState::Complete, None),
        "the running snapshot was not failed by the start beside it"
    );
}

/// A snapshot whose process still runs it stays `Collecting` when another starts beside it, and
/// completes; so does the one started beside it.
#[test]
fn a_start_beside_a_running_snapshot_leaves_it_collecting() {
    let state = case("beside");
    let seen = Rc::default();
    let outer = snapshot::take_in(&state, &[starts_another(&state, &seen)])
        .expect("the running snapshot ends");
    both_complete(&state, &outer, &seen);
}

/// As [`a_start_beside_a_running_snapshot_leaves_it_collecting`], for a snapshot over a store
/// handed to the driver ([`snapshot::take_from`]), which knows no state directory.
#[test]
fn a_start_beside_a_snapshot_over_a_handed_store_leaves_it_collecting() {
    let state = case("handed");
    let seen = Rc::default();
    let outer = snapshot::take_from(
        store::open(&state).expect("open the store"),
        StartSnapshot {
            started_at: Timestamp("2026-10-16T06:00:00Z".to_owned()),
            disk_free_bytes: 1,
            budget: None,
        },
        &[starts_another(&state, &seen)],
    )
    .expect("the running snapshot ends");
    both_complete(&state, &outer, &seen);
}

/// The live counts of one snapshot's observations, by collector, as a snapshot of 2026-10-16
/// recorded them: the cost measurement records as many.
const LIVE: [(&str, usize); 8] = [
    ("repositories", 62),
    ("github", 1247),
    ("gitlab", 1),
    ("blockers", 72),
    ("specifications", 57),
    ("sessions", 12),
    ("secrets", 1),
    ("usage", 1),
];

/// Takes one snapshot over a copy of a live store, as many observations as [`LIVE`] counts, with
/// another writer committing after every 50 (a live tree took 200 to 400 commits an hour), and
/// prints the bytes read and written and the time. Ignored: run it with `SNAPSHOT_COST_STATE`
/// naming the copy's state directory, under a `target/`; the copy is written.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "a measurement over a copy of a live store"]
fn measure_a_snapshot_over_a_live_copy() {
    active::isolate();
    let state = PathBuf::from(
        std::env::var("SNAPSHOT_COST_STATE").expect("SNAPSHOT_COST_STATE names the copy"),
    );
    assert!(
        state.components().any(|part| part.as_os_str() == "target"),
        "the copy lies under a target/: {}",
        state.display()
    );
    assert!(
        state.join("tree/store.json").is_file(),
        "{} holds a tree store",
        state.display()
    );
    let opened = Instant::now();
    let beside = Rc::new(Beside::open(&state));
    println!(
        "the other writer opened the copy in {:?}, peak resident {} kB",
        opened.elapsed(),
        status_kb("VmHWM")
    );
    let collectors: Vec<Collector> = LIVE
        .iter()
        .map(|(name, count)| recording(name, *count, Rc::clone(&beside), 50))
        .collect();
    let (taken, read, written, took) = measured(&state, &collectors);
    println!(
        "snapshot {} ended {:?} in {took:?}: read {read} B ({} MB), wrote {written} B ({} MB), \
         peak resident {} kB",
        taken.snapshot_id.0.0,
        taken.state,
        read / 1_000_000,
        written / 1_000_000,
        status_kb("VmHWM")
    );
}

/// The field `name` of `/proc/self/status`, in kB.
fn status_kb(name: &str) -> u64 {
    let status = fs::read_to_string("/proc/self/status").expect("read /proc/self/status");
    status
        .lines()
        .find_map(|line| {
            line.strip_prefix(name)?
                .strip_prefix(':')?
                .trim()
                .strip_suffix(" kB")?
                .parse()
                .ok()
        })
        .unwrap_or_else(|| panic!("no {name} in /proc/self/status"))
}
