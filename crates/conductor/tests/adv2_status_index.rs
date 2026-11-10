//! Adversary cases for `story:status-current-snapshot`, wave 49, second pass: the correction
//! that keeps only `{first, version}` in each row of the index of the instance's streams
//! (`store/index.rs`) and reads each stream's event at that version from the log. The rows a
//! build before the correction left, rows ahead of the index's position (what a list meets when
//! a catch-up commits while it pages), a version the log no longer holds, and redactions
//! interleaved with catch-ups, each against the whole-feed fold the index replaced.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`.

mod active;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use conductor_cli::store::{self, Store, StoreError};
use conductor_model::behaviour::GoalStorage;
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use eventlog_core::{
    BoxFuture, EventLogError, EventStore as _, MAX_READ_LIMIT, ProjectionSpec, ProjectionStore,
    Projector, RecordedEvent, StreamId, TenantId,
};
use eventlog_sqlite::SqliteEventStore;
use serde_json::{Value, json};

/// One `Goal` as a list answers it: `(goal_id, state, title)`.
type Row = (String, String, String);

const GOAL: &str = "conductor.direction.Goal";

/// The index's table, as `store/index.rs` declares it.
const SPEC: ProjectionSpec = ProjectionSpec {
    name: "conductor_streams",
    indexed: &[],
};

const POSITION: &str = "~position";

fn state_dir(case: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv2_status_index")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn open(state: &Path) -> Store {
    store::open(state).expect("open the store")
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime")
}

fn raw_file(state: &Path) -> String {
    state
        .join(store::STORE_FILE)
        .to_str()
        .expect("a UTF-8 path")
        .to_owned()
}

fn tenant() -> TenantId {
    TenantId::new("conductor").expect("tenant")
}

fn goal(id: &str, state: GoalState, title: &str) -> GoalSnapshot {
    GoalSnapshot {
        state,
        data: GoalData {
            goal_id: GoalId(id.to_owned()),
            actor: conductor_cli::actor::operator("operator"),
            title: title.to_owned(),
            exit_evidence: format!("evidence for {id}"),
            evidence: None,
            reason: None,
        },
    }
}

fn put(state: &Path, snapshot: GoalSnapshot) {
    let mut store = open(state);
    let _ = GoalStorage::get(&store, &snapshot.data.goal_id);
    GoalStorage::put(&mut store, snapshot);
    store.check().expect("the put is kept");
}

fn delete(state: &Path, id: &str) {
    let mut store = open(state);
    let id = GoalId(id.to_owned());
    let _ = GoalStorage::get(&store, &id);
    GoalStorage::delete(&mut store, &id);
    store.check().expect("the delete is kept");
}

/// Brings the index up to date as this build does (`Store::retain` runs it first).
fn index(state: &Path) {
    open(state).retain(0).expect("retention runs");
}

fn listed(store: &Store) -> Vec<Row> {
    GoalStorage::list(store)
        .into_iter()
        .map(|goal| {
            (
                goal.data.goal_id.0,
                format!("{:?}", goal.state),
                goal.data.title,
            )
        })
        .collect()
}

fn redact(state: &Path, id: &str, version: u64) {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        log.redact(
            &StreamId::new(tenant(), GOAL, id).expect("stream"),
            version,
            "erased on request",
        )
        .await
        .expect("redact the record");
    });
}

/// The head version of the goal `id`'s stream.
fn head(state: &Path, id: &str) -> u64 {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        log.stream_version(&StreamId::new(tenant(), GOAL, id).expect("stream"))
            .await
            .expect("read the head")
            .unwrap_or(0)
    })
}

/// What the whole feed holds of the goals, folded as the store did before the index: each
/// stream in the order first stored, its newest record; a deleted one left out, and one whose
/// newest record is redacted left out and named in the second half.
fn oracle(state: &Path) -> (Vec<Row>, Vec<String>) {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        let mut order: Vec<String> = Vec::new();
        let mut newest: HashMap<String, RecordedEvent> = HashMap::new();
        let mut after = 0;
        loop {
            let page = log
                .read_feed(&tenant(), after, MAX_READ_LIMIT)
                .await
                .expect("read the feed");
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            for event in page.events {
                if event.stream_type != GOAL {
                    continue;
                }
                if !newest.contains_key(&event.stream_id) {
                    order.push(event.stream_id.clone());
                }
                newest.insert(event.stream_id.clone(), event);
            }
            if !more {
                break;
            }
        }
        let mut rows = Vec::new();
        let mut unreadable = Vec::new();
        for stream in order {
            let event = newest.remove(&stream).expect("held");
            if event.name != "stored" {
                continue;
            }
            if event.redacted_at.is_some() {
                unreadable.push(stream);
                continue;
            }
            rows.push((
                stream,
                event.data["state"].as_str().unwrap_or_default().to_owned(),
                event.data["title"].as_str().unwrap_or_default().to_owned(),
            ));
        }
        (rows, unreadable)
    })
}

/// The index's projector as the build before the correction (2f5d3607f) wrote it: each row
/// with a copy of the stream's newest event, body included.
struct EarlierBuild;

/// The index's projector writing this build's rows and leaving the position row where it is:
/// the state a list meets when a catch-up commits between its read of the position and its
/// read of the rows.
struct RowsAhead;

fn apply_row<'a>(
    event: &'a RecordedEvent,
    store: &'a mut dyn ProjectionStore,
    earlier: bool,
    position: bool,
) -> BoxFuture<'a, Result<(), EventLogError>> {
    Box::pin(async move {
        let key = format!("{}/{}", event.stream_type, event.stream_id);
        let held = store.get(&SPEC, &event.tenant, &key).await?;
        if held
            .as_ref()
            .and_then(|row| row["version"].as_u64())
            .is_some_and(|version| version >= event.version)
        {
            return Ok(());
        }
        let first = held.map_or_else(
            || {
                json!({
                    "at": event.recorded_at.unix_timestamp_nanos().to_string(),
                    "seq": event.global_seq,
                })
            },
            |row| row["first"].clone(),
        );
        let row = if earlier {
            json!({
                "first": first,
                "version": event.version,
                "name": event.name,
                "schema_version": event.schema_version,
                "redacted": event.redacted_at.is_some(),
                "data": event.data,
            })
        } else {
            json!({"first": first, "version": event.version})
        };
        store.upsert(&SPEC, &event.tenant, &key, &row).await?;
        if position {
            store
                .upsert(
                    &SPEC,
                    &event.tenant,
                    POSITION,
                    &json!({"position": event.global_seq}),
                )
                .await?;
        }
        Ok(())
    })
}

impl Projector for EarlierBuild {
    fn name(&self) -> &'static str {
        SPEC.name
    }

    fn projections(&self) -> &'static [ProjectionSpec] {
        &[SPEC]
    }

    fn apply<'a>(
        &'a self,
        event: &'a RecordedEvent,
        store: &'a mut dyn ProjectionStore,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        apply_row(event, store, true, true)
    }
}

impl Projector for RowsAhead {
    fn name(&self) -> &'static str {
        SPEC.name
    }

    fn projections(&self) -> &'static [ProjectionSpec] {
        &[SPEC]
    }

    fn apply<'a>(
        &'a self,
        event: &'a RecordedEvent,
        store: &'a mut dyn ProjectionStore,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        apply_row(event, store, false, false)
    }
}

/// Brings the index up to date through `projector` instead of this build's.
fn catch_up_with(state: &Path, projector: Arc<dyn Projector>) {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        log.create_projections(Arc::clone(&projector))
            .await
            .expect("create the index's table");
        loop {
            let progress = log
                .run_catch_up(Arc::clone(&projector), &tenant(), MAX_READ_LIMIT)
                .await
                .expect("catch up");
            if !progress.more_waiting || progress.applied == 0 {
                break;
            }
        }
    });
}

/// Every row of the index's table, by key.
fn index_rows(state: &Path) -> Vec<(String, Value)> {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        let mut rows = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = log
                .projection_page(&SPEC, &tenant(), None, cursor.as_deref(), MAX_READ_LIMIT)
                .await
                .expect("page the index");
            rows.extend(page.rows);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        rows
    })
}

fn readable() -> Vec<Row> {
    vec![("G1".to_owned(), "Draft".to_owned(), "Readable".to_owned())]
}

/// Rows the build before the correction left are read by their version alone: the copy of the
/// body they hold is not answered once the record is redacted, and the next catch-up of this
/// build replaces a rewritten stream's row by `{first, version}`.
#[test]
fn adv2_rows_of_the_earlier_build_are_read_at_their_version_and_replaced_on_the_next_write() {
    let state = state_dir("earlier-rows-read");
    put(&state, goal("G1", GoalState::Draft, "Readable"));
    put(&state, goal("G2", GoalState::Draft, "Erased on request"));
    catch_up_with(&state, Arc::new(EarlierBuild));
    assert!(
        index_rows(&state)
            .iter()
            .any(|(key, row)| key.ends_with("/G2") && row.get("data").is_some()),
        "the earlier build's row of G2 holds its body"
    );
    redact(&state, "G2", 1);

    let reader = open(&state);
    let rows = listed(&reader);
    let check = reader.check();
    assert_eq!(
        rows,
        readable(),
        "G2's copied body is not answered ({check:?})"
    );
    assert!(
        matches!(
            &check,
            Err(StoreError::Undecodable { entity, stream, .. })
                if *entity == GOAL && stream == "G2"
        ),
        "G2 is reported unreadable: {check:?}"
    );

    put(&state, goal("G1", GoalState::Confirmed, "Readable again"));
    index(&state);
    let g1 = index_rows(&state)
        .into_iter()
        .find(|(key, _)| key.ends_with("/G1"))
        .expect("G1's row");
    assert_eq!(
        g1.1.as_object().map(|row| {
            let mut keys: Vec<&str> = row.keys().map(String::as_str).collect();
            keys.sort_unstable();
            keys
        }),
        Some(vec!["first", "version"]),
        "G1's rewritten row holds only first and version: {g1:?}"
    );
    assert_eq!(
        listed(&open(&state)),
        vec![(
            "G1".to_owned(),
            "Confirmed".to_owned(),
            "Readable again".to_owned()
        )]
    );
}

/// A catch-up that commits while a list reads the index leaves the rows ahead of the position
/// the list read first: the list then folds, over rows of later versions, the feed after an
/// earlier position. It answers what the feed holds.
#[test]
fn adv2_a_list_over_rows_ahead_of_the_index_position_answers_the_feed() {
    let state = state_dir("rows-ahead");
    put(&state, goal("G1", GoalState::Draft, "one"));
    put(&state, goal("G2", GoalState::Draft, "two"));
    put(&state, goal("G3", GoalState::Draft, "three"));
    index(&state);
    put(&state, goal("G1", GoalState::Confirmed, "one, confirmed"));
    delete(&state, "G2");
    put(&state, goal("G4", GoalState::Draft, "four"));
    put(&state, goal("G0", GoalState::Draft, "zero, stored last"));
    catch_up_with(&state, Arc::new(RowsAhead));
    let rows = index_rows(&state);
    let position = rows
        .iter()
        .find(|(key, _)| key == POSITION)
        .and_then(|(_, row)| row["position"].as_u64())
        .expect("the position row");
    let g0 = rows
        .iter()
        .find(|(key, _)| key.ends_with("/G0"))
        .and_then(|(_, row)| row["first"]["seq"].as_u64())
        .expect("G0's row");
    assert!(
        g0 > position,
        "the rows are ahead of the position ({g0} > {position})"
    );
    put(&state, goal("G3", GoalState::Met, "three, met"));
    put(&state, goal("G2", GoalState::Draft, "two, again"));

    let (expected, unreadable) = oracle(&state);
    assert!(unreadable.is_empty());
    let reader = open(&state);
    assert_eq!(listed(&reader), expected);
    reader.check().expect("every list read");
    let kept = open(&state);
    kept.at_once(|kept| {
        assert_eq!(listed(kept), expected, "inside at_once");
        assert_eq!(listed(kept), expected, "inside at_once, twice");
    });
    kept.check().expect("every list read");
}

/// A version the index names that the log does not hold (the event rows of a stream removed
/// beneath the index) leaves that stream out and is reported; the other streams are answered.
#[test]
fn adv2_a_version_the_log_no_longer_holds_hides_only_its_stream() {
    let state = state_dir("version-gone");
    put(&state, goal("G1", GoalState::Draft, "Readable"));
    put(&state, goal("G2", GoalState::Draft, "first"));
    put(&state, goal("G2", GoalState::Confirmed, "second"));
    index(&state);
    let connection = rusqlite::Connection::open(raw_file(&state)).expect("open the database");
    let removed = connection
        .execute(
            &format!(
                "DELETE FROM {}_events WHERE stream_type = ?1 AND stream_id = 'G2' AND version = 2",
                store::PREFIX
            ),
            [GOAL],
        )
        .expect("remove G2's newest event");
    assert_eq!(removed, 1);
    drop(connection);

    let reader = open(&state);
    assert_eq!(listed(&reader), readable());
    let check = reader.check();
    assert!(
        matches!(&check, Err(failure) if format!("{failure:?}").contains("G2")),
        "G2 is reported: {check:?}"
    );
}

/// The Goals list equals the whole-feed fold, redactions included, after every step of a
/// fixed-seed run of puts, deletes, redactions of a stream's newest record or an older one, and
/// index catch-ups; and a fresh handle reports exactly the streams whose newest record is
/// redacted.
#[test]
fn adv2_the_indexed_list_equals_the_whole_feed_fold_through_redactions_and_catch_ups() {
    let state = state_dir("redactions-equal-feed");
    let states = [
        GoalState::Draft,
        GoalState::Confirmed,
        GoalState::Met,
        GoalState::Dropped,
    ];
    let mut seed: u64 = 0x5eed_0049_0702;
    let mut next = |bound: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % bound
    };
    let mut present: HashMap<String, bool> = HashMap::new();
    let mut erased: HashSet<String> = HashSet::new();
    for step in 0..160 {
        let id = format!("G{}", next(10));
        let choice = next(12);
        if erased.contains(&id) {
            index(&state);
        } else {
            match choice {
                0..=4 => {
                    let held = states[usize::try_from(next(4)).expect("small")];
                    put(&state, goal(&id, held, &format!("step {step}")));
                    present.insert(id, true);
                }
                5 if present.get(&id).copied().unwrap_or(false) => {
                    delete(&state, &id);
                    present.insert(id, false);
                }
                6 | 7 if present.get(&id).copied().unwrap_or(false) => {
                    let newest = head(&state, &id);
                    let version = if choice == 6 || newest == 1 {
                        erased.insert(id.clone());
                        newest
                    } else {
                        1 + next(newest - 1)
                    };
                    redact(&state, &id, version);
                }
                _ => index(&state),
            }
        }
        let (expected, unreadable) = oracle(&state);
        let reader = open(&state);
        assert_eq!(
            listed(&reader),
            expected,
            "step {step}: the list against the feed"
        );
        let check = reader.check();
        match unreadable.as_slice() {
            [] => assert!(check.is_ok(), "step {step}: {check:?}"),
            _ => assert!(
                matches!(
                    &check,
                    Err(StoreError::Undecodable { entity, stream, .. })
                        if *entity == GOAL && unreadable.contains(stream)
                ),
                "step {step}: one of {unreadable:?} is reported: {check:?}"
            ),
        }
    }
}
