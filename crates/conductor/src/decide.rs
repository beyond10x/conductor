//! `conductor decision`: decision requests and the decisions that answer them (design § 5).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else the instance's `state` when a config file names it, else `state/`;
//! [`crate::state::open`]), runs the generated behaviour, and checks that the store kept what the
//! behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0 and prints nothing, except the two that record a decision, which print
//! its id on one line. A refused command exits 1 with one line on standard error: the command, the
//! error the specification declares, and what it means. A flag left out is named on standard error
//! (exit 1) before any store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! **Ids.** A request's id comes from the controller's message and is never allocated; one a
//! request already has is refused (`RequestExists`), naming it, and nothing is written. Given no
//! `--decision-id`, `record-conductor-decision` and `record-operator-decision` allocate
//! `DEC-YYYYMMDD-NN`: the UTC date of `--decided-at`, and one more than the highest number the
//! store holds for that date, at least two digits. A `--decided-at` whose UTC year is outside 1 to
//! 9999 is refused. A decision id given by hand is in that form ([`allocator_form`]), or it is
//! refused naming the form, before any store is opened; one a decision already has is refused
//! (`DecisionExists`), naming it, and nothing is written. When another process records the
//! allocated id first, the id is allocated again, while the date's highest number moves and at
//! most [`ATTEMPTS`] times in all. Either refusal is one line, `<command>: <request|decision>
//! "<id>" already exists; nothing was written`. `record-conductor-decision` asks the
//! specification whether it refuses the class before it allocates an id, so a class O or H
//! refusal names no allocated id.
//!
//! A view opens only an existing store ([`crate::state::open_existing`]) and creates nothing. It
//! lists every row it can read, in the order the rows were first stored, and answers through
//! [`crate::store::show`]: a record it cannot read is named on standard error and exits 1, after
//! the other rows. `decision show` renders one row of the `decisions` view the same way. No
//! handler reads or writes `decisions/*.jsonl`.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage, Generated};
use conductor_model::decision::obligations::{
    AnswerEscalatedRequestBehavior, AnswerRequestBehavior, DecisionsQuery, EscalateRequestBehavior,
    HandsTodoQuery, RaiseDecisionRequestBehavior, RecordConductorDecisionBehavior,
    RecordOperatorDecisionBehavior, RequestsQuery, ReverseConductorDecisionBehavior,
    ReverseOperatorDecisionBehavior, WithdrawRequestBehavior,
};
use conductor_model::decision::{
    AnswerEscalatedRequest, AnswerEscalatedRequestOutcome, AnswerRequest, AnswerRequestOutcome,
    Decider, DecisionClass, DecisionId, DecisionRequestSnapshot, DecisionRequestState,
    DecisionSnapshot, DecisionState, EscalateRequest, EscalateRequestOutcome, RaiseDecisionRequest,
    RaiseDecisionRequestOutcome, RecordConductorDecision, RecordConductorDecisionOutcome,
    RecordOperatorDecision, RecordOperatorDecisionOutcome, RequestId, ReverseConductorDecision,
    ReverseConductorDecisionOutcome, ReverseOperatorDecision, ReverseOperatorDecisionOutcome,
    ShowDecision, WithdrawRequest, WithdrawRequestOutcome,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, Month, OffsetDateTime, UtcOffset};

use crate::cli::{
    AnswerEscalatedRequestArgs, AnswerRequestArgs, EscalateRequestArgs, Format, JsonInput,
    RaiseDecisionRequestArgs, RecordConductorDecisionArgs, RecordOperatorDecisionArgs,
    ReverseConductorDecisionArgs, ReverseOperatorDecisionArgs, ShowDecisionArgs, ViewArgs,
    WithdrawRequestArgs,
};
use crate::not_implemented;
use crate::store::{Store, StoreError};

/// How many ids a record or a send allocates at most, one per attempt. An attempt is lost only to
/// another writer's record, and each loss is retried while the date's highest number moves.
pub const ATTEMPTS: usize = 64;

/// The fields of the `Requests` and `HandsTodo` views, in the specification's order
/// (`spec/domains/decision.yaml`).
const REQUEST_COLUMNS: [&str; 11] = [
    "escalation_class",
    "escalation_reason",
    "request_id",
    "state",
    "repository",
    "decision_id",
    "question",
    "options",
    "recommendation",
    "raised_at",
    "blocker",
];

/// The fields of the `Decisions` view, in the specification's order.
const DECISION_COLUMNS: [&str; 10] = [
    "decision_id",
    "state",
    "decision_class",
    "decided_by",
    "choice",
    "question",
    "options",
    "reason",
    "evidence",
    "decided_at",
];

/// `conductor decision raise-decision-request`: the command
/// `conductor.decision.RaiseDecisionRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the request, or it already holds a request with this id.
pub fn raise_decision_request(
    state: Option<&Path>,
    args: RaiseDecisionRequestArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision raise-decision-request";
    flags_only(
        args.input_json,
        "decision raise-decision-request --input-json",
    )?;
    let input = RaiseDecisionRequest {
        request_id: RequestId(flag(COMMAND, "--request-id", args.request_id)?),
        repository: RepositoryName(flag(COMMAND, "--repository", args.repository)?),
        question: flag(COMMAND, "--question", args.question)?,
        options: flag(COMMAND, "--options", args.options)?,
        recommendation: flag(COMMAND, "--recommendation", args.recommendation)?,
        raised_at: Timestamp(flag(COMMAND, "--raised-at", args.raised_at)?),
        blocker: args.blocker,
    };
    let request = input.request_id.clone();
    let mut generated = record(COMMAND, state)?;
    // The behaviour reads the id before it writes, so the write expects no stream, and a request
    // another process raises in between is refused by the store rather than replaced.
    let outcome = generated
        .raise_decision_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    match generated.ports.store.check() {
        Ok(()) => {}
        Err(StoreError::Exists { identity, .. }) if identity == request.0 => {
            return request_held(COMMAND, &request);
        }
        Err(failure) => {
            return Err(anyhow::Error::new(failure))
                .with_context(|| format!("{COMMAND}: the store did not keep it"));
        }
    }
    match outcome {
        RaiseDecisionRequestOutcome::Raised { .. } => Ok(ExitCode::SUCCESS),
        RaiseDecisionRequestOutcome::AlreadyRaised { error } => {
            request_held(COMMAND, &error.request_id)
        }
    }
}

/// `conductor decision record-conductor-decision`: the command
/// `conductor.decision.RecordConductorDecision`. Prints the decision's id.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the decision, the given id is already held, no id could be
/// allocated, or the specification refuses it: `ReservedForOperator`.
pub fn record_conductor_decision(
    state: Option<&Path>,
    args: RecordConductorDecisionArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision record-conductor-decision";
    flags_only(
        args.input_json,
        "decision record-conductor-decision --input-json",
    )?;
    let fields = Fields::read(
        COMMAND,
        Flags {
            decision_id: args.decision_id,
            decision_class: args.decision_class,
            question: args.question,
            options: args.options,
            choice: args.choice,
            reason: args.reason,
            evidence: args.evidence,
            decided_at: args.decided_at,
        },
    )?;
    let input = |decision_id| RecordConductorDecision {
        decision_id,
        decision_class: fields.decision_class,
        question: fields.question.clone(),
        options: fields.options.clone(),
        choice: fields.choice.clone(),
        reason: fields.reason.clone(),
        evidence: fields.evidence.clone(),
        decided_at: fields.decided_at.clone(),
    };
    // The specification refuses class O and H on the input alone (`reserved`, before it reads a
    // record), so it is asked over a store that holds and keeps nothing before an id is allocated:
    // the refusal then names no id that the next decision is given.
    if fields.decision_id.is_none()
        && let RecordConductorDecisionOutcome::Reserved { .. } = Generated::new(Unstored)
            .record_conductor_decision(input(DecisionId(String::new())))
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?
    {
        return refused(
            COMMAND,
            "ReservedForOperator",
            &format!(
                "a class {} decision is one only the operator decides (design § 5); no id was \
                 allocated and nothing was recorded",
                class_name(fields.decision_class)
            ),
        );
    }
    record_decision(COMMAND, state, &fields, |generated, decision_id| {
        let outcome = generated
            .record_conductor_decision(input(decision_id))
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        Ok(match outcome {
            RecordConductorDecisionOutcome::Recorded { .. } => Answer::Recorded,
            RecordConductorDecisionOutcome::AlreadyRecorded { .. } => Answer::Held,
            RecordConductorDecisionOutcome::Reserved { error } => Answer::Refused(Refusal {
                error: "ReservedForOperator",
                detail: format!(
                    "decision {:?} is class {}, which only the operator decides (design § 5); \
                     nothing was recorded",
                    error.decision_id.0,
                    class_name(fields.decision_class)
                ),
            }),
        })
    })
}

/// `conductor decision record-operator-decision`: the command
/// `conductor.decision.RecordOperatorDecision`. Prints the decision's id.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the decision, the given id is already held, or no id could be
/// allocated.
pub fn record_operator_decision(
    state: Option<&Path>,
    args: RecordOperatorDecisionArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision record-operator-decision";
    flags_only(
        args.input_json,
        "decision record-operator-decision --input-json",
    )?;
    let fields = Fields::read(
        COMMAND,
        Flags {
            decision_id: args.decision_id,
            decision_class: args.decision_class,
            question: args.question,
            options: args.options,
            choice: args.choice,
            reason: args.reason,
            evidence: args.evidence,
            decided_at: args.decided_at,
        },
    )?;
    record_decision(COMMAND, state, &fields, |generated, decision_id| {
        let outcome = generated
            .record_operator_decision(RecordOperatorDecision {
                decision_id,
                decision_class: fields.decision_class,
                question: fields.question.clone(),
                options: fields.options.clone(),
                choice: fields.choice.clone(),
                reason: fields.reason.clone(),
                evidence: fields.evidence.clone(),
                decided_at: fields.decided_at.clone(),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        Ok(match outcome {
            RecordOperatorDecisionOutcome::Recorded { .. } => Answer::Recorded,
            RecordOperatorDecisionOutcome::AlreadyRecorded { .. } => Answer::Held,
        })
    })
}

/// `conductor decision answer-request`: the command `conductor.decision.AnswerRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DecisionNotFound`,
/// `DecisionReversed`, `RequestStateConflict`.
pub fn answer_request(state: Option<&Path>, args: AnswerRequestArgs) -> Result<ExitCode> {
    const COMMAND: &str = "decision answer-request";
    flags_only(args.input_json, "decision answer-request --input-json")?;
    let input = AnswerRequest {
        request_id: RequestId(flag(COMMAND, "--request-id", args.request_id)?),
        decision_id: DecisionId(flag(COMMAND, "--decision-id", args.decision_id)?),
    };
    let request = input.request_id.clone();
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .answer_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        AnswerRequestOutcome::Answered { .. } => Ok(ExitCode::SUCCESS),
        AnswerRequestOutcome::NoSuchDecision { error } => refused(
            COMMAND,
            "DecisionNotFound",
            &no_decision(&error.decision_id),
        ),
        AnswerRequestOutcome::Reversed { error } => {
            refused(COMMAND, "DecisionReversed", &reversed(&error.decision_id))
        }
        AnswerRequestOutcome::WrongState { error } => refused(
            COMMAND,
            "RequestStateConflict",
            &format!(
                "request {:?} is not open, so nothing moved: an escalated request is the \
                 operator's (answer-escalated-request), and an answered or withdrawn one is closed",
                error.request_id.0
            ),
        ),
        AnswerRequestOutcome::WrongStateUnknownInstance => {
            refused(COMMAND, "RequestStateConflict", &no_request(&request))
        }
    }
}

/// `conductor decision answer-escalated-request`: the command
/// `conductor.decision.AnswerEscalatedRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DecisionNotFound`,
/// `DecisionReversed`, `RequestStateConflict`.
pub fn answer_escalated_request(
    state: Option<&Path>,
    args: AnswerEscalatedRequestArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision answer-escalated-request";
    flags_only(
        args.input_json,
        "decision answer-escalated-request --input-json",
    )?;
    let input = AnswerEscalatedRequest {
        request_id: RequestId(flag(COMMAND, "--request-id", args.request_id)?),
        decision_id: DecisionId(flag(COMMAND, "--decision-id", args.decision_id)?),
    };
    let request = input.request_id.clone();
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .answer_escalated_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        AnswerEscalatedRequestOutcome::Answered { .. } => Ok(ExitCode::SUCCESS),
        AnswerEscalatedRequestOutcome::NoSuchDecision { error } => refused(
            COMMAND,
            "DecisionNotFound",
            &no_decision(&error.decision_id),
        ),
        AnswerEscalatedRequestOutcome::Reversed { error } => {
            refused(COMMAND, "DecisionReversed", &reversed(&error.decision_id))
        }
        AnswerEscalatedRequestOutcome::WrongState { error } => refused(
            COMMAND,
            "RequestStateConflict",
            &format!(
                "request {:?} is not escalated, so nothing moved: only an escalated request is \
                 answered here",
                error.request_id.0
            ),
        ),
        AnswerEscalatedRequestOutcome::WrongStateUnknownInstance => {
            refused(COMMAND, "RequestStateConflict", &no_request(&request))
        }
    }
}

/// `conductor decision escalate-request`: the command `conductor.decision.EscalateRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `NotEscalatable`,
/// `RequestNotFound`, `RequestStateConflict`.
pub fn escalate_request(state: Option<&Path>, args: EscalateRequestArgs) -> Result<ExitCode> {
    const COMMAND: &str = "decision escalate-request";
    flags_only(args.input_json, "decision escalate-request --input-json")?;
    let request_id = RequestId(flag(COMMAND, "--request-id", args.request_id)?);
    let escalation_class = class(
        COMMAND,
        "--escalation-class",
        flag(COMMAND, "--escalation-class", args.escalation_class)?,
    )?;
    let input = EscalateRequest {
        request_id,
        escalation_class,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .escalate_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        EscalateRequestOutcome::Escalated { .. } => Ok(ExitCode::SUCCESS),
        EscalateRequestOutcome::ClassC { error } => refused(
            COMMAND,
            "NotEscalatable",
            &format!(
                "class C is decided by conductor, never escalated; request {:?} is unchanged",
                error.request_id.0
            ),
        ),
        EscalateRequestOutcome::NoSuchRequest { error } => {
            refused(COMMAND, "RequestNotFound", &no_request(&error.request_id))
        }
        EscalateRequestOutcome::WrongState { error } => refused(
            COMMAND,
            "RequestStateConflict",
            &format!(
                "request {:?} is not open, and only an open request is escalated",
                error.request_id.0
            ),
        ),
    }
}

/// `conductor decision withdraw-request`: the command `conductor.decision.WithdrawRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `RequestNotFound`,
/// `RequestStateConflict`.
pub fn withdraw_request(state: Option<&Path>, args: WithdrawRequestArgs) -> Result<ExitCode> {
    const COMMAND: &str = "decision withdraw-request";
    flags_only(args.input_json, "decision withdraw-request --input-json")?;
    let input = WithdrawRequest {
        request_id: RequestId(flag(COMMAND, "--request-id", args.request_id)?),
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .withdraw_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        WithdrawRequestOutcome::Withdrawn { .. } => Ok(ExitCode::SUCCESS),
        WithdrawRequestOutcome::NoSuchRequest { error } => {
            refused(COMMAND, "RequestNotFound", &no_request(&error.request_id))
        }
        WithdrawRequestOutcome::WrongState { error } => refused(
            COMMAND,
            "RequestStateConflict",
            &format!(
                "request {:?} is already answered or withdrawn",
                error.request_id.0
            ),
        ),
    }
}

/// `conductor decision reverse-conductor-decision`: the command
/// `conductor.decision.ReverseConductorDecision`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it:
/// `ReversalReservedForOperator`, `DecisionNotFound`, `DecisionStateConflict`.
pub fn reverse_conductor_decision(
    state: Option<&Path>,
    args: ReverseConductorDecisionArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision reverse-conductor-decision";
    flags_only(
        args.input_json,
        "decision reverse-conductor-decision --input-json",
    )?;
    let input = ReverseConductorDecision {
        decision_id: DecisionId(flag(COMMAND, "--decision-id", args.decision_id)?),
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .reverse_conductor_decision(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        ReverseConductorDecisionOutcome::Reversed { .. } => Ok(ExitCode::SUCCESS),
        ReverseConductorDecisionOutcome::OperatorDecided { error } => refused(
            COMMAND,
            "ReversalReservedForOperator",
            &format!(
                "decision {:?} was recorded by the operator, so only he reverses it \
                 (reverse-operator-decision); it stays in force",
                error.decision_id.0
            ),
        ),
        ReverseConductorDecisionOutcome::NoSuchDecision { error } => refused(
            COMMAND,
            "DecisionNotFound",
            &no_decision(&error.decision_id),
        ),
        ReverseConductorDecisionOutcome::WrongState { error } => refused(
            COMMAND,
            "DecisionStateConflict",
            &already_reversed(&error.decision_id),
        ),
    }
}

/// `conductor decision reverse-operator-decision`: the command
/// `conductor.decision.ReverseOperatorDecision`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DecisionNotFound`,
/// `DecisionStateConflict`.
pub fn reverse_operator_decision(
    state: Option<&Path>,
    args: ReverseOperatorDecisionArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision reverse-operator-decision";
    flags_only(
        args.input_json,
        "decision reverse-operator-decision --input-json",
    )?;
    let input = ReverseOperatorDecision {
        decision_id: DecisionId(flag(COMMAND, "--decision-id", args.decision_id)?),
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .reverse_operator_decision(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        ReverseOperatorDecisionOutcome::Reversed { .. } => Ok(ExitCode::SUCCESS),
        ReverseOperatorDecisionOutcome::NoSuchDecision { error } => refused(
            COMMAND,
            "DecisionNotFound",
            &no_decision(&error.decision_id),
        ),
        ReverseOperatorDecisionOutcome::WrongState { error } => refused(
            COMMAND,
            "DecisionStateConflict",
            &already_reversed(&error.decision_id),
        ),
    }
}

/// `conductor decision requests`: the view `conductor.decision.Requests`, served by the generated
/// query over the existing store and rendered as `--format` asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn requests(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const VIEW: &str = "decision requests";
    let generated = existing(VIEW, state)?;
    let rows: Vec<Vec<Value>> = generated
        .requests()
        .map_err(|unmet| anyhow!("{VIEW}: {unmet}"))?
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.escalation_class.map(class_name)),
                Value::from(row.escalation_reason),
                Value::from(row.request_id.0),
                Value::from(request_state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(row.decision_id.map(|id| id.0)),
                Value::from(row.question),
                Value::from(row.options),
                Value::from(row.recommendation),
                Value::from(row.raised_at.0),
                Value::from(row.blocker),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports.store,
        "decision requests",
        &view.render(&REQUEST_COLUMNS, &rows),
    )
}

/// `conductor decision hands-todo`: the view `conductor.decision.HandsTodo`, every request
/// escalated as class H and waiting for the operator's hands, served by the generated query over
/// the existing store and rendered as `--format` asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn hands_todo(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const VIEW: &str = "decision hands-todo";
    let generated = existing(VIEW, state)?;
    let rows: Vec<Vec<Value>> = generated
        .hands_todo()
        .map_err(|unmet| anyhow!("{VIEW}: {unmet}"))?
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.escalation_class.map(class_name)),
                Value::from(row.escalation_reason),
                Value::from(row.request_id.0),
                Value::from(request_state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(row.decision_id.map(|id| id.0)),
                Value::from(row.question),
                Value::from(row.options),
                Value::from(row.recommendation),
                Value::from(row.raised_at.0),
                Value::from(row.blocker),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports.store,
        "class-H to-do",
        &view.render(&REQUEST_COLUMNS, &rows),
    )
}

/// `conductor decision decisions`: the view `conductor.decision.Decisions`, served by the generated
/// query over the existing store and rendered as `--format` asks. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn decisions(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const VIEW: &str = "decision decisions";
    let generated = existing(VIEW, state)?;
    let rows: Vec<Vec<Value>> = generated
        .decisions()
        .map_err(|unmet| anyhow!("{VIEW}: {unmet}"))?
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.decision_id.0),
                Value::from(decision_state_name(row.state)),
                Value::from(class_name(row.decision_class)),
                Value::from(decider_name(row.decided_by)),
                Value::from(row.choice),
                Value::from(row.question),
                Value::from(row.options),
                Value::from(row.reason),
                Value::from(row.evidence),
                Value::from(row.decided_at.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports.store,
        "decisions",
        &view.render(&DECISION_COLUMNS, &rows),
    )
}

/// The fields `decision show` renders, in its order: a row of the `Decisions` view.
const SHOWN: [&str; 10] = [
    "decision_id",
    "state",
    "decision_class",
    "decided_by",
    "decided_at",
    "question",
    "options",
    "choice",
    "reason",
    "evidence",
];

/// `conductor decision show`: the binding's `decision-show` callable over
/// `conductor.decision.ShowDecision`. Renders the decision `--decision-id` names, its row of the
/// `Decisions` view: one `field: value` line each in [`SHOWN`]'s order, a line break inside a value
/// written as a space (`--format text`, the default), or one JSON object with those keys in that
/// order (`--format json`). A view, so it opens only an existing store and creates nothing.
///
/// # Errors
///
/// `--decision-id` is missing, `--format` is neither `text` nor `json`, there is no store or it
/// does not open, a read fails for a reason other than a record this build cannot read, or no
/// decision has the id: one line naming it. A record this build cannot read is named on standard
/// error, with exit 1, as every view names one.
pub fn show(state: Option<&Path>, args: ShowDecisionArgs) -> Result<ExitCode> {
    const COMMAND: &str = "decision show";
    let input = ShowDecision {
        decision_id: flag(COMMAND, "--decision-id", args.decision_id)?,
        format: args.format,
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is neither text nor json"),
    };
    let generated = existing(COMMAND, state)?;
    let row = generated
        .decisions()
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?
        .into_iter()
        .find(|row| row.decision_id.0 == input.decision_id);
    let Some(row) = row else {
        // The decision asked for may be a record this build cannot read: name each such record,
        // as a view does, before answering that no decision has the id.
        if crate::store::show(&generated.ports.store, "decisions", "")? != ExitCode::SUCCESS {
            return Ok(ExitCode::FAILURE);
        }
        bail!("{COMMAND}: no decision has the id {:?}", input.decision_id);
    };
    let values = [
        Value::from(row.decision_id.0),
        Value::from(decision_state_name(row.state)),
        Value::from(class_name(row.decision_class)),
        Value::from(decider_name(row.decided_by)),
        Value::from(row.decided_at.0),
        Value::from(row.question),
        Value::from(row.options),
        Value::from(row.choice),
        Value::from(row.reason),
        Value::from(row.evidence),
    ];
    let rendered = if json {
        ViewArgs {
            format: Format::Jsonl,
        }
        .render(&SHOWN, &[values.to_vec()])
    } else {
        SHOWN
            .iter()
            .zip(&values)
            .map(|(field, value)| {
                let text = value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned);
                format!(
                    "{field}: {}\n",
                    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
                )
            })
            .collect()
    };
    crate::store::show(&generated.ports.store, "decisions", &rendered)
}

/// The ports the decision commands and views run over: the store.
struct Ports {
    store: Store,
}

/// Opens the store a command records into, creating it when it is not there.
fn record(command: &str, state: Option<&Path>) -> Result<Generated<Ports>> {
    Ok(Generated::new(Ports {
        store: crate::state::open(state).with_context(|| command.to_owned())?,
    }))
}

/// Opens the existing store a view reads, creating nothing.
fn existing(view: &str, state: Option<&Path>) -> Result<Generated<Ports>> {
    Ok(Generated::new(Ports {
        store: crate::state::open_existing(state).with_context(|| view.to_owned())?,
    }))
}

impl DecisionStorage for Ports {
    fn get(&self, identity: &DecisionId) -> Option<DecisionSnapshot> {
        DecisionStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: DecisionSnapshot) {
        DecisionStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &DecisionId) {
        DecisionStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<DecisionSnapshot> {
        DecisionStorage::list(&self.store)
    }
}

impl DecisionRequestStorage for Ports {
    fn get(&self, identity: &RequestId) -> Option<DecisionRequestSnapshot> {
        DecisionRequestStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: DecisionRequestSnapshot) {
        DecisionRequestStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &RequestId) {
        DecisionRequestStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<DecisionRequestSnapshot> {
        DecisionRequestStorage::list(&self.store)
    }
}

/// A decision store that holds nothing and keeps nothing: the specification's refusals that read
/// only the input are asked over it before an id is allocated.
struct Unstored;

impl DecisionStorage for Unstored {
    fn get(&self, _: &DecisionId) -> Option<DecisionSnapshot> {
        None
    }

    fn put(&mut self, _: DecisionSnapshot) {}

    fn delete(&mut self, _: &DecisionId) {}

    fn list(&self) -> Vec<DecisionSnapshot> {
        Vec::new()
    }
}

/// The flags both record commands take, as given.
struct Flags {
    decision_id: Option<String>,
    decision_class: Option<String>,
    question: Option<String>,
    options: Option<String>,
    choice: Option<String>,
    reason: Option<String>,
    evidence: Option<String>,
    decided_at: Option<String>,
}

/// The input fields both record commands take, read from their flags.
struct Fields {
    /// `--decision-id`, when it was given.
    decision_id: Option<DecisionId>,
    decision_class: DecisionClass,
    question: String,
    options: String,
    choice: String,
    reason: String,
    evidence: String,
    decided_at: Timestamp,
}

impl Fields {
    /// Every flag but `--decision-id` is required; the first one missing is named, in the order
    /// the specification lists the input fields. A `--decision-id` given is then held to the
    /// allocator's form.
    fn read(command: &str, flags: Flags) -> Result<Self> {
        let decision_class = flag(command, "--decision-class", flags.decision_class)?;
        let fields = Self {
            decision_id: flags.decision_id.map(DecisionId),
            decision_class: class(command, "--decision-class", decision_class)?,
            question: flag(command, "--question", flags.question)?,
            options: flag(command, "--options", flags.options)?,
            choice: flag(command, "--choice", flags.choice)?,
            reason: flag(command, "--reason", flags.reason)?,
            evidence: flag(command, "--evidence", flags.evidence)?,
            decided_at: Timestamp(flag(command, "--decided-at", flags.decided_at)?),
        };
        if let Some(given) = &fields.decision_id {
            in_allocator_form(command, "--decision-id", DECISION_PREFIX, &given.0)?;
        }
        Ok(fields)
    }
}

/// A refusal the specification declares: its error, and what it means.
struct Refusal {
    error: &'static str,
    detail: String,
}

/// What a record command's behaviour answered for one decision id.
enum Answer {
    /// The decision is recorded under the id.
    Recorded,
    /// A decision already has the id (`DecisionExists`), and nothing was recorded.
    Held,
    /// Another refusal the specification declares, and nothing was recorded.
    Refused(Refusal),
}

/// Records one decision through `run`, which runs the generated behaviour for the id it is given
/// and answers what it decided. The id is `--decision-id`, refused when a decision already has it
/// (`DecisionExists`), or else the [`next_id`] after the [`highest`] number stored for the UTC
/// date of `--decided-at`, allocated again when another process records it first: while that
/// highest number moves, and in at most [`ATTEMPTS`] attempts. Prints the id of the recorded
/// decision.
///
/// The behaviour reads the id before it writes, so the write expects no stream: a decision
/// another process records in between is refused by the store ([`StoreError::Exists`]) rather
/// than replaced, and is answered as one the behaviour found.
fn record_decision(
    command: &str,
    state: Option<&Path>,
    fields: &Fields,
    mut run: impl FnMut(&mut Generated<Ports>, DecisionId) -> Result<Answer>,
) -> Result<ExitCode> {
    let date = utc_day(&fields.decided_at.0)
        .with_context(|| format!("{command}: --decided-at {:?}", fields.decided_at.0))?;
    let prefix = format!("{DECISION_PREFIX}{date}-");
    let mut before = None;
    for _ in 0..ATTEMPTS {
        // A fresh handle per attempt: a handle refuses every write after its first failure.
        let mut generated = record(command, state)?;
        let decision_id = match &fields.decision_id {
            Some(given) => given.clone(),
            None => {
                let highest = stored_highest(command, &generated.ports, &prefix)?;
                if before.replace(highest) == Some(highest) {
                    bail!(
                        "{command}: the id allocated above {prefix}{highest:02} was taken, yet \
                         the highest number stored for {date} did not move; nothing was written"
                    );
                }
                DecisionId(next_id(command, &prefix, highest)?)
            }
        };
        let answer = run(&mut generated, decision_id.clone())?;
        let answer = match generated.ports.store.check() {
            Ok(()) => answer,
            Err(StoreError::Exists { identity, .. }) if identity == decision_id.0 => Answer::Held,
            Err(failure) => {
                return Err(anyhow::Error::new(failure))
                    .with_context(|| format!("{command}: the store did not keep it"));
            }
        };
        match answer {
            Answer::Recorded => return print_line(command, &decision_id.0),
            Answer::Refused(Refusal { error, detail }) => return refused(command, error, &detail),
            Answer::Held if fields.decision_id.is_some() => {
                return already_held(command, &decision_id);
            }
            // Another process recorded the allocated id first: allocate again.
            Answer::Held => {}
        }
    }
    bail!(
        "{command}: another process recorded each of the {ATTEMPTS} decision ids allocated for \
         {date}; nothing was written"
    )
}

/// The highest number the store holds under `prefix` ([`highest`]).
///
/// Every decision is read: one the store holds but this build cannot read might carry the highest
/// number, so it fails the allocation rather than being passed over.
fn stored_highest(command: &str, store: &Ports, prefix: &str) -> Result<u128> {
    let held = DecisionStorage::list(store);
    store
        .store
        .check()
        .with_context(|| format!("{command}: read the decisions to allocate an id"))?;
    Ok(highest(
        prefix,
        held.iter()
            .map(|decision| decision.data.decision_id.0.as_str()),
    ))
}

/// The prefix `conductor.decision.DecisionId` declares (`spec/domains/decision.yaml`).
const DECISION_PREFIX: &str = "DEC-";

/// The highest number among `ids` under `prefix` (`DEC-<date>-`, `DSP-<date>-`), 0 when there is
/// none. An id whose suffix is not all digits is not counted, and neither is one too large for a
/// `u128`, which no id allocated here can equal.
pub(crate) fn highest<'a>(prefix: &str, ids: impl IntoIterator<Item = &'a str>) -> u128 {
    ids.into_iter()
        .filter_map(|id| id.strip_prefix(prefix))
        .filter(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .filter_map(|number| number.parse::<u128>().ok())
        .max()
        .unwrap_or(0)
}

/// The id after `highest` under `prefix`: one more, written with at least two digits.
///
/// # Errors
///
/// No number is left after `highest`.
pub(crate) fn next_id(command: &str, prefix: &str, highest: u128) -> Result<String> {
    let next = highest
        .checked_add(1)
        .ok_or_else(|| anyhow!("{command}: no number is left after {prefix}{highest}"))?;
    Ok(format!("{prefix}{next:02}"))
}

/// Whether `id` is in the allocator's form, `<prefix>YYYYMMDD-NN`: a date of the years 1 to 9999,
/// then a number of two digits, or one of 100 or more without a leading zero. Every number has one
/// such spelling, so two ids in this form carry one number of one date only when they are equal.
pub(crate) fn allocator_form(prefix: &str, id: &str) -> bool {
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let Some((date, number)) = id
        .strip_prefix(prefix)
        .and_then(|rest| rest.split_once('-'))
    else {
        return false;
    };
    let date = (date.len() == 8 && digits(date))
        .then(|| {
            let year = date[..4].parse::<i32>().ok()?;
            let month = Month::try_from(date[4..6].parse::<u8>().ok()?).ok()?;
            let day = date[6..].parse::<u8>().ok()?;
            (year >= 1)
                .then(|| Date::from_calendar_date(year, month, day).ok())
                .flatten()
        })
        .flatten();
    date.is_some()
        && digits(number)
        && (number.len() == 2 || (number.len() > 2 && !number.starts_with('0')))
}

/// Refuses a `flag` value `id`, given by hand, that is not in the [`allocator_form`] under
/// `prefix`.
///
/// # Errors
///
/// `id` is in another form: one line naming it and the form.
pub(crate) fn in_allocator_form(command: &str, flag: &str, prefix: &str, id: &str) -> Result<()> {
    if allocator_form(prefix, id) {
        return Ok(());
    }
    bail!(
        "{command}: {flag} {id:?} is not in the form {prefix}YYYYMMDD-NN, a date and then a number \
         of two digits or one of 100 or more without a leading zero; nothing was written"
    )
}

/// The UTC date of the RFC 3339 instant `at`, as `YYYYMMDD`.
///
/// # Errors
///
/// `at` is not RFC 3339, or its UTC date falls outside the years 1 to 9999.
pub(crate) fn utc_day(at: &str) -> Result<String> {
    let date = OffsetDateTime::parse(at, &Rfc3339)
        .context("not an RFC 3339 instant")?
        .checked_to_offset(UtcOffset::UTC)
        .ok_or_else(|| anyhow!("its UTC date falls outside the years 1 to 9999"))?
        .date();
    let year = date.year();
    if !(1..=9999).contains(&year) {
        bail!("its UTC date falls in the year {year}, outside 1 to 9999");
    }
    Ok(format!(
        "{year:04}{:02}{:02}",
        u8::from(date.month()),
        date.day()
    ))
}

/// The refusal `DecisionExists`: a decision already has the id given by hand.
fn already_held(command: &str, decision_id: &DecisionId) -> Result<ExitCode> {
    bail!(
        "{command}: decision {:?} already exists; nothing was written",
        decision_id.0
    )
}

/// The refusal `RequestExists`: a request already has the id.
fn request_held(command: &str, request: &RequestId) -> Result<ExitCode> {
    bail!(
        "{command}: request {:?} already exists; nothing was written",
        request.0
    )
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

/// The class the flag `name` gives; its parser has already held it to `C`, `O` or `H`.
fn class(command: &str, name: &str, value: String) -> Result<DecisionClass> {
    [DecisionClass::C, DecisionClass::O, DecisionClass::H]
        .into_iter()
        .find(|class| class_name(*class) == value)
        .ok_or_else(|| anyhow!("{command}: {name} {value:?} is no DecisionClass"))
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

fn no_decision(decision: &DecisionId) -> String {
    format!("no decision has the id {:?}; nothing moved", decision.0)
}

fn reversed(decision: &DecisionId) -> String {
    format!(
        "decision {:?} was reversed, so it answers no request; the request stays where it was",
        decision.0
    )
}

fn already_reversed(decision: &DecisionId) -> String {
    format!("decision {:?} is already reversed", decision.0)
}

fn no_request(request: &RequestId) -> String {
    format!(
        "no decision request has the id {:?}; nothing moved",
        request.0
    )
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn decision_state_name(state: DecisionState) -> &'static str {
    match state {
        DecisionState::InForce => "InForce",
        DecisionState::Reversed => "Reversed",
    }
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn request_state_name(state: DecisionRequestState) -> &'static str {
    match state {
        DecisionRequestState::Answered => "Answered",
        DecisionRequestState::Escalated => "Escalated",
        DecisionRequestState::Open => "Open",
        DecisionRequestState::Withdrawn => "Withdrawn",
    }
}

/// The class's name as the specification spells it. Exhaustive, so a class the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn class_name(class: DecisionClass) -> &'static str {
    match class {
        DecisionClass::C => "C",
        DecisionClass::O => "O",
        DecisionClass::H => "H",
    }
}

/// The decider's name as the specification spells it. Exhaustive, so a decider the specification
/// adds fails to compile here rather than rendering under a guessed name.
fn decider_name(decider: Decider) -> &'static str {
    match decider {
        Decider::Conductor => "Conductor",
        Decider::Operator => "Operator",
    }
}
