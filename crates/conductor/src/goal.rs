//! `conductor goal`: the goals dispatches serve, and the repositories serving each.
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else `state/`; [`crate::state::open`]), runs the generated behaviour, and checks
//! that the store kept what the behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0; one that assigns an identity prints it on one line. A refused command
//! exits 1 with one line on standard error: the command, the error the specification declares, and
//! what it means. A flag left out is named on standard error (exit 1) before any store is opened.
//! `--input-json` is not read yet, and answers [`NotImplemented`](crate::NotImplemented).
//!
//! A view opens only an existing store ([`crate::state::open_existing`]) and creates nothing. It
//! lists every row it can read, and answers through [`crate::store::show`]: a record it cannot
//! read is named on standard error and exits 1, after the other rows.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    Generated, GoalServingStorage, GoalStorage, TryContext, unmet_context,
};
use conductor_model::direction::obligations::{
    AddServingBehavior, ConfirmGoalBehavior, DropGoalBehavior, GoalServingsQuery,
    MarkGoalMetBehavior, ProposeGoalBehavior, RemoveServingBehavior,
};
use conductor_model::direction::{
    AddServing, AddServingOutcome, ConfirmGoal, ConfirmGoalOutcome, DropGoal, DropGoalOutcome,
    GoalId, GoalServingSnapshot, GoalServingState, GoalSnapshot, MarkGoalMet, MarkGoalMetOutcome,
    ProposeGoal, ProposeGoalOutcome, RemoveServing, RemoveServingOutcome, ServingId,
};
use conductor_model::dispatch::{GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::Uuid;
use serde_json::Value;

use crate::cli::{
    AddServingArgs, ConfirmGoalArgs, DropGoalArgs, JsonInput, MarkGoalMetArgs, ProposeGoalArgs,
    RemoveServingArgs, ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// `conductor goal propose-goal`: the command `conductor.direction.ProposeGoal`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the goal, or the specification refuses it: `InvalidGoalId`,
/// `GoalExists`.
pub fn propose_goal(state: Option<&Path>, args: ProposeGoalArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal propose-goal";
    flags_only(args.input_json, "goal propose-goal --input-json")?;
    let input = ProposeGoal {
        goal_id: GoalId(flag(COMMAND, "--goal-id", args.goal_id)?),
        title: flag(COMMAND, "--title", args.title)?,
        exit_evidence: flag(COMMAND, "--exit-evidence", args.exit_evidence)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .propose_goal(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        ProposeGoalOutcome::Proposed { .. } => Ok(ExitCode::SUCCESS),
        ProposeGoalOutcome::InvalidGoalId { error } => {
            refused(COMMAND, "InvalidGoalId", &invalid_goal_id(&error.goal_id))
        }
        ProposeGoalOutcome::AlreadyProposed { error } => refused(
            COMMAND,
            "GoalExists",
            &format!(
                "a goal already has the id {:?}; the held goal is unchanged",
                error.goal_id.0
            ),
        ),
    }
}

/// `conductor goal confirm-goal`: the command `conductor.direction.ConfirmGoal`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `InvalidGoalId`,
/// `GoalNotFound`, `GoalStateConflict`.
pub fn confirm_goal(state: Option<&Path>, args: ConfirmGoalArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal confirm-goal";
    flags_only(args.input_json, "goal confirm-goal --input-json")?;
    let input = ConfirmGoal {
        goal_id: GoalId(flag(COMMAND, "--goal-id", args.goal_id)?),
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .confirm_goal(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        ConfirmGoalOutcome::Confirmed { .. } => Ok(ExitCode::SUCCESS),
        ConfirmGoalOutcome::InvalidGoalId { error } => {
            refused(COMMAND, "InvalidGoalId", &invalid_goal_id(&error.goal_id))
        }
        ConfirmGoalOutcome::NoSuchGoal { error } => {
            refused(COMMAND, "GoalNotFound", &no_goal(&error.goal_id))
        }
        ConfirmGoalOutcome::WrongState { error } => refused(
            COMMAND,
            "GoalStateConflict",
            &format!(
                "goal {:?} is not a draft, so nothing moved",
                error.goal_id.0
            ),
        ),
    }
}

/// `conductor goal mark-goal-met`: the command `conductor.direction.MarkGoalMet`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `InvalidGoalId`,
/// `GoalNotFound`, `GoalStateConflict`.
pub fn mark_goal_met(state: Option<&Path>, args: MarkGoalMetArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal mark-goal-met";
    flags_only(args.input_json, "goal mark-goal-met --input-json")?;
    let input = MarkGoalMet {
        goal_id: GoalId(flag(COMMAND, "--goal-id", args.goal_id)?),
        evidence: flag(COMMAND, "--evidence", args.evidence)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .mark_goal_met(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        MarkGoalMetOutcome::Met { .. } => Ok(ExitCode::SUCCESS),
        MarkGoalMetOutcome::InvalidGoalId { error } => {
            refused(COMMAND, "InvalidGoalId", &invalid_goal_id(&error.goal_id))
        }
        MarkGoalMetOutcome::NoSuchGoal { error } => {
            refused(COMMAND, "GoalNotFound", &no_goal(&error.goal_id))
        }
        MarkGoalMetOutcome::WrongState { error } => refused(
            COMMAND,
            "GoalStateConflict",
            &format!(
                "goal {:?} is not confirmed, and only a confirmed goal can be met",
                error.goal_id.0
            ),
        ),
    }
}

/// `conductor goal drop-goal`: the command `conductor.direction.DropGoal`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `InvalidGoalId`,
/// `GoalNotFound`, `GoalStateConflict`.
pub fn drop_goal(state: Option<&Path>, args: DropGoalArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal drop-goal";
    flags_only(args.input_json, "goal drop-goal --input-json")?;
    let input = DropGoal {
        goal_id: GoalId(flag(COMMAND, "--goal-id", args.goal_id)?),
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .drop_goal(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        DropGoalOutcome::Dropped { .. } => Ok(ExitCode::SUCCESS),
        DropGoalOutcome::InvalidGoalId { error } => {
            refused(COMMAND, "InvalidGoalId", &invalid_goal_id(&error.goal_id))
        }
        DropGoalOutcome::NoSuchGoal { error } => {
            refused(COMMAND, "GoalNotFound", &no_goal(&error.goal_id))
        }
        DropGoalOutcome::WrongState { error } => refused(
            COMMAND,
            "GoalStateConflict",
            &format!("goal {:?} is already met or dropped", error.goal_id.0),
        ),
    }
}

/// `conductor goal add-serving`: the command `conductor.direction.AddServing`. Prints the id of the
/// serving it adds.
///
/// The generated behaviour reads the goal by `--goal-id` before any guard, and the store cannot
/// read an id it cannot keep. So the id is first checked against the bound `story:goal-id-bounds`
/// set, and one no goal can have is refused here as `InvalidGoalId`, before the store is opened.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the serving, the id cannot be printed, or the command is refused:
/// `InvalidGoalId`, `GoalNotFound`, `GoalStateConflict`.
pub fn add_serving(state: Option<&Path>, args: AddServingArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal add-serving";
    flags_only(args.input_json, "goal add-serving --input-json")?;
    let input = AddServing {
        goal_id: GoalId(flag(COMMAND, "--goal-id", args.goal_id)?),
        repository: RepositoryName(flag(COMMAND, "--repository", args.repository)?),
    };
    if !admitted(&input.goal_id)? {
        return refused(COMMAND, "InvalidGoalId", &invalid_goal_id(&input.goal_id));
    }
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .add_serving(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        AddServingOutcome::Added { serving_added } => {
            print_line(COMMAND, &serving_added.serving_id.0.0)
        }
        AddServingOutcome::NoSuchGoal { error } => {
            refused(COMMAND, "GoalNotFound", &no_goal(&error.goal_id))
        }
        AddServingOutcome::GoalClosed { error } => refused(
            COMMAND,
            "GoalStateConflict",
            &format!(
                "goal {:?} is met or dropped, so it gains no serving repository",
                error.goal_id.0
            ),
        ),
    }
}

/// `conductor goal remove-serving`: the command `conductor.direction.RemoveServing`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `ServingNotFound`,
/// `ServingStateConflict`.
pub fn remove_serving(state: Option<&Path>, args: RemoveServingArgs) -> Result<ExitCode> {
    const COMMAND: &str = "goal remove-serving";
    flags_only(args.input_json, "goal remove-serving --input-json")?;
    let input = RemoveServing {
        serving_id: ServingId(Uuid(flag(COMMAND, "--serving-id", args.serving_id)?)),
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .remove_serving(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        RemoveServingOutcome::Removed { .. } => Ok(ExitCode::SUCCESS),
        RemoveServingOutcome::NoSuchServing { error } => refused(
            COMMAND,
            "ServingNotFound",
            &format!("no serving repository has the id {}", error.serving_id.0.0),
        ),
        RemoveServingOutcome::WrongState { error } => refused(
            COMMAND,
            "ServingStateConflict",
            &format!(
                "serving {} was already removed from its goal",
                error.serving_id.0.0
            ),
        ),
    }
}

/// `conductor goal goals`: the view `conductor.direction.Goals`, served by the generated query over
/// the existing store under the state directory (`--state-dir`, else `state/`;
/// [`crate::state::dir`]) and rendered as `--format` asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn goals(state: Option<&std::path::Path>, view: ViewArgs) -> Result<ExitCode> {
    use anyhow::anyhow;
    use conductor_model::behaviour::Generated;
    use conductor_model::direction::GoalState;
    use conductor_model::direction::obligations::GoalsQuery;
    use serde_json::Value;

    /// The view's fields, in the specification's order (`spec/domains/direction.yaml`).
    const COLUMNS: [&str; 4] = ["goal_id", "state", "title", "exit_evidence"];

    /// The state's name as the specification spells it. Exhaustive, so a state the specification
    /// adds fails to compile here rather than rendering under a guessed name.
    fn state_name(state: GoalState) -> &'static str {
        match state {
            GoalState::Confirmed => "Confirmed",
            GoalState::Draft => "Draft",
            GoalState::Dropped => "Dropped",
            GoalState::Met => "Met",
        }
    }

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .goals()
        .map_err(|unmet| anyhow!("goal goals: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.goal_id.0),
                Value::from(state_name(row.state)),
                Value::from(row.title),
                Value::from(row.exit_evidence),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "goals", &view.render(&COLUMNS, &rows))
}

/// `conductor goal servings`: the view `conductor.direction.GoalServings`, served by the generated
/// query over the existing store under the state directory (`--state-dir`, else `state/`;
/// [`crate::state::dir`]) and rendered as `--format` asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn goal_servings(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/direction.yaml`).
    const COLUMNS: [&str; 4] = ["serving_id", "state", "goal_id", "repository"];

    /// The state's name as the specification spells it. Exhaustive, so a state the specification
    /// adds fails to compile here rather than rendering under a guessed name.
    fn state_name(state: GoalServingState) -> &'static str {
        match state {
            GoalServingState::Removed => "Removed",
            GoalServingState::Serving => "Serving",
        }
    }

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .goal_servings()
        .map_err(|unmet| anyhow!("goal servings: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.serving_id.0.0),
                Value::from(state_name(row.state)),
                Value::from(row.goal_id.0),
                Value::from(row.repository.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "serving repositories",
        &view.render(&COLUMNS, &rows),
    )
}

/// Whether the specification admits `goal` as a goal id at all: the bound `story:goal-id-bounds`
/// set, decided by the generated `invalid-goal-id` guard of `ConfirmGoal` over a port that holds
/// no goal, so the bound is read from the generated code rather than restated here. Nothing is
/// stored: an admitted id is answered `no-such-goal` by that port.
fn admitted(goal: &GoalId) -> Result<bool> {
    /// A goal port that holds no goal and keeps nothing.
    struct NoGoals;

    impl GoalStorage for NoGoals {
        fn get(&self, _identity: &GoalId) -> Option<GoalSnapshot> {
            None
        }

        fn put(&mut self, _snapshot: GoalSnapshot) {}

        fn delete(&mut self, _identity: &GoalId) {}

        fn list(&self) -> Vec<GoalSnapshot> {
            Vec::new()
        }
    }

    let outcome = Generated::new(NoGoals)
        .confirm_goal(ConfirmGoal {
            goal_id: goal.clone(),
        })
        .map_err(|unmet| anyhow!("goal add-serving: check the goal id: {unmet}"))?;
    Ok(!matches!(outcome, ConfirmGoalOutcome::InvalidGoalId { .. }))
}

/// The ports `AddServing` runs over: the store, and the serving id it assigns.
struct Ports {
    store: Store,
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
        GoalServingStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: GoalServingSnapshot) {
        GoalServingStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &ServingId) {
        GoalServingStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<GoalServingSnapshot> {
        GoalServingStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Ok(ServingId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }
}

/// Refuses `--input-json`, which no handler reads yet, as `what` not implemented.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `name`, or an error naming it.
fn flag<T>(command: &str, name: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| anyhow!("{command}: {name} is missing"))
}

/// Whether the store kept every read and write the behaviour made; an outcome is answered only
/// then.
fn kept(command: &str, store: &Store) -> Result<()> {
    store
        .check()
        .with_context(|| format!("{command}: the store did not keep it"))
}

/// The refusal `error` of `command`, as the one line the binary writes on standard error.
fn refused(command: &str, error: &str, detail: &str) -> Result<ExitCode> {
    bail!("{command}: {error}: {detail}")
}

/// Writes `line` to standard output.
fn print_line(command: &str, line: &str) -> Result<ExitCode> {
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}

fn invalid_goal_id(goal: &GoalId) -> String {
    format!(
        "the goal id {:?} is empty or longer than 170 UTF-8 bytes, so no goal can have it and \
         nothing moved",
        goal.0
    )
}

fn no_goal(goal: &GoalId) -> String {
    format!("no goal has the id {:?}", goal.0)
}
