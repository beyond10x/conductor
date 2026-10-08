//! Adversary cases for `story:goal-id-bounds`, wave 04, pass 1: the two commands that take a
//! `conductor.direction.GoalId` and carry no `invalid-goal-id` guard, `AddServing` and
//! `SendDispatch`, run against the real store.
//!
//! Both read the goal row through `GoalStorage::get` with the unchecked `input.goal_id`
//! (`crates/conductor-model/src/behaviour.rs` `add_serving` and `send_dispatch`). No stream can
//! carry some ids, so no goal is stored under one, and `story:store-hardening` (row "Added
//! 2026-10-06") wants such a `get` to answer absent without latching the handle: the command
//! answers `no-such-goal`, and a later write on the same handle is kept. Until wave 05 the store
//! latched the log's refusal of the id and refused every later write.
//!
//! These cases put the goal port on the real store and the serving and dispatch ports in memory,
//! so they reach the store's `get` without `goal add-serving`'s own id check in front of it.
//!
//! No `invalid-goal-id` name appears here, so the file builds against the unit's base as well.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::store::{self, Store, StoreError};
use conductor_model::behaviour::{
    Context, DispatchStorage, Generated, GoalServingStorage, GoalStorage,
};
use conductor_model::direction::obligations::{
    AddServingBehavior, GoalsQuery, ProposeGoalBehavior,
};
use conductor_model::direction::{
    AddServing, AddServingOutcome, GoalData, GoalId, GoalServingSnapshot, GoalSnapshot, GoalState,
    ProposeGoal, ProposeGoalOutcome, ServingId,
};
use conductor_model::dispatch::obligations::SendDispatchBehavior;
use conductor_model::dispatch::{
    DispatchId, DispatchSnapshot, GuardDecisionId, MessageId, ResourceRequestId, SendDispatch,
    SendDispatchOutcome,
};
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use eventlog_core::EventLogError;

/// What these cases assert, and where it is decided.
const WANTED: &str = "story:store-hardening, row \"Added 2026-10-06\": a get on an id no stream \
                      can carry answers absent and does not latch the handle";

/// The goal port on the real store; the serving and dispatch ports in memory.
struct Ports {
    store: Store,
    servings: Vec<GoalServingSnapshot>,
    dispatches: Vec<DispatchSnapshot>,
}

impl GoalStorage for Ports {
    fn get(&self, identity: &GoalId) -> Option<GoalSnapshot> {
        GoalStorage::get(&self.store, identity)
    }
    fn put(&mut self, snapshot: GoalSnapshot) {
        GoalStorage::put(&mut self.store, snapshot);
    }
    fn delete(&mut self, identity: &GoalId) {
        GoalStorage::delete(&mut self.store, identity);
    }
    fn list(&self) -> Vec<GoalSnapshot> {
        GoalStorage::list(&self.store)
    }
}

impl GoalServingStorage for Ports {
    fn get(&self, identity: &ServingId) -> Option<GoalServingSnapshot> {
        self.servings
            .iter()
            .find(|held| held.data.serving_id == *identity)
            .cloned()
    }
    fn put(&mut self, snapshot: GoalServingSnapshot) {
        self.servings
            .retain(|held| held.data.serving_id != snapshot.data.serving_id);
        self.servings.push(snapshot);
    }
    fn delete(&mut self, identity: &ServingId) {
        self.servings
            .retain(|held| held.data.serving_id != *identity);
    }
    fn list(&self) -> Vec<GoalServingSnapshot> {
        self.servings.clone()
    }
}

impl DispatchStorage for Ports {
    fn get(&self, identity: &DispatchId) -> Option<DispatchSnapshot> {
        self.dispatches
            .iter()
            .find(|held| held.data.dispatch_id == *identity)
            .cloned()
    }
    fn put(&mut self, snapshot: DispatchSnapshot) {
        self.dispatches
            .retain(|held| held.data.dispatch_id != snapshot.data.dispatch_id);
        self.dispatches.push(snapshot);
    }
    fn delete(&mut self, identity: &DispatchId) {
        self.dispatches
            .retain(|held| held.data.dispatch_id != *identity);
    }
    fn list(&self) -> Vec<DispatchSnapshot> {
        self.dispatches.clone()
    }
}

impl Context for Ports {
    fn generate_conductor_direction_serving_id(&mut self) -> ServingId {
        ServingId(Uuid("00000000-0000-4000-8000-000000000001".to_owned()))
    }
    fn generate_conductor_dispatch_guard_decision_id(&mut self) -> GuardDecisionId {
        unreachable!("AddServing and SendDispatch assign no guard decision id")
    }
    fn generate_conductor_dispatch_message_id(&mut self) -> MessageId {
        unreachable!("AddServing and SendDispatch assign no message id")
    }
    fn generate_conductor_dispatch_resource_request_id(&mut self) -> ResourceRequestId {
        unreachable!("AddServing and SendDispatch assign no resource request id")
    }
    fn generate_conductor_observation_board_id(&mut self) -> BoardId {
        unreachable!("AddServing and SendDispatch assign no board id")
    }
    fn generate_conductor_observation_observation_id(&mut self) -> ObservationId {
        unreachable!("AddServing and SendDispatch assign no observation id")
    }
    fn generate_conductor_observation_snapshot_id(&mut self) -> SnapshotId {
        unreachable!("AddServing and SendDispatch assign no snapshot id")
    }
}

fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_goal_id_unguarded")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn open(state: &Path) -> Generated<Ports> {
    Generated::new(Ports {
        store: store::open(state).expect("open the store"),
        servings: Vec::new(),
        dispatches: Vec::new(),
    })
}

/// The ids no stream can carry, each with eventlog's refusal of it: empty, and 171 `%` (513 bytes
/// once escaped).
fn unkeepable() -> Vec<(&'static str, String, &'static str)> {
    vec![
        ("the empty string", String::new(), "stream id is required"),
        (
            "171 percent signs, 513 bytes escaped",
            "%".repeat(171),
            "stream id is too long",
        ),
    ]
}

/// The store cannot keep a goal under `goal`: a write of one, on a handle of its own, is refused
/// with eventlog's `reason`. Without this the cases below would pass for an id the store can hold.
fn assert_unkeepable(state: &Path, what: &str, goal: &str, reason: &str) {
    let mut store = store::open(state).expect("open the store");
    GoalStorage::put(
        &mut store,
        GoalSnapshot {
            state: GoalState::Draft,
            data: GoalData {
                goal_id: GoalId(goal.to_owned()),
                title: "unkeepable".to_owned(),
                exit_evidence: "evidence".to_owned(),
            },
        },
    );
    let check = store.check();
    assert!(
        matches!(
            &check,
            Err(StoreError::Log {
                entity: "conductor.direction.Goal",
                identity: Some(identity),
                source: EventLogError::Invalid(detail),
            }) if identity.as_str() == goal && detail.as_str() == reason
        ),
        "{what}: the store refuses to keep a goal under this id with {reason:?}, got {check:?}"
    );
}

/// What the command under attack leaves behind: a handle that read the goal id as absent and
/// latched nothing, so a proposal after it is kept and a reopened store holds it.
fn assert_the_handle_is_kept(mut generated: Generated<Ports>, state: &Path, what: &str) {
    let check = generated.ports.store.check();
    assert!(
        check.is_ok(),
        "{what}: the read of a goal id no stream can carry latched {check:?}; {WANTED}"
    );
    let after = generated
        .propose_goal(ProposeGoal {
            goal_id: GoalId("G-after".to_owned()),
            title: "after".to_owned(),
            exit_evidence: "evidence".to_owned(),
        })
        .expect("ProposeGoal decides an outcome");
    assert!(
        matches!(after, ProposeGoalOutcome::Proposed { .. }),
        "{what}: a proposal after it: {after:?}"
    );
    let check = generated.ports.store.check();
    assert!(check.is_ok(), "{what}: the proposal is kept: {check:?}");
    drop(generated);
    let reopened = open(state);
    let held: Vec<String> = reopened
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| row.goal_id.0)
        .collect();
    assert_eq!(
        held,
        ["G-after"],
        "{what}: the later write on the same handle is kept; {WANTED}"
    );
}

/// `AddServing` names a goal by an id no stream can carry. It answers `no-such-goal`, the store
/// handle it ran on latches nothing, and a later write on that handle is kept.
#[test]
fn adv_add_serving_with_an_unkeepable_goal_id_answers_no_such_goal_and_keeps_the_handle() {
    for (n, (what, goal, reason)) in unkeepable().into_iter().enumerate() {
        let state = state_dir(&format!("add-serving-{n}"));
        assert_unkeepable(&state, what, &goal, reason);
        let mut generated = open(&state);
        let outcome = generated
            .add_serving(AddServing {
                goal_id: GoalId(goal.clone()),
                repository: RepositoryName("conductor".to_owned()),
            })
            .expect("AddServing decides an outcome");
        assert!(
            matches!(outcome, AddServingOutcome::NoSuchGoal { .. }),
            "{what}: no goal has this id: {outcome:?}"
        );
        assert_the_handle_is_kept(generated, &state, what);
    }
}

/// `SendDispatch` names a goal by an id no stream can carry. It answers `no-such-goal`, the store
/// handle it ran on latches nothing, and a later write on that handle is kept.
#[test]
fn adv_send_dispatch_with_an_unkeepable_goal_id_answers_no_such_goal_and_keeps_the_handle() {
    for (n, (what, goal, reason)) in unkeepable().into_iter().enumerate() {
        let state = state_dir(&format!("send-dispatch-{n}"));
        assert_unkeepable(&state, what, &goal, reason);
        let mut generated = open(&state);
        let outcome = generated
            .send_dispatch(SendDispatch {
                dispatch_id: DispatchId("D1".to_owned()),
                repository: RepositoryName("conductor".to_owned()),
                goal_id: GoalId(goal.clone()),
                brief: "brief".to_owned(),
                sent_at: Timestamp("2026-10-06T09:00:00Z".to_owned()),
                priority: 0,
                waits_on: None,
            })
            .expect("SendDispatch decides an outcome");
        assert!(
            matches!(outcome, SendDispatchOutcome::NoSuchGoal { .. }),
            "{what}: no goal has this id: {outcome:?}"
        );
        assert_the_handle_is_kept(generated, &state, what);
    }
}
