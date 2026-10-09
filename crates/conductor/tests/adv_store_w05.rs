//! Adversary cases for `story:store-hardening`, wave 05, pass 1: one unreadable record seen through
//! the binary's views, an unreadable deletion, which record of an instance the view judges, a write
//! after an unreadable read, and two handles and a handle crossing in and out of a caller's tokio
//! runtime.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`. Every case uses only interfaces the unit's base already had, so the file builds
//! against the base as well.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use conductor_cli::store::{self, Store, StoreError};
use conductor_model::behaviour::{
    BlockerObservationStorage, ControllerStorage, Generated, GoalServingStorage, GoalStorage,
    GuardDecisionStorage, PullRequestObservationStorage, RepositoryObservationStorage,
    ResourceRequestStorage, SessionObservationStorage, SnapshotStorage,
    SpecificationObservationStorage, WorkflowRunObservationStorage,
};
use conductor_model::config::SessionName;
use conductor_model::direction::obligations::{
    ConfirmGoalBehavior, GoalsQuery, ProposeGoalBehavior,
};
use conductor_model::direction::{
    ConfirmGoal, ConfirmGoalOutcome, GoalData, GoalId, GoalServingData, GoalServingSnapshot,
    GoalServingState, GoalSnapshot, GoalState, ProposeGoal, ProposeGoalOutcome, ServingId,
};
use conductor_model::dispatch::{
    ControllerData, ControllerId, ControllerSnapshot, ControllerState, GuardDecisionData,
    GuardDecisionId, GuardDecisionSnapshot, GuardDecisionState, ResourceKind, ResourceRequestData,
    ResourceRequestId, ResourceRequestSnapshot, ResourceRequestState, Verdict,
};
use conductor_model::observation::{Harness, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, NewEvent, StreamId, TenantId, new_event_id,
    request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

const GOAL: &str = "conductor.direction.Goal";
const GUARD_DECISION: &str = "conductor.dispatch.GuardDecision";

/// One `Goals` row, as `(goal_id, state, title)`.
type Row = (String, GoalState, String);

/// An empty state directory for one case.
fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_store_w05")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn id(text: &str) -> GoalId {
    GoalId(text.to_owned())
}

fn open(state: &Path) -> Generated<Store> {
    Generated::new(store::open(state).expect("open the store"))
}

fn propose(generated: &mut Generated<Store>, goal: &str, title: &str) -> ProposeGoalOutcome {
    generated
        .propose_goal(ProposeGoal {
            goal_id: id(goal),
            title: title.to_owned(),
            exit_evidence: format!("evidence for {goal}"),
        })
        .expect("ProposeGoal decides an outcome")
}

/// Proposes `goal` on a fresh handle and checks the proposal was kept.
fn propose_kept(state: &Path, goal: &str, title: &str) {
    let mut generated = open(state);
    propose(&mut generated, goal, title);
    generated.ports.check().expect("the proposal is kept");
}

fn rows(generated: &Generated<Store>) -> Vec<Row> {
    generated
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| (row.goal_id.0, row.state, row.title))
        .collect()
}

fn row(goal: &str, state: GoalState, title: &str) -> Row {
    (goal.to_owned(), state, title.to_owned())
}

fn callers_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("the caller's runtime")
}

/// Appends one event to `entity`'s instance `identity` with the raw log, as another build would.
fn append_raw(
    state: &Path,
    entity: &str,
    identity: &str,
    name: &str,
    version: u32,
    expected: Expected,
    data: Value,
) {
    callers_runtime().block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the raw log");
        let tenant = TenantId::new("conductor").expect("tenant");
        let stream = StreamId::new(tenant, entity, identity).expect("stream");
        let events = [NewEvent::new(name, version, data).expect("event")];
        let key = new_event_id();
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: request_hash(&events).expect("request hash"),
            subject: "conductor".to_owned(),
            actor: "conductor".to_owned(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::now_utc(),
            claim: None,
        };
        log.append(&stream, expected, &events, &meta)
            .await
            .expect("append the other build's record");
    });
}

/// A Goal body as a later schema version would write it.
fn later_goal(goal: &str, state: &str) -> Value {
    json!({
        "state": state,
        "goal_id": goal,
        "title": "Written by a later build",
        "exit_evidence": "e",
        "owner": "a field version 2 added",
    })
}

/// Runs the `conductor` binary on `state` with `args`.
fn conductor(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_conductor"))
        .arg("--state-dir")
        .arg(state)
        .args(args)
        .output()
        .expect("run conductor")
}

/// The story's wanted column, through the command a person runs: `conductor goal goals` over a
/// store holding G1 and G3 beside a G2 written by a later schema lists G1 and G3, says G2 was
/// unreadable, and does not exit 0.
///
/// `story:store-hardening`: "only that goal is hidden, and the view says one record was
/// unreadable". `goal::goals` (`src/goal.rs`) returns `check()`'s error before it writes a row, so
/// the command prints no goal at all.
#[test]
fn adv_w05_the_goals_command_lists_the_goals_beside_an_unreadable_one() {
    let state = state_dir("goals-command");
    propose_kept(&state, "G1", "Readable");
    append_raw(
        &state,
        GOAL,
        "G2",
        "stored",
        2,
        Expected::NoStream,
        later_goal("G2", "Draft"),
    );
    propose_kept(&state, "G3", "Readable after");

    let output = conductor(&state, &["goal", "goals", "--format", "jsonl"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("\"G1\"") && stdout.contains("\"G3\""),
        "the Goals view lists G1 and G3 beside the unreadable G2; status {:?}, stdout {stdout:?}, \
         stderr {stderr:?}",
        output.status.code()
    );
    assert!(
        !stdout.contains("\"G2\""),
        "the unreadable G2 is hidden: {stdout:?}"
    );
    assert!(
        stderr.contains("G2"),
        "the view says G2 was unreadable: {stderr:?}"
    );
    assert!(
        !output.status.success(),
        "a view that skipped a record does not exit 0"
    );
}

/// The same, through the view of the one record kind the shipping binary writes today:
/// `conductor guard guard-decisions` over a store holding one readable decision beside one written
/// by a later schema lists the readable one and names the other.
#[test]
fn adv_w05_the_guard_decisions_command_lists_the_decisions_beside_an_unreadable_one() {
    let state = state_dir("guard-decisions-command");
    let readable = "11111111-1111-4111-8111-111111111111";
    let later = "22222222-2222-4222-8222-222222222222";
    {
        let mut handle = store::open(&state).expect("open the store");
        GuardDecisionStorage::put(
            &mut handle,
            GuardDecisionSnapshot {
                state: GuardDecisionState::Recorded,
                data: GuardDecisionData {
                    guard_decision_id: GuardDecisionId(Uuid(readable.to_owned())),
                    repository: RepositoryName("delta".to_owned()),
                    session_ref: "s1".to_owned(),
                    tool: "Bash".to_owned(),
                    target: "git push".to_owned(),
                    verdict: Verdict::Deny,
                    reason: "a readable verdict".to_owned(),
                    decided_at: Timestamp("2026-10-07T00:00:00Z".to_owned()),
                },
            },
        );
        handle.check().expect("the readable verdict is kept");
    }
    append_raw(
        &state,
        GUARD_DECISION,
        later,
        "stored",
        2,
        Expected::NoStream,
        json!({
            "state": "Recorded",
            "guard_decision_id": later,
            "repository": "delta",
            "session_ref": "s2",
            "tool": "Bash",
            "target": "git push",
            "verdict": "Deny",
            "reason": "written by a later build",
            "decided_at": "2026-10-07T00:00:01Z",
            "rule": "a field version 2 added",
        }),
    );

    let output = conductor(&state, &["guard", "guard-decisions", "--format", "jsonl"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains(readable),
        "the view lists the readable decision beside the unreadable one; status {:?}, stdout \
         {stdout:?}, stderr {stderr:?}",
        output.status.code()
    );
    assert!(
        stderr.contains(later),
        "the view names the unreadable decision: {stderr:?}"
    );
}

/// A `deleted` record written by a later schema version is a record this build cannot read, and
/// the unit's own claim and doc (`store.rs` `rows`) say such a newest record is reported through
/// `check()`. `body()` takes every `deleted` as a deletion at any version, so G2 disappears and
/// `check()` stays `Ok`.
#[test]
fn adv_w05_a_deletion_written_by_a_later_schema_is_reported() {
    let state = state_dir("later-deletion");
    propose_kept(&state, "G1", "Readable");
    propose_kept(&state, "G2", "Deleted by a later build");
    append_raw(
        &state,
        GOAL,
        "G2",
        "deleted",
        2,
        Expected::Exact(1),
        json!({"reason": "a field version 2 added"}),
    );

    let reopened = open(&state);
    let listed = rows(&reopened);
    let check = reopened.ports.check();
    assert_eq!(listed, vec![row("G1", GoalState::Draft, "Readable")]);
    assert!(
        matches!(
            &check,
            Err(StoreError::Undecodable { entity, stream, .. })
                if *entity == GOAL && stream == "G2"
        ),
        "the later schema's deletion of G2 is reported: {check:?}"
    );
}

/// The view judges an instance by its newest record: G2, proposed as Draft and then moved by a
/// later schema to a state this build cannot read, is hidden, not shown in its older readable
/// state. No case of the unit's has an unreadable record on top of a readable one.
#[test]
fn adv_w05_an_unreadable_newest_record_hides_the_older_readable_state() {
    let state = state_dir("newest-unreadable");
    propose_kept(&state, "G1", "Readable");
    propose_kept(&state, "G2", "Moved on by a later build");
    append_raw(
        &state,
        GOAL,
        "G2",
        "stored",
        2,
        Expected::Exact(1),
        later_goal("G2", "Dropped"),
    );

    let reopened = open(&state);
    let listed = rows(&reopened);
    let check = reopened.ports.check();
    assert_eq!(
        listed,
        vec![row("G1", GoalState::Draft, "Readable")],
        "G2's older Draft is not shown under a newer record this build cannot read"
    );
    assert!(
        matches!(
            &check,
            Err(StoreError::Undecodable { stream, .. }) if stream == "G2"
        ),
        "{check:?}"
    );
}

/// And the other way round: an unreadable record under a readable newest one does not hide the
/// instance and is not reported.
#[test]
fn adv_w05_an_unreadable_older_record_under_a_readable_newest_one_is_shown() {
    let state = state_dir("older-unreadable");
    append_raw(
        &state,
        GOAL,
        "G2",
        "stored",
        2,
        Expected::NoStream,
        later_goal("G2", "Draft"),
    );
    append_raw(
        &state,
        GOAL,
        "G2",
        "stored",
        1,
        Expected::Exact(1),
        json!({
            "state": "Confirmed",
            "goal_id": "G2",
            "title": "Readable on top",
            "exit_evidence": "e",
        }),
    );

    let reopened = open(&state);
    assert_eq!(
        rows(&reopened),
        vec![row("G2", GoalState::Confirmed, "Readable on top")]
    );
    reopened
        .ports
        .check()
        .expect("only the newest record is judged");
}

/// A write on a handle whose view met an unreadable record: the behaviour still reports
/// `Proposed`, the store refuses the write, and `check()` answers the unreadable G2, not the
/// refused G4.
#[test]
fn adv_w05_a_write_after_an_unreadable_read_is_refused_and_reported() {
    let state = state_dir("write-after-unreadable");
    propose_kept(&state, "G1", "Readable");
    append_raw(
        &state,
        GOAL,
        "G2",
        "stored",
        2,
        Expected::NoStream,
        later_goal("G2", "Draft"),
    );

    let mut handle = open(&state);
    assert_eq!(rows(&handle), vec![row("G1", GoalState::Draft, "Readable")]);
    let outcome = propose(&mut handle, "G4", "After the view");
    assert!(
        matches!(outcome, ProposeGoalOutcome::Proposed { .. }),
        "{outcome:?}"
    );
    let check = handle.ports.check();
    assert!(
        matches!(&check, Err(StoreError::Undecodable { stream, .. }) if stream == "G2"),
        "{check:?}"
    );
    drop(handle);
    assert_eq!(
        rows(&open(&state)),
        vec![row("G1", GoalState::Draft, "Readable")],
        "the refused G4 is not kept"
    );
}

/// Two handles inside a caller's runtime keep the `Exists` and `Moved` refusals through the
/// store's scoped-thread path.
#[test]
fn adv_w05_two_handles_inside_a_runtime_keep_exists_and_moved() {
    let state = state_dir("two-handles-inside");
    callers_runtime().block_on(async {
        let mut first = open(&state);
        let mut second = open(&state);

        assert!(GoalStorage::get(&second.ports, &id("G1")).is_none());
        propose(&mut first, "G1", "From the first handle");
        first.ports.check().expect("the first proposal is kept");
        GoalStorage::put(
            &mut second.ports,
            GoalSnapshot {
                state: GoalState::Draft,
                data: GoalData {
                    goal_id: id("G1"),
                    title: "From the second handle".to_owned(),
                    exit_evidence: "e".to_owned(),
                },
            },
        );
        let check = second.ports.check();
        assert!(
            matches!(&check, Err(StoreError::Exists { identity, .. }) if identity == "G1"),
            "{check:?}"
        );

        let mut third = open(&state);
        let held = GoalStorage::get(&third.ports, &id("G1")).expect("third reads G1");
        let confirmed = first
            .confirm_goal(ConfirmGoal { goal_id: id("G1") })
            .expect("ConfirmGoal decides an outcome");
        assert!(matches!(confirmed, ConfirmGoalOutcome::Confirmed { .. }));
        first.ports.check().expect("the confirmation is kept");
        let mut stale = held;
        stale.state = GoalState::Dropped;
        GoalStorage::put(&mut third.ports, stale);
        let check = third.ports.check();
        assert!(
            matches!(
                &check,
                Err(StoreError::Moved { identity, read: 1, held: 2, .. }) if identity == "G1"
            ),
            "{check:?}"
        );
    });
    assert_eq!(
        rows(&open(&state)),
        vec![row("G1", GoalState::Confirmed, "From the first handle")]
    );
}

/// A handle opened inside a caller's runtime and handed out of it answers, writes and drops on a
/// thread with no runtime.
#[test]
fn adv_w05_a_handle_opened_inside_a_runtime_works_outside_it() {
    let state = state_dir("opened-inside-used-outside");
    let mut handle = callers_runtime().block_on(async { open(&state) });
    propose(&mut handle, "G1", "Opened inside");
    handle.ports.check().expect("the proposal is kept");
    assert_eq!(
        rows(&handle),
        vec![row("G1", GoalState::Draft, "Opened inside")]
    );
    drop(handle);
    assert_eq!(
        rows(&open(&state)),
        vec![row("G1", GoalState::Draft, "Opened inside")]
    );
}

// ---------------------------------------------------------------------------------------------
// Wave 05 U6: the rest of each class the cases above are one instance of
// ---------------------------------------------------------------------------------------------

/// The views the cases above and `tests/adv_snapshot.rs` do not run: each lists its readable row
/// beside two records it cannot read (one at a later schema version, one naming a state this build
/// does not know), names each of them on a line of its own on standard error, and exits 1.
#[test]
fn adv_w05_every_other_view_lists_its_readable_row_and_names_each_unreadable_record() {
    const SERVING: &str = "conductor.direction.GoalServing";
    const REQUEST: &str = "conductor.dispatch.ResourceRequest";
    const CONTROLLER: &str = "conductor.dispatch.Controller";
    let readable = "11111111-1111-4111-8111-111111111111";
    let state = state_dir("every-other-view");
    {
        let mut handle = store::open(&state).expect("open the store");
        GoalServingStorage::put(
            &mut handle,
            GoalServingSnapshot {
                state: GoalServingState::Serving,
                data: GoalServingData {
                    serving_id: ServingId(Uuid(readable.to_owned())),
                    goal_id: id("G1"),
                    repository: RepositoryName("delta".to_owned()),
                },
            },
        );
        ResourceRequestStorage::put(
            &mut handle,
            ResourceRequestSnapshot {
                state: ResourceRequestState::Requested,
                data: ResourceRequestData {
                    resource_request_id: ResourceRequestId(Uuid(readable.to_owned())),
                    repository: RepositoryName("delta".to_owned()),
                    resource: ResourceKind::BuildSlot,
                    amount: 1,
                    requested_at: Timestamp("2026-10-07T00:00:00Z".to_owned()),
                },
            },
        );
        ControllerStorage::put(
            &mut handle,
            ControllerSnapshot {
                state: ControllerState::Running,
                data: ControllerData {
                    controller_id: ControllerId(Uuid(readable.to_owned())),
                    repository: RepositoryName("delta".to_owned()),
                    harness: Harness::Claude,
                    session_name: SessionName("delta".to_owned()),
                    charter_revision: 1,
                },
            },
        );
        handle.check().expect("the readable rows are kept");
    }
    let views: [(&[&str], &str, &str, Value); 3] = [
        (
            &["goal", "servings"],
            SERVING,
            "serving_id",
            json!({"state": "Serving", "goal_id": "G1", "repository": "delta"}),
        ),
        (
            &["resource", "resource-requests"],
            REQUEST,
            "resource_request_id",
            json!({"state": "Requested", "repository": "delta", "resource": "BuildSlot",
                   "amount": 1, "requested_at": "2026-10-07T00:00:00Z"}),
        ),
        (
            &["controller", "controllers"],
            CONTROLLER,
            "controller_id",
            json!({"state": "Running", "repository": "delta", "harness": "Claude",
                   "session_name": "delta", "charter_revision": 1}),
        ),
    ];
    let mut problems = Vec::new();
    for (n, (args, entity, key, fields)) in views.iter().enumerate() {
        let later = format!("22222222-2222-4222-8222-00000000000{n}");
        let unknown = format!("33333333-3333-4333-8333-00000000000{n}");
        let mut body = fields.clone();
        body[*key] = Value::from(later.as_str());
        append_raw(
            &state,
            entity,
            &later,
            "stored",
            2,
            Expected::NoStream,
            body,
        );
        let mut body = fields.clone();
        body[*key] = Value::from(unknown.as_str());
        body["state"] = Value::from("Superseded");
        append_raw(
            &state,
            entity,
            &unknown,
            "stored",
            1,
            Expected::NoStream,
            body,
        );

        let output = conductor(&state, &[args, &["--format", "jsonl"][..]].concat());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        let named_once = |stream: &str| lines.iter().filter(|l| l.contains(stream)).count() == 1;
        if !stdout.contains(readable)
            || stdout.contains(&later)
            || stdout.contains(&unknown)
            || lines.len() != 2
            || !named_once(&later)
            || !named_once(&unknown)
            || output.status.code() != Some(1)
        {
            problems.push(format!(
                "`{}`: status {:?}, stdout {stdout:?}, stderr {stderr:?}",
                args.join(" "),
                output.status.code()
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of 3 views do not list the readable row and name each unreadable record once:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// `story:store-hardening`, row "Added 2026-10-06": a `get` on an id no stream can carry answers
/// absent on every storage port the store implements, latches nothing, and a later write on the
/// same handle is kept.
#[test]
fn adv_w05_a_get_on_an_id_no_stream_can_carry_answers_absent_on_every_port() {
    for (n, key) in [String::new(), "%".repeat(171)].into_iter().enumerate() {
        let state = state_dir(&format!("unkeepable-get-{n}"));
        let mut handle = store::open(&state).expect("open the store");
        let uuid = || Uuid(key.clone());
        let observation = || ObservationId(uuid());
        let answered = [
            ("Goal", GoalStorage::get(&handle, &id(&key)).is_some()),
            (
                "GuardDecision",
                GuardDecisionStorage::get(&handle, &GuardDecisionId(uuid())).is_some(),
            ),
            (
                "GoalServing",
                GoalServingStorage::get(&handle, &ServingId(uuid())).is_some(),
            ),
            (
                "ResourceRequest",
                ResourceRequestStorage::get(&handle, &ResourceRequestId(uuid())).is_some(),
            ),
            (
                "Controller",
                ControllerStorage::get(&handle, &ControllerId(uuid())).is_some(),
            ),
            (
                "Snapshot",
                SnapshotStorage::get(&handle, &SnapshotId(uuid())).is_some(),
            ),
            (
                "RepositoryObservation",
                RepositoryObservationStorage::get(&handle, &observation()).is_some(),
            ),
            (
                "PullRequestObservation",
                PullRequestObservationStorage::get(&handle, &observation()).is_some(),
            ),
            (
                "WorkflowRunObservation",
                WorkflowRunObservationStorage::get(&handle, &observation()).is_some(),
            ),
            (
                "SessionObservation",
                SessionObservationStorage::get(&handle, &observation()).is_some(),
            ),
            (
                "BlockerObservation",
                BlockerObservationStorage::get(&handle, &observation()).is_some(),
            ),
            (
                "SpecificationObservation",
                SpecificationObservationStorage::get(&handle, &observation()).is_some(),
            ),
        ];
        let present: Vec<&str> = answered
            .iter()
            .filter(|(_, present)| *present)
            .map(|(port, _)| *port)
            .collect();
        assert!(present.is_empty(), "id {n}: answered present: {present:?}");
        let check = handle.check();
        assert!(
            check.is_ok(),
            "id {n}: a get on an id no stream can carry latched the handle: {check:?}"
        );
        GoalStorage::put(
            &mut handle,
            GoalSnapshot {
                state: GoalState::Draft,
                data: GoalData {
                    goal_id: id("G1"),
                    title: "After".to_owned(),
                    exit_evidence: "e".to_owned(),
                },
            },
        );
        handle.check().expect("a write after the reads is kept");
        drop(handle);
        assert_eq!(
            rows(&open(&state)),
            vec![row("G1", GoalState::Draft, "After")]
        );
    }
}
