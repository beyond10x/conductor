//! `story:local-store`: the generated `ProposeGoal` behaviour, driven directly against the store
//! rather than through a CLI handler, answers from records after the store is reopened.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`.

use std::fs;
use std::path::PathBuf;

use conductor_cli::store::{self, StoreError};
use conductor_model::behaviour::{Generated, GoalStorage};
use conductor_model::direction::obligations::{GoalsQuery, ProposeGoalBehavior};
use conductor_model::direction::{
    Goal, GoalData, GoalExists, GoalId, GoalProposed, GoalState, Goals, ProposeGoal,
    ProposeGoalOutcome,
};

/// An empty state directory for one case.
fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("store")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn g9() -> GoalId {
    GoalId("G9".to_owned())
}

fn propose_g9(title: &str, exit_evidence: &str) -> ProposeGoal {
    ProposeGoal {
        goal_id: g9(),
        title: title.to_owned(),
        exit_evidence: exit_evidence.to_owned(),
    }
}

fn g9_draft() -> Goals {
    Goals {
        goal_id: g9(),
        state: GoalState::Draft,
        title: "Local store".to_owned(),
        exit_evidence: "a reopened store lists G9".to_owned(),
    }
}

#[test]
fn a_reopened_store_lists_a_proposed_goal_as_draft_and_refuses_it_again() {
    let state = state_dir("reopen");

    // 1. Propose G9.
    let mut generated = Generated::new(store::open(&state).expect("open the store"));
    let proposed = generated
        .propose_goal(propose_g9("Local store", "a reopened store lists G9"))
        .expect("ProposeGoal decides an outcome");
    assert_eq!(
        proposed,
        ProposeGoalOutcome::Proposed {
            goal_proposed: GoalProposed {
                goal_id: g9(),
                title: "Local store".to_owned(),
            },
        }
    );
    generated
        .ports
        .check()
        .expect("the store kept the accepted outcome");
    assert!(
        state.join("tree").is_dir(),
        "the store lives under state/tree/"
    );

    // 2. Drop the store and reopen it from the same directory.
    drop(generated);
    let mut generated = Generated::new(store::open(&state).expect("reopen the store"));

    // 3. The Goals view lists G9 as Draft.
    assert_eq!(generated.goals().expect("Goals answers"), vec![g9_draft()]);

    // 4. A second ProposeGoal for G9 is refused as an existing instance.
    let again = generated
        .propose_goal(propose_g9("Another title", "other evidence"))
        .expect("ProposeGoal decides an outcome");
    assert_eq!(
        again,
        ProposeGoalOutcome::AlreadyProposed {
            error: GoalExists { goal_id: g9() },
        }
    );
    generated.ports.check().expect("a refusal is not a failure");
    drop(generated);

    // The refusal wrote nothing: a third handle still reads the first proposal, once.
    let generated = Generated::new(store::open(&state).expect("reopen the store again"));
    assert_eq!(generated.goals().expect("Goals answers"), vec![g9_draft()]);
}

#[test]
fn the_store_refuses_a_creation_another_handle_made_first() {
    let state = state_dir("race");
    let mut late = store::open(&state).expect("open the late handle");
    assert_eq!(
        GoalStorage::get(&late, &g9()),
        None,
        "the late handle reads G9 as absent"
    );

    let mut first = Generated::new(store::open(&state).expect("open the first handle"));
    first
        .propose_goal(propose_g9("Local store", "a reopened store lists G9"))
        .expect("ProposeGoal decides an outcome");
    first.ports.check().expect("the first creation is kept");

    // The late handle decided on "absent"; the store holds G9 by now and refuses the write.
    let data = GoalData {
        goal_id: g9(),
        title: "Late title".to_owned(),
        exit_evidence: "late evidence".to_owned(),
    };
    GoalStorage::put(
        &mut late,
        conductor_model::direction::AnyGoal::Draft(Goal::new(data)).snapshot(),
    );
    match late.check() {
        Err(StoreError::Exists { entity, identity }) => {
            assert_eq!(entity, "conductor.direction.Goal");
            assert_eq!(identity, "G9");
        }
        other => panic!("expected the store to refuse G9 as existing, got {other:?}"),
    }
    drop(late);

    let reopened = Generated::new(store::open(&state).expect("reopen the store"));
    assert_eq!(reopened.goals().expect("Goals answers"), vec![g9_draft()]);
}
