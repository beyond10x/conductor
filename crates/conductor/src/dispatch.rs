//! `conductor dispatch`: briefs to controllers and their reports (design § 4, § 6).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else the instance's `state` when a config file names it, else `state/`;
//! [`crate::state::open`]), runs the generated behaviour, and checks that the store kept what the
//! behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0; `send-dispatch` prints the dispatch's id on one line. A refused
//! command exits 1 with one line on standard error: the command, the error the specification
//! declares, and what it means. A flag left out is named on standard error (exit 1) before any
//! store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! `send-dispatch` without `--dispatch-id` allocates `DSP-YYYYMMDD-NN`: the UTC date of
//! `--sent-at`, and one more than the highest number the store holds for that date, at least two
//! digits. A `--sent-at` whose UTC year is outside 1 to 9999 is refused. An id given by hand is
//! in that form ([`allocator_form`](crate::decide::allocator_form)), or it is refused naming the
//! form; one a dispatch already has is refused (`DispatchExists`), naming it, and nothing is
//! written. When another writer takes an allocated id first, the command allocates again, while
//! the date's highest number moves and at most [`ATTEMPTS`] times in all.
//!
//! The view opens only an existing store ([`crate::state::open_existing`]) and creates nothing. It
//! lists every row it can read, in the order the dispatches were first stored, and answers through
//! [`crate::store::show`].

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{DispatchStorage, Generated};
use conductor_model::direction::GoalId;
use conductor_model::dispatch::obligations::{
    CancelDispatchBehavior, DispatchesQuery, ReportBlockedBehavior, ReportDoneBehavior,
    ReportFailedBehavior, ReportStartedBehavior, ReportUnblockedBehavior, SendDispatchBehavior,
};
use conductor_model::dispatch::{
    CancelDispatch, CancelDispatchOutcome, DispatchId, DispatchState, ReportBlocked,
    ReportBlockedOutcome, ReportDone, ReportDoneOutcome, ReportFailed, ReportFailedOutcome,
    ReportStarted, ReportStartedOutcome, ReportUnblocked, ReportUnblockedOutcome, SendDispatch,
    SendDispatchOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::Value;

use crate::cli::{
    CancelDispatchArgs, JsonInput, ReportBlockedArgs, ReportDoneArgs, ReportFailedArgs,
    ReportStartedArgs, ReportUnblockedArgs, SendDispatchArgs, ViewArgs,
};
use crate::decide::{ATTEMPTS, highest, in_allocator_form, next_id, utc_day};
use crate::not_implemented;
use crate::store::{Store, StoreError};

/// The prefix `conductor.dispatch.DispatchId` declares (`spec/domains/dispatch.yaml`).
const DISPATCH_PREFIX: &str = "DSP-";

/// `conductor.dispatch.Dispatch`, as the store names the entity in a [`StoreError`].
const DISPATCH: &str = "conductor.dispatch.Dispatch";

/// `conductor dispatch send-dispatch`: the command `conductor.dispatch.SendDispatch`. Prints the
/// dispatch's id: the one given, or the one allocated.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)),
/// `--dispatch-id` is not in the allocator's form or names a dispatch that exists, `--sent-at`
/// falls outside the UTC years 1 to 9999, the store does not open or keep the dispatch, another
/// writer took every allocated id, or the specification refuses it: `InvalidPriority`,
/// `WaitsOnItself`, `GoalNotFound`, `GoalClosed`.
pub fn send_dispatch(state: Option<&Path>, args: SendDispatchArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch send-dispatch";
    flags_only(args.input_json, "dispatch send-dispatch --input-json")?;
    let repository = RepositoryName(flag(COMMAND, "--repository", args.repository)?);
    let goal_id = GoalId(flag(COMMAND, "--goal-id", args.goal_id)?);
    let brief = flag(COMMAND, "--brief", args.brief)?;
    let sent_at = flag(COMMAND, "--sent-at", args.sent_at)?;
    let priority = flag(COMMAND, "--priority", args.priority)?;
    let waits_on = args.waits_on.map(DispatchId);
    let day = utc_day(&sent_at).with_context(|| format!("{COMMAND}: --sent-at {sent_at:?}"))?;
    if let Some(given) = &args.dispatch_id {
        in_allocator_form(COMMAND, "--dispatch-id", DISPATCH_PREFIX, given)?;
    }
    let given = args.dispatch_id.map(DispatchId);
    let prefix = format!("{DISPATCH_PREFIX}{day}-");

    let mut before = None;
    for _ in 0..ATTEMPTS {
        // A fresh handle per attempt: a handle refuses every write after its first failure.
        let store = crate::state::open(state)?;
        let dispatch_id = match &given {
            Some(id) => id.clone(),
            None => {
                let highest = stored_highest(COMMAND, &store, &prefix)?;
                if before.replace(highest) == Some(highest) {
                    bail!(
                        "{COMMAND}: the id allocated above {prefix}{highest:02} was taken, yet \
                         the highest number stored for {day} did not move; nothing was sent"
                    );
                }
                DispatchId(next_id(COMMAND, &prefix, highest)?)
            }
        };
        // The behaviour reads the id before it writes (`already-sent`), so the store creates the
        // dispatch only where no stream is, and answers `StoreError::Exists` when another writer
        // created it in between.
        let mut generated = Generated::new(store);
        let outcome = generated
            .send_dispatch(SendDispatch {
                dispatch_id: dispatch_id.clone(),
                repository: repository.clone(),
                goal_id: goal_id.clone(),
                brief: brief.clone(),
                sent_at: Timestamp(sent_at.clone()),
                priority,
                waits_on: waits_on.clone(),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        let held = match generated.ports.check() {
            Ok(()) => matches!(outcome, SendDispatchOutcome::AlreadySent { .. }),
            Err(StoreError::Exists { entity, identity })
                if entity == DISPATCH && identity == dispatch_id.0 =>
            {
                true
            }
            Err(failure) => {
                return Err(failure)
                    .with_context(|| format!("{COMMAND}: the store did not keep it"));
            }
        };
        if held {
            match &given {
                Some(id) => return exists(COMMAND, id),
                // Another writer sent the allocated id first: allocate again.
                None => continue,
            }
        }
        return match outcome {
            SendDispatchOutcome::Sent { dispatch_sent } => {
                print_line(COMMAND, &dispatch_sent.dispatch_id.0)
            }
            SendDispatchOutcome::AlreadySent { error } => exists(COMMAND, &error.dispatch_id),
            SendDispatchOutcome::NegativePriority { error } => refused(
                COMMAND,
                "InvalidPriority",
                &format!(
                    "the priority {} is negative, so nothing was sent",
                    error.priority
                ),
            ),
            SendDispatchOutcome::WaitsOnItself { error } => refused(
                COMMAND,
                "WaitsOnItself",
                &format!(
                    "dispatch {} names itself in --waits-on, so it could never start; nothing \
                     was sent",
                    error.dispatch_id.0
                ),
            ),
            SendDispatchOutcome::NoSuchGoal { error } => refused(
                COMMAND,
                "GoalNotFound",
                &format!(
                    "no goal has the id {:?}, and every dispatch serves a goal; nothing was sent",
                    error.goal_id.0
                ),
            ),
            SendDispatchOutcome::GoalClosed { error } => refused(
                COMMAND,
                "GoalClosed",
                &format!(
                    "goal {:?} is met or dropped, so nothing was sent",
                    error.goal_id.0
                ),
            ),
        };
    }
    bail!(
        "{COMMAND}: another writer took each of the {ATTEMPTS} ids allocated for {day}; nothing \
         was sent"
    )
}

/// `conductor dispatch report-started`: the command `conductor.dispatch.ReportStarted`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_started(state: Option<&Path>, args: ReportStartedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-started";
    flags_only(args.input_json, "dispatch report-started --input-json")?;
    let input = ReportStarted {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
    };
    match run(state, COMMAND, |generated| generated.report_started(input))? {
        ReportStartedOutcome::Started { .. } => Ok(ExitCode::SUCCESS),
        ReportStartedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportStartedOutcome::WrongState { error } => {
            wrong_state(COMMAND, &error.dispatch_id, "only a sent dispatch starts")
        }
    }
}

/// `conductor dispatch report-blocked`: the command `conductor.dispatch.ReportBlocked`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_blocked(state: Option<&Path>, args: ReportBlockedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-blocked";
    flags_only(args.input_json, "dispatch report-blocked --input-json")?;
    let input = ReportBlocked {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    match run(state, COMMAND, |generated| generated.report_blocked(input))? {
        ReportBlockedOutcome::Blocked { .. } => Ok(ExitCode::SUCCESS),
        ReportBlockedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportBlockedOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a started dispatch blocks",
        ),
    }
}

/// `conductor dispatch report-unblocked`: the command `conductor.dispatch.ReportUnblocked`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_unblocked(state: Option<&Path>, args: ReportUnblockedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-unblocked";
    flags_only(args.input_json, "dispatch report-unblocked --input-json")?;
    let input = ReportUnblocked {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
    };
    match run(state, COMMAND, |generated| {
        generated.report_unblocked(input)
    })? {
        ReportUnblockedOutcome::Unblocked { .. } => Ok(ExitCode::SUCCESS),
        ReportUnblockedOutcome::NoSuchDispatch { error } => {
            no_dispatch(COMMAND, &error.dispatch_id)
        }
        ReportUnblockedOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a blocked dispatch is unblocked",
        ),
    }
}

/// `conductor dispatch report-done`: the command `conductor.dispatch.ReportDone`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_done(state: Option<&Path>, args: ReportDoneArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-done";
    flags_only(args.input_json, "dispatch report-done --input-json")?;
    let input = ReportDone {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        evidence: flag(COMMAND, "--evidence", args.evidence)?,
    };
    match run(state, COMMAND, |generated| generated.report_done(input))? {
        ReportDoneOutcome::Done { .. } => Ok(ExitCode::SUCCESS),
        ReportDoneOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportDoneOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a started dispatch is done",
        ),
    }
}

/// `conductor dispatch report-failed`: the command `conductor.dispatch.ReportFailed`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_failed(state: Option<&Path>, args: ReportFailedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-failed";
    flags_only(args.input_json, "dispatch report-failed --input-json")?;
    let input = ReportFailed {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    match run(state, COMMAND, |generated| generated.report_failed(input))? {
        ReportFailedOutcome::Failed { .. } => Ok(ExitCode::SUCCESS),
        ReportFailedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportFailedOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a started or blocked dispatch fails",
        ),
    }
}

/// `conductor dispatch cancel-dispatch`: the command `conductor.dispatch.CancelDispatch`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn cancel_dispatch(state: Option<&Path>, args: CancelDispatchArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch cancel-dispatch";
    flags_only(args.input_json, "dispatch cancel-dispatch --input-json")?;
    let input = CancelDispatch {
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    match run(state, COMMAND, |generated| generated.cancel_dispatch(input))? {
        CancelDispatchOutcome::Cancelled { .. } => Ok(ExitCode::SUCCESS),
        CancelDispatchOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        CancelDispatchOutcome::WrongState { error } => {
            wrong_state(COMMAND, &error.dispatch_id, "the dispatch already ended")
        }
    }
}

/// `conductor dispatch dispatches`: the view `conductor.dispatch.Dispatches`, served by the
/// generated query over the existing store under the state directory and rendered as `--format`
/// asks, in the order the dispatches were first stored. `--format jsonl` is the export of the
/// dispatch records; two runs over one store give the same bytes. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn dispatches(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 8] = [
        "dispatch_id",
        "state",
        "repository",
        "goal_id",
        "brief",
        "sent_at",
        "priority",
        "waits_on",
    ];

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .dispatches()
        .map_err(|unmet| anyhow!("dispatch dispatches: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.dispatch_id.0),
                Value::from(state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(row.goal_id.0),
                Value::from(row.brief),
                Value::from(row.sent_at.0),
                Value::from(row.priority),
                Value::from(row.waits_on.map(|waits_on| waits_on.0)),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "dispatches",
        &view.render(&COLUMNS, &rows),
    )
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn state_name(state: DispatchState) -> &'static str {
    match state {
        DispatchState::Blocked => "Blocked",
        DispatchState::Cancelled => "Cancelled",
        DispatchState::Done => "Done",
        DispatchState::Failed => "Failed",
        DispatchState::Sent => "Sent",
        DispatchState::Started => "Started",
    }
}

/// The highest number `store` holds under `prefix`, `DSP-<day>-` ([`highest`]). Every dispatch is
/// read: one the store holds but this build cannot read might carry the highest number, so it
/// fails the allocation rather than being passed over.
fn stored_highest(command: &str, store: &Store, prefix: &str) -> Result<u128> {
    let held = DispatchStorage::list(store);
    kept(command, store)?;
    Ok(highest(
        prefix,
        held.iter()
            .map(|dispatch| dispatch.data.dispatch_id.0.as_str()),
    ))
}

/// The refusal `DispatchExists`: an id given by hand that a dispatch already has.
fn exists(command: &str, dispatch: &DispatchId) -> Result<ExitCode> {
    bail!(
        "{command}: --dispatch-id {}: a dispatch already has this id; nothing was sent",
        dispatch.0
    )
}

/// Opens the store, runs one generated behaviour on it, and answers its outcome once the store has
/// kept every read and write the behaviour made.
fn run<O>(
    state: Option<&Path>,
    command: &str,
    behaviour: impl FnOnce(&mut Generated<Store>) -> Result<O, UnmetObligation>,
) -> Result<O> {
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = behaviour(&mut generated).map_err(|unmet| anyhow!("{command}: {unmet}"))?;
    kept(command, &generated.ports)?;
    Ok(outcome)
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

/// `--dispatch-id`, which the command line has already checked starts with `DSP-`.
fn dispatch_id(command: &str, value: Option<String>) -> Result<DispatchId> {
    Ok(DispatchId(flag(command, "--dispatch-id", value)?))
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

fn no_dispatch(command: &str, dispatch: &DispatchId) -> Result<ExitCode> {
    refused(
        command,
        "DispatchNotFound",
        &format!("no dispatch has the id {}", dispatch.0),
    )
}

fn wrong_state(command: &str, dispatch: &DispatchId, rule: &str) -> Result<ExitCode> {
    refused(
        command,
        "DispatchStateConflict",
        &format!("dispatch {} did not move: {rule}", dispatch.0),
    )
}

/// Writes `line` to standard output.
fn print_line(command: &str, line: &str) -> Result<ExitCode> {
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}
