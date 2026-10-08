//! Adversary cases for `story:goal-id-bounds`, wave 04, pass 1: the four `invalid-goal-id` guards
//! in `spec/domains/direction.yaml` against each other and against what the store already holds.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::store::{self, Store};
use conductor_model::behaviour::{Generated, GoalStorage};
use conductor_model::direction::obligations::{
    ConfirmGoalBehavior, DropGoalBehavior, GoalsQuery, MarkGoalMetBehavior, ProposeGoalBehavior,
};
use conductor_model::direction::{
    ConfirmGoal, ConfirmGoalOutcome, DropGoal, DropGoalOutcome, GoalData, GoalId, GoalSnapshot,
    GoalState, MarkGoalMet, MarkGoalMetOutcome, ProposeGoal, ProposeGoalOutcome,
};

/// The bound `spec/domains/direction.yaml` states on all four commands.
const BOUND: usize = 170;

fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_goal_id_bounds")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn open(state: &Path) -> Generated<Store> {
    Generated::new(store::open(state).expect("open the store"))
}

fn id(text: &str) -> GoalId {
    GoalId(text.to_owned())
}

/// `unit` repeated to fill the bound, then `fill`; exactly `BOUND` bytes.
fn at_bound(unit: &str, fill: &str) -> String {
    let mut goal = unit.repeat((BOUND - fill.len()) / unit.len());
    goal.push_str(fill);
    assert_eq!(goal.len(), BOUND, "{goal:?} sits at the bound");
    goal
}

fn propose(generated: &mut Generated<Store>, goal: &str) -> ProposeGoalOutcome {
    generated
        .propose_goal(ProposeGoal {
            goal_id: id(goal),
            title: "title".to_owned(),
            exit_evidence: "evidence".to_owned(),
        })
        .expect("ProposeGoal decides an outcome")
}

/// Every id `ProposeGoal` admits is one `ConfirmGoal`, `MarkGoalMet` and `DropGoal` also admit:
/// a goal proposed at the bound can be confirmed, met or dropped, and the reopened store holds the
/// state each one ended in.
///
/// The unit's own case sends only refused ids to the three commands that move a goal, so the four
/// bounds are tied together from above (171 is refused by all four) and not from below. A
/// specification whose `ConfirmGoal`, `MarkGoalMet` or `DropGoal` bound is 169 keeps the unit's
/// suite and the `goal-id-bounds` scenario green, and leaves every 170-byte goal a draft forever.
#[test]
fn adv_a_goal_proposed_at_the_bound_can_be_confirmed_met_and_dropped() {
    let state = state_dir("moves-at-bound");
    let met: Vec<(&str, String)> = vec![
        ("one byte", "M".to_owned()),
        ("170 ASCII letters", at_bound("m", "")),
        ("170 percent signs", at_bound("%", "")),
        (
            "42 four-byte characters and a two-byte one",
            at_bound("\u{1f3af}", "\u{e9}"),
        ),
        (
            "56 three-byte characters and 2 ASCII",
            at_bound("\u{76ee}", "mm"),
        ),
        ("controls, slash and NUL", at_bound(" \t\n\u{0}/", "")),
    ];
    let dropped: Vec<(&str, String)> = vec![
        ("one byte", "D".to_owned()),
        ("170 ASCII letters", at_bound("d", "")),
        ("169 percent signs and one letter", at_bound("%", "d")),
        ("85 two-byte characters", at_bound("\u{e9}", "")),
    ];

    let mut generated = open(&state);
    for (what, goal) in met.iter().chain(dropped.iter()) {
        let proposed = propose(&mut generated, goal);
        assert!(
            matches!(proposed, ProposeGoalOutcome::Proposed { .. }),
            "{what}: ProposeGoal admits it: {proposed:?}"
        );
    }
    for (what, goal) in &met {
        let confirmed = generated
            .confirm_goal(ConfirmGoal { goal_id: id(goal) })
            .expect("ConfirmGoal decides an outcome");
        assert!(
            matches!(confirmed, ConfirmGoalOutcome::Confirmed { .. }),
            "{what}: ConfirmGoal admits what ProposeGoal admitted: {confirmed:?}"
        );
        let marked = generated
            .mark_goal_met(MarkGoalMet {
                goal_id: id(goal),
                evidence: "evidence".to_owned(),
            })
            .expect("MarkGoalMet decides an outcome");
        assert!(
            matches!(marked, MarkGoalMetOutcome::Met { .. }),
            "{what}: MarkGoalMet admits what ProposeGoal admitted: {marked:?}"
        );
    }
    for (what, goal) in &dropped {
        let gone = generated
            .drop_goal(DropGoal {
                goal_id: id(goal),
                reason: "reason".to_owned(),
            })
            .expect("DropGoal decides an outcome");
        assert!(
            matches!(gone, DropGoalOutcome::Dropped { .. }),
            "{what}: DropGoal admits what ProposeGoal admitted: {gone:?}"
        );
    }
    generated
        .ports
        .check()
        .unwrap_or_else(|failure| panic!("the store kept every move: {failure}"));
    drop(generated);

    let reopened = open(&state);
    let held: Vec<(String, GoalState)> = reopened
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| (row.goal_id.0, row.state))
        .collect();
    let expected: Vec<(String, GoalState)> = met
        .iter()
        .map(|(_, goal)| (goal.clone(), GoalState::Met))
        .chain(
            dropped
                .iter()
                .map(|(_, goal)| (goal.clone(), GoalState::Dropped)),
        )
        .collect();
    assert_eq!(held, expected, "the reopened store holds every move");
    reopened
        .ports
        .check()
        .expect("every read after the reopen succeeds");
}

/// Why the case below asserts a listed goal that no command moves, and when that stops being true.
const STORED_PAST_THE_BOUND: &str = "introduced, INFEASIBLE: no binary has written a goal before \
                                     the bound; a store written before it needs a migration story \
                                     if one ever exists";

/// A goal the store already holds under an id past the bound (written through the same port the
/// base commit's `ProposeGoal` wrote through, which kept any id of 1 to 512 escaped bytes) is
/// listed by `Goals`, and `ConfirmGoal` answers `invalid-goal-id`, "No goal can have this id".
/// The view and the refusal disagree. That is today's behaviour, and this case asserts it.
#[test]
fn adv_a_goal_stored_past_the_bound_is_listed_and_refused_as_one_no_goal_can_have() {
    let state = state_dir("stored-past-bound");
    let goal = "e".repeat(200);
    let mut generated = open(&state);
    GoalStorage::put(
        &mut generated.ports,
        GoalSnapshot {
            state: GoalState::Draft,
            data: GoalData {
                goal_id: id(&goal),
                title: "kept before the bound".to_owned(),
                exit_evidence: "evidence".to_owned(),
            },
        },
    );
    generated
        .ports
        .check()
        .expect("the store keeps a 200-byte ASCII id");
    drop(generated);

    let mut reopened = open(&state);
    let listed: Vec<String> = reopened
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| row.goal_id.0)
        .collect();
    let confirmed = reopened
        .confirm_goal(ConfirmGoal { goal_id: id(&goal) })
        .expect("ConfirmGoal decides an outcome");
    assert_eq!(
        listed,
        vec![goal.clone()],
        "Goals lists the 200-byte goal today; {STORED_PAST_THE_BOUND}"
    );
    assert!(
        matches!(&confirmed, ConfirmGoalOutcome::InvalidGoalId { error } if error.goal_id == id(&goal)),
        "ConfirmGoal refuses the listed 200-byte goal as one no goal can have today, got \
         {confirmed:?}; {STORED_PAST_THE_BOUND}"
    );
}
