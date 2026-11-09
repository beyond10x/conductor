//! Adversary cases for `story:status-current-snapshot`, wave 49: the index of the instance's
//! streams (`store/index.rs`) against the whole-feed fold it replaced. A list of the instance's
//! tenant must answer what the feed holds, whatever the index was brought up to date with.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`.

mod active;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::store::{self, Store, StoreError};
use conductor_model::behaviour::GoalStorage;
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use eventlog_core::{EventStore as _, MAX_READ_LIMIT, StreamId, TenantId};
use eventlog_sqlite::SqliteEventStore;

/// One `Goal` as a list answers it: `(goal_id, state, title)`.
type Row = (String, String, String);

const GOAL: &str = "conductor.direction.Goal";

fn state_dir(case: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_status_index")
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

/// Puts `snapshot` through a fresh handle, reading the goal first so the write expects where
/// the stream is.
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

/// Brings the index of the instance's streams up to date, as the handle that completes a
/// snapshot does (`Store::retain` runs it first; with `keep` 0 it drops nothing).
fn index(state: &Path) {
    open(state).retain(0).expect("retention runs");
}

fn listed(store: &Store) -> Vec<Row> {
    GoalStorage::list(store)
        .into_iter()
        .map(|goal| (goal.data.goal_id.0, format!("{:?}", goal.state), goal.data.title))
        .collect()
}

/// What the whole feed holds of the goals, folded as the store did before the index: each stream
/// in the order first stored, its newest record, a deleted one left out.
fn oracle(state: &Path) -> Vec<Row> {
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(state), store::PREFIX)
            .await
            .expect("open the raw log");
        let tenant = TenantId::new("conductor").expect("tenant");
        let mut order: Vec<String> = Vec::new();
        let mut newest: HashMap<String, (String, serde_json::Value)> = HashMap::new();
        let mut after = 0;
        loop {
            let page = log
                .read_feed(&tenant, after, MAX_READ_LIMIT)
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
                newest.insert(event.stream_id, (event.name, event.data));
            }
            if !more {
                break;
            }
        }
        order
            .into_iter()
            .filter_map(|stream| {
                let (name, data) = newest.remove(&stream)?;
                (name == "stored").then(|| {
                    (
                        stream,
                        data["state"].as_str().unwrap_or_default().to_owned(),
                        data["title"].as_str().unwrap_or_default().to_owned(),
                    )
                })
            })
            .collect()
    })
}

/// A record redacted after the index was brought up to date is a record the store cannot read,
/// as it was before the index (`adv_one_record_the_store_cannot_read_hides_only_itself` in
/// `tests/adv_store.rs`, `story:store-hardening`): the Goals list leaves G2 out and the handle
/// reports it. The index keeps a copy of G2's body that the redaction does not reach, so a list
/// that reads the index answers the erased title as if nothing had happened.
#[test]
fn adv_a_record_redacted_after_the_index_caught_up_is_not_answered_from_the_index() {
    let state = state_dir("redacted-after-index");
    put(&state, goal("G1", GoalState::Draft, "Readable"));
    put(&state, goal("G2", GoalState::Draft, "Erased on request"));
    index(&state);
    runtime().block_on(async {
        let log = SqliteEventStore::open(&raw_file(&state), store::PREFIX)
            .await
            .expect("open the raw log");
        let tenant = TenantId::new("conductor").expect("tenant");
        log.redact(
            &StreamId::new(tenant, GOAL, "G2").expect("stream"),
            1,
            "erased on request",
        )
        .await
        .expect("redact the record");
    });

    let reader = open(&state);
    let rows = listed(&reader);
    let check = reader.check();
    assert_eq!(
        rows,
        vec![("G1".to_owned(), "Draft".to_owned(), "Readable".to_owned())],
        "the redacted G2 is not answered, its erased title least of all ({check:?})"
    );
    assert!(
        matches!(
            &check,
            Err(StoreError::Undecodable { entity, stream, .. })
                if *entity == GOAL && stream == "G2"
        ),
        "the redacted G2 is reported as a record this build cannot read: {check:?}"
    );
}

/// The Goals list equals the whole-feed fold after every step of a fixed-seed run of puts,
/// deletes, re-creations after a delete and index catch-ups, read by a fresh handle and by a
/// handle inside `Store::at_once` that the run keeps open across catch-ups.
#[test]
fn adv_the_indexed_list_equals_the_whole_feed_fold_through_puts_deletes_and_catch_ups() {
    let state = state_dir("indexed-equals-feed");
    let states = [
        GoalState::Draft,
        GoalState::Confirmed,
        GoalState::Met,
        GoalState::Dropped,
    ];
    let mut seed: u64 = 0x5eed_0049_0007;
    let mut next = |bound: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % bound
    };
    let mut present: HashMap<String, bool> = HashMap::new();
    let kept = open(&state);
    kept.at_once(|kept| {
        for step in 0..120 {
            let id = format!("G{}", next(8));
            match next(10) {
                0..=4 => {
                    let held = states[usize::try_from(next(4)).expect("small")];
                    put(&state, goal(&id, held, &format!("step {step}")));
                    present.insert(id, true);
                }
                5 | 6 if present.get(&id).copied().unwrap_or(false) => {
                    delete(&state, &id);
                    present.insert(id, false);
                }
                _ => index(&state),
            }
            let expected = oracle(&state);
            assert_eq!(
                listed(&open(&state)),
                expected,
                "step {step}: a fresh handle's list against the feed"
            );
            assert_eq!(
                listed(kept),
                expected,
                "step {step}: a list inside at_once against the feed"
            );
        }
    });
    kept.check().expect("every list read");
}
