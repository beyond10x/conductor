//! `conductor decision`: decision requests and the decisions that answer them (design § 5).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else the instance's `state` when a config file names it, else `state/`;
//! [`crate::state::open`]), runs the generated behaviour, and checks that the store kept what the
//! behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0 and prints nothing, except the three that record a decision, which print
//! its id on one line. A refused command exits 1 with one line on standard error: the command, the
//! error the specification declares, and what it means. A flag left out is named on standard error
//! (exit 1) before any store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! **Ids.** A request's id comes from the controller's message and is never allocated; one a
//! request already has is refused (`RequestExists`), naming it, and nothing is written. Given no
//! `--decision-id`, `record-conductor-decision` and `record-operator-decision` allocate
//! `DEC-YYYYMMDD-NN`: the UTC date of `--decided-at`, and one more than the highest number for
//! that date that the store holds or a line of the instance's hand-written decision files holds
//! ([`written_highest`]), at least two digits. A `--decided-at` whose UTC year is outside 1 to
//! 9999 is refused. A decision id given by hand is in that form ([`allocator_form`]), or it is
//! refused naming the form, before any store is opened; one a decision already has is refused
//! (`DecisionExists`), naming it, and nothing is written. When another process records the
//! allocated id first, the id is allocated again, while the date's highest number moves and at
//! most [`ATTEMPTS`] times in all. Either refusal is one line, `<command>: <request|decision>
//! "<id>" already exists; nothing was written`. `record-conductor-decision` asks the
//! specification whether it refuses the class before it allocates an id, so a class O or H
//! refusal names no allocated id.
//!
//! **Charter.** Given `--repository`, `record-conductor-decision` prints that repository's
//! charter's "Do not touch" and standing-rule lines on standard error before it records
//! ([`print_charter`], `story:decision-reads-charter`); a class O or H refusal prints none.
//!
//! A view opens only an existing store ([`crate::state::open_existing`]) and creates nothing. It
//! lists every row it can read, in the order the rows were first stored, and answers through
//! [`crate::store::show`]: a record it cannot read is named on standard error and exits 1, after
//! the other rows. `decision show` renders one row of the `decisions` view the same way.
//!
//! **Records.** Once the store kept it, every command of this group that records an outcome
//! appends one line to the records' decision log, `decisions/YYYY-MM.jsonl` ([`crate::records`]):
//! `decided` by the two record commands, `requested`, `answered` (both answer commands), both
//! `decided` and `answered` by `answer-escalated-as-operator` (`story:decisions-batch-answer`),
//! `escalated`, `withdrawn`, and `reversed` (both reverse commands), each with the id of the
//! record it moved and its repository as `repo`. A refusal appends nothing.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage, Generated};
use conductor_model::decision::obligations::{
    AnswerEscalatedRequestBehavior, AnswerRequestBehavior, DecisionsQuery, EscalateRequestBehavior,
    HandsTodoQuery, RaiseDecisionRequestBehavior, RecordConductorDecisionBehavior,
    RecordOperatorDecisionBehavior, RecordOperatorDecisionForRequestBehavior, RequestsQuery,
    ReverseConductorDecisionBehavior, ReverseOperatorDecisionBehavior, WithdrawRequestBehavior,
};
use conductor_model::decision::{
    AnswerEscalatedRequest, AnswerEscalatedRequestOutcome, AnswerRequest, AnswerRequestOutcome,
    Decider, DecisionClass, DecisionId, DecisionLogEvent, DecisionLogLine, DecisionRequestSnapshot,
    DecisionRequestState, DecisionSnapshot, DecisionState, EscalateRequest, EscalateRequestOutcome,
    RaiseDecisionRequest, RaiseDecisionRequestOutcome, RecordConductorDecision,
    RecordConductorDecisionOutcome, RecordOperatorDecision, RecordOperatorDecisionForRequest,
    RecordOperatorDecisionForRequestOutcome, RecordOperatorDecisionOutcome, RequestId, RequestKind,
    RequestOption, ReverseConductorDecision, ReverseConductorDecisionOutcome,
    ReverseOperatorDecision, ReverseOperatorDecisionOutcome, ShowDecision, WithdrawRequest,
    WithdrawRequestOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, Month, OffsetDateTime, UtcOffset};

use crate::cli::{
    AnswerEscalatedRequestArgs, AnswerRequestArgs, EscalateRequestArgs, Format, JsonInput,
    RaiseDecisionRequestArgs, RecordConductorDecisionArgs, RecordOperatorDecisionArgs,
    RecordOperatorDecisionForRequestArgs, ReverseConductorDecisionArgs,
    ReverseOperatorDecisionArgs, ShowDecisionArgs, ViewArgs, WithdrawRequestArgs,
};
use crate::not_implemented;
use crate::records;
use crate::store::{Store, StoreError};

/// How many ids a record or a send allocates at most, one per attempt. An attempt is lost only to
/// another writer's record, and each loss is retried while the date's highest number moves.
pub const ATTEMPTS: usize = 64;

/// The fields of the `Requests` view, in the specification's order
/// (`spec/domains/decision.yaml`).
const REQUEST_COLUMNS: [&str; 16] = [
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
    "kind",
    "lettered_options",
    "recommended_letters",
    "default_letters",
    "deadline",
];

/// The columns `decision hands-todo` prints as a table (`text`, `markdown`): the fields of the
/// `HandsTodo` view, then `age`, how long each request has been open in whole days
/// (`story:operator-items-age`). `age` is computed when the table is rendered and is no field of
/// the specification's view, so `json` and `jsonl` print the view's fields alone
/// ([`HANDS_TODO_FIELDS`]).
const HANDS_TODO_COLUMNS: [&str; 14] = [
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
    "dry_run",
    "no_dry_run",
    "age",
];

/// The fields of the `HandsTodo` view, in the specification's order: the `Requests` view's, then
/// each item's dry run or why it has none (`story:hand-item-dry-run`).
const HANDS_TODO_FIELDS: [&str; 13] = [
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
    "dry_run",
    "no_dry_run",
];

/// The fields of the `Decisions` view, in the specification's order.
const DECISION_COLUMNS: [&str; 13] = [
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
    "recorded_by",
    "operator_words",
    "repository",
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
    let mut line = records::decision_line(
        DecisionLogEvent::Requested,
        Some(&input.repository.0),
        &request.0,
    )?;
    line.question = Some(input.question.clone());
    line.options = Some(input.options.clone());
    line.recommendation = Some(input.recommendation.clone());
    line.blocker.clone_from(&input.blocker);
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
        RaiseDecisionRequestOutcome::Raised { .. } => {
            records::append_decision(&line)
                .with_context(|| format!("{COMMAND}: request {:?}", request.0))?;
            Ok(ExitCode::SUCCESS)
        }
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
            repository: args.repository,
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
        repository: fields.repository.clone(),
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
    if let Some(repository) = &fields.repository {
        print_charter(COMMAND, &repository.0)?;
    }
    record_decision(
        COMMAND,
        state,
        &fields,
        Decider::Conductor,
        None,
        |generated, decision_id| {
            let outcome = generated
                .record_conductor_decision(input(decision_id))
                .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
            Ok(match outcome {
                RecordConductorDecisionOutcome::Recorded { .. } => Answer::Recorded,
                RecordConductorDecisionOutcome::AlreadyRecorded { .. } => Answer::Held,
                RecordConductorDecisionOutcome::Reserved { error } => Answer::Refused(Refusal {
                    error: "ReservedForOperator",
                    detail: format!(
                        "decision {:?} is class {}, which only the operator decides \
                         (design § 5); nothing was recorded",
                        error.decision_id.0,
                        class_name(fields.decision_class)
                    ),
                }),
            })
        },
    )
}

/// `conductor decision record-operator-decision`: the command
/// `conductor.decision.RecordOperatorDecision`. Prints the decision's id.
///
/// Besides the fields both record commands take, it records who wrote the decision down and on
/// what words (`story:operator-decision-guarded`): `--recorded-by`, the session that runs it, and
/// `--operator-words`, the operator's words, quoted. Each is required and must hold more than
/// white space; the command reads no session from its environment.
///
/// # Errors
///
/// A flag is missing, `--recorded-by` or `--operator-words` is blank, `--input-json` is given
/// ([`NotImplemented`](crate::NotImplemented)), the store does not open or keep the decision, the
/// given id is already held, or no id could be allocated.
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
            repository: args.repository,
        },
    )?;
    let recorded_by = not_blank(COMMAND, "--recorded-by", args.recorded_by)?;
    let operator_words = not_blank(COMMAND, "--operator-words", args.operator_words)?;
    record_decision(
        COMMAND,
        state,
        &fields,
        Decider::Operator,
        Some((recorded_by.as_str(), operator_words.as_str())),
        |generated, decision_id| {
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
                    recorded_by: recorded_by.clone(),
                    operator_words: operator_words.clone(),
                    repository: fields.repository.clone(),
                })
                .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
            Ok(match outcome {
                RecordOperatorDecisionOutcome::Recorded { .. } => Answer::Recorded,
                RecordOperatorDecisionOutcome::AlreadyRecorded { .. } => Answer::Held,
            })
        },
    )
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
        AnswerRequestOutcome::Answered { request_answered } => request_logged(
            COMMAND,
            &generated.ports,
            &request_answered.request_id,
            DecisionLogEvent::Answered,
            |line| line.decision_id = Some(request_answered.decision_id.clone()),
        ),
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
        AnswerEscalatedRequestOutcome::Answered { request_answered } => request_logged(
            COMMAND,
            &generated.ports,
            &request_answered.request_id,
            DecisionLogEvent::Answered,
            |line| line.decision_id = Some(request_answered.decision_id.clone()),
        ),
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

/// The operator's words an answer from the Decisions page is recorded on when he gave none
/// (`story:decisions-batch-answer`).
pub const PAGE_WORDS: &str = "chosen on the Decisions page";

/// The choice a class-H item is answered with: the operator acted on it.
pub const DONE: &str = "done";

/// One answer of the operator to an escalated request (`story:decisions-batch-answer`), as the
/// Decisions page sends it or conductor relays it: [`answer_as_operator`] records it.
#[derive(Debug, Clone, Default)]
pub struct OperatorAnswer {
    /// The escalated request it answers.
    pub request_id: String,
    /// The letters chosen, each one of the request's lettered options; [`DONE`] for class H.
    pub choice: Vec<String>,
    /// The operator's words; none, or only white space, records [`PAGE_WORDS`].
    pub operator_words: Option<String>,
    /// The session that records it: `dashboard` for the Decisions page.
    pub recorded_by: String,
    /// The decision's id; allocated as `record-operator-decision` allocates one when none.
    pub decision_id: Option<String>,
    /// When he decided; now when none.
    pub decided_at: Option<String>,
    /// Why; the operator's words when none.
    pub reason: Option<String>,
    /// What it rests on; the request and the recording session when none.
    pub evidence: Option<String>,
    /// What the caller says the request holds, checked against it: the class, the question, the
    /// options and the repository. The request's own are recorded.
    pub decision_class: Option<String>,
    pub question: Option<String>,
    pub options: Option<String>,
    pub repository: Option<String>,
}

/// What [`answer_as_operator`] did with one answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorAnswered {
    /// The decision is recorded under this id, and its request is Answered with it.
    Recorded(String),
    /// Nothing was recorded: the specification's error, and why.
    Refused { error: &'static str, reason: String },
}

/// `conductor decision answer-escalated-as-operator`: the command
/// `conductor.decision.RecordOperatorDecisionForRequest`, then
/// `conductor.decision.AnswerEscalatedRequest`, over one store handle
/// ([`answer_as_operator`]). Prints the decision's id.
///
/// # Errors
///
/// A flag is missing, `--recorded-by` or `--operator-words` is blank, `--input-json` is given
/// ([`NotImplemented`](crate::NotImplemented)), a flag names what the request does not hold, the
/// store does not open or keep the answer, or the specification refuses it: `RequestNotFound`,
/// `RequestStateConflict`, `ChoiceInvalid`, `DecisionExists`.
pub fn answer_escalated_as_operator(
    state: Option<&Path>,
    args: RecordOperatorDecisionForRequestArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision answer-escalated-as-operator";
    flags_only(
        args.input_json,
        "decision answer-escalated-as-operator --input-json",
    )?;
    let answer = OperatorAnswer {
        request_id: flag(COMMAND, "--request-id", args.request_id)?,
        choice: flag(COMMAND, "--choice", args.choice)?
            .split(',')
            .map(|letter| letter.trim().to_owned())
            .collect(),
        operator_words: Some(not_blank(COMMAND, "--operator-words", args.operator_words)?),
        recorded_by: not_blank(COMMAND, "--recorded-by", args.recorded_by)?,
        decision_id: args.decision_id,
        decided_at: args.decided_at,
        reason: args.reason,
        evidence: args.evidence,
        decision_class: args.decision_class,
        question: args.question,
        options: args.options,
        repository: args.repository,
    };
    if let Some(given) = &answer.decision_id {
        in_allocator_form(COMMAND, "--decision-id", DECISION_PREFIX, given)?;
    }
    match answer_as_operator(state, &answer).with_context(|| COMMAND.to_owned())? {
        OperatorAnswered::Recorded(decision_id) => print_line(COMMAND, &decision_id),
        OperatorAnswered::Refused { error, reason } => refused(COMMAND, error, &reason),
    }
}

/// Records `answer` as the operator's decision and moves its request from Escalated to Answered
/// with it (`story:decisions-batch-answer`): the one path of the Decisions page and of conductor
/// relaying an answer.
///
/// It reads the request first and refuses, recording nothing, when no request has the id
/// (`RequestNotFound`), the request is not escalated (`RequestStateConflict`: open, or answered
/// or withdrawn meanwhile) or the choice does not fit it (`ChoiceInvalid`, [`choice_for`]). It
/// then records the decision through `RecordOperatorDecisionForRequest`, whose class, question,
/// options and repository are the request's, and moves the request through
/// `AnswerEscalatedRequest` over the same handle, appending the `decided` and `answered` lines.
/// The store commits each write on its own: should another process answer or withdraw the
/// request between the two, the decision just recorded is reversed as the operator's, with that
/// reason, and the answer is refused `RequestStateConflict`, so no decision answers no request.
///
/// # Errors
///
/// The store does not open or keep a read or a write, a records line cannot be written, a field
/// the caller gives is not the request's, or no decision id could be allocated.
pub fn answer_as_operator(
    state: Option<&Path>,
    answer: &OperatorAnswer,
) -> Result<OperatorAnswered> {
    const COMMAND: &str = "answer the escalated request as the operator";
    let request = RequestId(answer.request_id.clone());
    let refusal =
        |error: &'static str, reason: String| Ok(OperatorAnswered::Refused { error, reason });
    let generated = record(COMMAND, state)?;
    let held = DecisionRequestStorage::get(&generated.ports, &request);
    kept(COMMAND, &generated.ports.store)?;
    let Some(held) = held else {
        return refusal("RequestNotFound", no_request(&request));
    };
    if held.state != DecisionRequestState::Escalated {
        return refusal("RequestStateConflict", not_escalated(&request, held.state));
    }
    let data = held.data;
    let class = data.escalation_class.unwrap_or(DecisionClass::O);
    let choice = match choice_for(class, &data, &answer.choice) {
        Ok(choice) => choice,
        Err(reason) => return refusal("ChoiceInvalid", format!("{reason}; nothing was recorded")),
    };
    let options = data.lettered_options.as_deref().map_or_else(
        || data.options.clone(),
        |options| {
            options
                .iter()
                .map(|option| format!("{}: {} ({})", option.letter, option.label, option.cost))
                .collect::<Vec<_>>()
                .join("; ")
        },
    );
    for (flag, given, held) in [
        (
            "--decision-class",
            answer.decision_class.as_deref(),
            Some(class_name(class)),
        ),
        (
            "--question",
            answer.question.as_deref(),
            Some(data.question.as_str()),
        ),
        ("--options", answer.options.as_deref(), None),
        (
            "--repository",
            answer.repository.as_deref(),
            Some(data.repository.0.as_str()),
        ),
    ] {
        let fits = match (flag, given) {
            (_, None) => true,
            ("--options", Some(given)) => given == data.options || given == options,
            (_, Some(given)) => Some(given) == held,
        };
        if !fits {
            bail!(
                "{flag} {:?} is not what request {:?} holds; leave it out and the request's is \
                 recorded",
                given.unwrap_or_default(),
                request.0
            );
        }
    }
    let words = answer
        .operator_words
        .as_deref()
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .unwrap_or(PAGE_WORDS)
        .to_owned();
    let fields = Fields {
        decision_id: answer.decision_id.clone().map(DecisionId),
        decision_class: class,
        question: data.question.clone(),
        options,
        choice,
        reason: answer.reason.clone().unwrap_or_else(|| words.clone()),
        evidence: answer.evidence.clone().unwrap_or_else(|| {
            format!(
                "the operator's answer to request {}, recorded by {}",
                request.0, answer.recorded_by
            )
        }),
        decided_at: match &answer.decided_at {
            Some(at) => Timestamp(at.clone()),
            None => records::now()?,
        },
        repository: Some(data.repository.clone()),
    };
    // The first attempt records over the handle the request was read with: one store open.
    let recorded = allocate(
        COMMAND,
        state,
        Some(generated),
        &fields,
        |generated, decision_id| {
            let outcome = generated
                .record_operator_decision_for_request(RecordOperatorDecisionForRequest {
                    request_id: request.clone(),
                    decision_id,
                    decision_class: fields.decision_class,
                    question: fields.question.clone(),
                    options: fields.options.clone(),
                    choice: fields.choice.clone(),
                    reason: fields.reason.clone(),
                    evidence: fields.evidence.clone(),
                    decided_at: fields.decided_at.clone(),
                    recorded_by: answer.recorded_by.clone(),
                    operator_words: words.clone(),
                    repository: fields.repository.clone(),
                })
                .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
            Ok(match outcome {
                RecordOperatorDecisionForRequestOutcome::Recorded { .. } => Answer::Recorded,
                RecordOperatorDecisionForRequestOutcome::AlreadyRecorded { .. } => Answer::Held,
                RecordOperatorDecisionForRequestOutcome::NoSuchRequest { error } => {
                    Answer::Refused(Refusal {
                        error: "RequestNotFound",
                        detail: no_request(&error.request_id),
                    })
                }
                RecordOperatorDecisionForRequestOutcome::NotEscalated { error } => {
                    Answer::Refused(Refusal {
                        error: "RequestStateConflict",
                        detail: format!(
                            "request {:?} is no longer escalated; nothing was recorded",
                            error.request_id.0
                        ),
                    })
                }
            })
        },
    )?;
    let (mut generated, decision_id) = match recorded {
        Allocated::Recorded(generated, decision_id) => (*generated, decision_id),
        Allocated::Refused(Refusal { error, detail }) => return refusal(error, detail),
        Allocated::Held(decision_id) => {
            return refusal(
                "DecisionExists",
                format!(
                    "decision {:?} already exists; nothing was written",
                    decision_id.0
                ),
            );
        }
    };
    decided(
        COMMAND,
        &fields,
        &decision_id,
        Some((answer.recorded_by.as_str(), words.as_str())),
    )?;
    let outcome = generated
        .answer_escalated_request(AnswerEscalatedRequest {
            request_id: request.clone(),
            decision_id: decision_id.clone(),
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    let moved = match (generated.ports.store.check(), outcome) {
        (Ok(()), AnswerEscalatedRequestOutcome::Answered { .. }) => true,
        (Ok(()), _) | (Err(StoreError::Moved { .. }), _) => false,
        (Err(failure), _) => {
            return Err(anyhow::Error::new(failure))
                .with_context(|| format!("{COMMAND}: the store did not keep the answer"));
        }
    };
    if moved {
        request_logged(
            COMMAND,
            &generated.ports,
            &request,
            DecisionLogEvent::Answered,
            |line| line.decision_id = Some(decision_id.clone()),
        )?;
        return Ok(OperatorAnswered::Recorded(decision_id.0));
    }
    // Another process moved the request between the two writes: the decision answers nothing,
    // so it does not stay in force.
    let reason = format!(
        "request {:?} was answered or withdrawn while this answer was recorded",
        request.0
    );
    let mut fresh = record(COMMAND, state)?;
    let reversed = fresh
        .reverse_operator_decision(ReverseOperatorDecision {
            decision_id: decision_id.clone(),
            reason: reason.clone(),
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &fresh.ports.store)?;
    if let ReverseOperatorDecisionOutcome::Reversed { .. } = reversed {
        decision_logged(
            COMMAND,
            &fresh.ports,
            &decision_id,
            DecisionLogEvent::Reversed,
            |line| {
                line.decided_by = Some(Decider::Operator);
                line.reason = Some(reason.clone());
            },
        )?;
    }
    refusal(
        "RequestStateConflict",
        format!("{reason}; decision {} was reversed", decision_id.0),
    )
}

/// The choice `given` records for a request of `class` holding `data`, joined by `,` in the order
/// of the request's options; why it does not fit, when it does not (`ChoiceInvalid`): a class-H
/// item takes [`DONE`]; a class-O request takes one of its lettered options for `yes-no` (`Y` or
/// `N`) and `one-of`, and one or more distinct ones for `several-of`. A class-O request escalated
/// without lettered options is answered in conductor's session, not by letter.
fn choice_for(
    class: DecisionClass,
    data: &conductor_model::decision::DecisionRequestData,
    given: &[String],
) -> std::result::Result<String, String> {
    let request = &data.request_id.0;
    let shown = given.join(",");
    if class == DecisionClass::H {
        return if given == [DONE] {
            Ok(DONE.to_owned())
        } else {
            Err(format!(
                "request {request:?} is a class-H item, answered {DONE:?}, not {shown:?}"
            ))
        };
    }
    let (Some(kind), Some(options)) = (data.kind, data.lettered_options.as_deref()) else {
        return Err(format!(
            "request {request:?} was escalated without lettered options; answer it in \
             conductor's session"
        ));
    };
    if given.is_empty() || given.iter().any(String::is_empty) {
        return Err(format!("the answer to request {request:?} names no letter"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for letter in given {
        if !options.iter().any(|option| option.letter == *letter) {
            return Err(format!(
                "{letter:?} is none of the letters of request {request:?} ({})",
                options
                    .iter()
                    .map(|option| option.letter.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !seen.insert(letter.as_str()) {
            return Err(format!(
                "the answer to request {request:?} names {letter:?} twice"
            ));
        }
    }
    if kind != RequestKind::SeveralOf && given.len() > 1 {
        return Err(format!(
            "request {request:?} is {}, answered with one letter, not {shown:?}",
            kind_name(kind)
        ));
    }
    Ok(options
        .iter()
        .filter(|option| seen.contains(option.letter.as_str()))
        .map(|option| option.letter.as_str())
        .collect::<Vec<_>>()
        .join(","))
}

/// Why `request`, resting in `state`, is not answered as an escalated one.
fn not_escalated(request: &RequestId, state: DecisionRequestState) -> String {
    format!(
        "request {:?} is {}, not escalated, so it is not the operator's to answer; nothing was \
         recorded",
        request.0,
        request_state_name(state)
    )
}

/// `conductor decision escalate-request`: the command `conductor.decision.EscalateRequest`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// file `--dry-run` names cannot be read, the store does not open or keep the move, or the
/// specification refuses it: `NotEscalatable`, `DryRunMissing` (class H with neither `--dry-run`
/// nor `--no-dry-run`, `story:hand-item-dry-run`), `RequestNotFound`, `RequestStateConflict`.
pub fn escalate_request(state: Option<&Path>, args: EscalateRequestArgs) -> Result<ExitCode> {
    escalate(state, args, None)
}

/// [`escalate_request`] for a caller inside conductor whose item's dry run is text it measured
/// itself, not a file (`story:hand-item-dry-run`): a disk item's numbers ([`crate::host`]).
/// `dry_run` stands for `--dry-run`, and `args.dry_run` is not read.
///
/// # Errors
///
/// As [`escalate_request`].
pub fn escalate_request_measured(
    state: Option<&Path>,
    args: EscalateRequestArgs,
    dry_run: String,
) -> Result<ExitCode> {
    escalate(state, args, Some(dry_run))
}

fn escalate(
    state: Option<&Path>,
    args: EscalateRequestArgs,
    measured: Option<String>,
) -> Result<ExitCode> {
    const COMMAND: &str = "decision escalate-request";
    flags_only(args.input_json, "decision escalate-request --input-json")?;
    let request_id = RequestId(flag(COMMAND, "--request-id", args.request_id)?);
    let escalation_class = class(
        COMMAND,
        "--escalation-class",
        flag(COMMAND, "--escalation-class", args.escalation_class)?,
    )?;
    let reason = flag(COMMAND, "--reason", args.reason)?;
    let choices = Choices::read(
        escalation_class,
        args.kind.as_deref(),
        args.lettered_options.as_deref(),
        args.recommended_letters.as_deref(),
        args.default_letters.as_deref(),
    )
    .map_err(|problem| {
        anyhow!(
            "{COMMAND}: OptionsInvalid: {problem}; request {:?} is unchanged",
            request_id.0
        )
    })?;
    let dry_run = match measured {
        Some(text) => Some(text),
        None => args
            .dry_run
            .map(|file| dry_run_of(COMMAND, &file))
            .transpose()?,
    };
    let input = EscalateRequest {
        request_id,
        escalation_class,
        reason: reason.clone(),
        dry_run,
        no_dry_run: args.no_dry_run,
        kind: choices.kind,
        lettered_options: choices.options,
        recommended_letters: choices.recommended,
        default_letters: choices.default,
        deadline: args.deadline.map(Timestamp),
    };
    let mut generated = record(COMMAND, state)?;
    let outcome = generated
        .escalate_request(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        EscalateRequestOutcome::Escalated { request_escalated } => request_logged(
            COMMAND,
            &generated.ports,
            &request_escalated.request_id,
            DecisionLogEvent::Escalated,
            |line| {
                line.escalation_class = Some(escalation_class);
                line.reason = Some(reason);
            },
        ),
        EscalateRequestOutcome::ClassC { error } => refused(
            COMMAND,
            "NotEscalatable",
            &format!(
                "class C is decided by conductor, never escalated; request {:?} is unchanged",
                error.request_id.0
            ),
        ),
        EscalateRequestOutcome::Untested { error } => refused(
            COMMAND,
            "DryRunMissing",
            &format!(
                "a class-H item carries its dry run (`--dry-run <file>`) or why it has none \
                 (`--no-dry-run <reason>`); request {:?} is unchanged",
                error.request_id.0
            ),
        ),
        EscalateRequestOutcome::Incomplete { error } => refused(
            COMMAND,
            "EscalationIncomplete",
            &format!(
                "a class-O escalation carries what the operator chooses from: `--kind`, \
                 `--lettered-options` (each option's letter, label and cost), \
                 `--recommended-letters`, `--default-letters` and `--deadline`; request {:?} is \
                 unchanged",
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

/// What the dry run in `file` printed (`story:hand-item-dry-run`), its trailing whitespace dropped;
/// a file that cannot be read is named, before any store is opened.
fn dry_run_of(command: &str, file: &str) -> Result<String> {
    let text = std::fs::read_to_string(file)
        .with_context(|| format!("{command}: --dry-run: read the dry run {file:?}"))?;
    Ok(text.trim_end().to_owned())
}

/// What an escalation names for the operator to choose from (`story:decisions-page`), read from
/// the flags of `decision escalate-request` and checked against what the specification cannot
/// say: each option has a letter, a label and a cost, no letter twice, a `yes-no` request's
/// letters are `Y` and `N`, and the recommended and default letters are among them, one for
/// `yes-no` and `one-of` and at least one for `several-of`. Letters are kept joined by `,`.
#[derive(Debug, Default)]
struct Choices {
    kind: Option<RequestKind>,
    options: Option<Vec<RequestOption>>,
    recommended: Option<String>,
    default: Option<String>,
}

impl Choices {
    /// The choices of an escalation of `class`. Letters without options are refused for class H;
    /// for class O the specification refuses the missing options (`EscalationIncomplete`).
    fn read(
        class: DecisionClass,
        kind: Option<&str>,
        options: Option<&str>,
        recommended: Option<&str>,
        default: Option<&str>,
    ) -> std::result::Result<Self, String> {
        let kind = kind
            .map(|name| kind_named(name).ok_or_else(|| format!("{name:?} is no RequestKind")))
            .transpose()?;
        let options = options.map(lettered_options).transpose()?;
        if let Some(options) = &options {
            let mut seen = std::collections::BTreeSet::new();
            for option in options {
                if !seen.insert(option.letter.as_str()) {
                    return Err(format!(
                        "the options name the letter {:?} twice",
                        option.letter
                    ));
                }
            }
            if kind == Some(RequestKind::YesNo) && seen != ["N", "Y"].into_iter().collect() {
                return Err(format!(
                    "a yes-no request's letters are Y and N, not {}",
                    seen.into_iter().collect::<Vec<_>>().join(", ")
                ));
            }
        }
        let letters = |flag: &str, given: Option<&str>| -> std::result::Result<_, String> {
            let Some(given) = given else {
                return Ok(None);
            };
            let letters: Vec<&str> = given
                .split(',')
                .map(str::trim)
                .filter(|letter| !letter.is_empty())
                .collect();
            if letters.is_empty() {
                return Err(format!("{flag} names no letter; it names at least one"));
            }
            if letters.len() > 1 && kind != Some(RequestKind::SeveralOf) {
                return Err(format!(
                    "{flag} names {}; only a several-of request takes more than one letter",
                    letters.join(",")
                ));
            }
            let Some(options) = &options else {
                if class == DecisionClass::O {
                    return Ok(Some(letters.join(",")));
                }
                return Err(format!(
                    "{flag} names letters, and no --lettered-options are given"
                ));
            };
            if let Some(stray) = letters
                .iter()
                .find(|letter| !options.iter().any(|option| option.letter == **letter))
            {
                return Err(format!(
                    "{flag} names {stray:?}, which is not an option's letter"
                ));
            }
            Ok(Some(letters.join(",")))
        };
        let recommended = letters("--recommended-letters", recommended)?;
        let default = letters("--default-letters", default)?;
        Ok(Self {
            kind,
            options,
            recommended,
            default,
        })
    }
}

/// The options `--lettered-options` holds: a JSON array of `{letter, label, cost}`, each text
/// and none blank, at least one.
fn lettered_options(text: &str) -> std::result::Result<Vec<RequestOption>, String> {
    let array: Vec<Value> = serde_json::from_str(text)
        .map_err(|error| format!("--lettered-options is no JSON array: {error}"))?;
    if array.is_empty() {
        return Err("--lettered-options names no option".to_owned());
    }
    array
        .iter()
        .enumerate()
        .map(|(at, option)| {
            let field = |name: &str| match option.get(name).and_then(Value::as_str) {
                Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
                _ => Err(format!(
                    "option {} of --lettered-options has no {name}; each has a letter, a label \
                     and a cost",
                    at + 1
                )),
            };
            Ok(RequestOption {
                letter: field("letter")?,
                label: field("label")?,
                cost: field("cost")?,
            })
        })
        .collect()
}

/// The kind's name as the specification spells it (`story:decisions-page`). Exhaustive, so a
/// kind the specification adds fails to compile here rather than being written under a guessed
/// name.
#[must_use]
pub fn kind_name(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::YesNo => "yes-no",
        RequestKind::OneOf => "one-of",
        RequestKind::SeveralOf => "several-of",
    }
}

/// The kind `name` spells, as [`kind_name`] writes it.
#[must_use]
pub fn kind_named(name: &str) -> Option<RequestKind> {
    [
        RequestKind::YesNo,
        RequestKind::OneOf,
        RequestKind::SeveralOf,
    ]
    .into_iter()
    .find(|kind| kind_name(*kind) == name)
}

/// An option as the views, the store and the dashboard write it: `{letter, label, cost}`.
#[must_use]
pub fn option_value(option: &RequestOption) -> Value {
    serde_json::json!({"letter": option.letter, "label": option.label, "cost": option.cost})
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
        WithdrawRequestOutcome::Withdrawn { request_withdrawn } => request_logged(
            COMMAND,
            &generated.ports,
            &request_withdrawn.request_id,
            DecisionLogEvent::Withdrawn,
            |line| line.reason = Some(request_withdrawn.reason.clone()),
        ),
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
        ReverseConductorDecisionOutcome::Reversed {
            decision_reversed_event,
        } => decision_logged(
            COMMAND,
            &generated.ports,
            &decision_reversed_event.decision_id,
            DecisionLogEvent::Reversed,
            |line| {
                line.decided_by = Some(Decider::Conductor);
                line.reason = Some(decision_reversed_event.reason.clone());
            },
        ),
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
        ReverseOperatorDecisionOutcome::Reversed {
            decision_reversed_event,
        } => decision_logged(
            COMMAND,
            &generated.ports,
            &decision_reversed_event.decision_id,
            DecisionLogEvent::Reversed,
            |line| {
                line.decided_by = Some(Decider::Operator);
                line.reason = Some(decision_reversed_event.reason.clone());
            },
        ),
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
                Value::from(row.kind.map(kind_name)),
                row.lettered_options.map_or(Value::Null, |options| {
                    Value::Array(options.iter().map(option_value).collect())
                }),
                Value::from(row.recommended_letters),
                Value::from(row.default_letters),
                Value::from(row.deadline.map(|deadline| deadline.0)),
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
/// the existing store and rendered as `--format` asks; a table also gives each row its age in
/// whole days since it was raised ([`HANDS_TODO_COLUMNS`]). A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn hands_todo(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const VIEW: &str = "decision hands-todo";
    let generated = existing(VIEW, state)?;
    // The age is for the reader of a table; JSON and JSONL keep the specification's view.
    let aged = matches!(view.format, Format::Text | Format::Markdown);
    let now = OffsetDateTime::now_utc();
    let rows: Vec<Vec<Value>> = generated
        .hands_todo()
        .map_err(|unmet| anyhow!("{VIEW}: {unmet}"))?
        .into_iter()
        .map(|row| {
            let age = age_in_days(&row.raised_at.0, now);
            let mut values = vec![
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
                Value::from(row.dry_run),
                Value::from(row.no_dry_run),
            ];
            if aged {
                values.push(Value::from(age));
            }
            values
        })
        .collect();
    let columns: &[&str] = if aged {
        &HANDS_TODO_COLUMNS
    } else {
        &HANDS_TODO_FIELDS
    };
    crate::store::show(
        &generated.ports.store,
        "class-H to-do",
        &view.render(columns, &rows),
    )
}

/// How long the request raised at the RFC 3339 instant `raised_at` has been open at `now`, in
/// whole days, written `<n>d` (`story:operator-items-age`); none when `raised_at` does not parse.
/// An instant after `now` is `0d`.
fn age_in_days(raised_at: &str, now: OffsetDateTime) -> Option<String> {
    let raised = OffsetDateTime::parse(raised_at, &Rfc3339).ok()?;
    Some(format!("{}d", (now - raised).whole_days().max(0)))
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
                Value::from(row.recorded_by),
                Value::from(row.operator_words),
                Value::from(row.repository.map(|repository| repository.0)),
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
const SHOWN: [&str; 13] = [
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
    "recorded_by",
    "operator_words",
    "repository",
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
        Value::from(row.recorded_by),
        Value::from(row.operator_words),
        Value::from(row.repository.map(|repository| repository.0)),
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

/// Appends the records line of a request `event` the store has kept: `id` the request, `repo` its
/// repository as the store holds it, and what `fill` adds.
fn request_logged(
    command: &str,
    ports: &Ports,
    request: &RequestId,
    event: DecisionLogEvent,
    fill: impl FnOnce(&mut DecisionLogLine),
) -> Result<ExitCode> {
    let held = DecisionRequestStorage::get(ports, request);
    kept(command, &ports.store)?;
    let held = held.ok_or_else(|| {
        anyhow!(
            "{command}: request {:?} moved, yet reads back as absent",
            request.0
        )
    })?;
    let mut line = records::decision_line(event, Some(&held.data.repository.0), &request.0)?;
    fill(&mut line);
    records::append_decision(&line)
        .with_context(|| format!("{command}: request {:?}", request.0))?;
    Ok(ExitCode::SUCCESS)
}

/// Appends the records line of a decision `event` the store has kept: `id` the decision, `repo`
/// the repository it is about as the store holds it, and what `fill` adds.
fn decision_logged(
    command: &str,
    ports: &Ports,
    decision: &DecisionId,
    event: DecisionLogEvent,
    fill: impl FnOnce(&mut DecisionLogLine),
) -> Result<ExitCode> {
    let held = DecisionStorage::get(ports, decision);
    kept(command, &ports.store)?;
    let held = held.ok_or_else(|| {
        anyhow!(
            "{command}: decision {} moved, yet reads back as absent",
            decision.0
        )
    })?;
    let repository = held.data.repository.map(|repository| repository.0);
    let mut line = records::decision_line(event, repository.as_deref(), &decision.0)?;
    fill(&mut line);
    records::append_decision(&line)
        .with_context(|| format!("{command}: decision {}", decision.0))?;
    Ok(ExitCode::SUCCESS)
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

/// `conductor.decision.RecordOperatorDecisionForRequest`, as its contract in
/// `crates/conductor-model/PLAN.md` states it: `no-such-request` when no request has the input's
/// id; `not-escalated` when it rests in any state but `Escalated`; `already-recorded` when a
/// decision has the input's id; `recorded` otherwise, creating an `InForce` decision the operator
/// decided, with the input's fields, as `RecordOperatorDecision` creates one.
impl RecordOperatorDecisionForRequestBehavior for Ports {
    fn record_operator_decision_for_request(
        &mut self,
        input: RecordOperatorDecisionForRequest,
    ) -> std::result::Result<RecordOperatorDecisionForRequestOutcome, UnmetObligation> {
        use conductor_model::decision::{
            AnyDecision, Decision, DecisionData, DecisionExists, DecisionRecorded, RequestNotFound,
            RequestStateConflict,
        };
        let Some(request) = DecisionRequestStorage::get(self, &input.request_id) else {
            return Ok(RecordOperatorDecisionForRequestOutcome::NoSuchRequest {
                error: RequestNotFound {
                    request_id: input.request_id,
                },
            });
        };
        if request.state != DecisionRequestState::Escalated {
            return Ok(RecordOperatorDecisionForRequestOutcome::NotEscalated {
                error: RequestStateConflict {
                    request_id: input.request_id,
                },
            });
        }
        if DecisionStorage::get(self, &input.decision_id).is_some() {
            return Ok(RecordOperatorDecisionForRequestOutcome::AlreadyRecorded {
                error: DecisionExists {
                    decision_id: input.decision_id,
                },
            });
        }
        let data = DecisionData {
            decision_id: input.decision_id.clone(),
            decision_class: input.decision_class,
            decided_by: Decider::Operator,
            question: input.question,
            options: input.options,
            choice: input.choice,
            reason: input.reason,
            evidence: input.evidence,
            decided_at: input.decided_at,
            recorded_by: Some(input.recorded_by),
            operator_words: Some(input.operator_words),
            repository: input.repository,
        };
        DecisionStorage::put(self, AnyDecision::InForce(Decision::new(data)).snapshot());
        Ok(RecordOperatorDecisionForRequestOutcome::Recorded {
            decision_recorded: DecisionRecorded {
                decision_id: input.decision_id,
                decision_class: input.decision_class,
                decided_by: Decider::Operator,
            },
        })
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
    repository: Option<String>,
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
    /// `--repository`, when it was given: the repository the decision is about.
    repository: Option<RepositoryName>,
}

impl Fields {
    /// Every flag but `--decision-id` and `--repository` is required; the first one missing is named, in the order
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
            repository: flags.repository.map(RepositoryName),
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
/// (`DecisionExists`), or else the [`next_id`] after the [`highest`] number stored or written by
/// hand ([`written_highest`]) for the UTC date of `--decided-at`, allocated again when another
/// process records it first: while that highest number moves, and in at most [`ATTEMPTS`]
/// attempts. Prints the id of the recorded decision.
///
/// The behaviour reads the id before it writes, so the write expects no stream: a decision
/// another process records in between is refused by the store ([`StoreError::Exists`]) rather
/// than replaced, and is answered as one the behaviour found.
fn record_decision(
    command: &str,
    state: Option<&Path>,
    fields: &Fields,
    decider: Decider,
    operator: Option<(&str, &str)>,
    run: impl FnMut(&mut Generated<Ports>, DecisionId) -> Result<Answer>,
) -> Result<ExitCode> {
    match allocate(command, state, None, fields, run)? {
        Allocated::Recorded(_, decision_id) => {
            let mut line = decided_line(fields, &decision_id, operator)?;
            line.decided_by = Some(decider);
            records::append_decision(&line)
                .with_context(|| format!("{command}: decision {}", decision_id.0))?;
            print_line(command, &decision_id.0)
        }
        Allocated::Refused(Refusal { error, detail }) => refused(command, error, &detail),
        Allocated::Held(decision_id) => already_held(command, &decision_id),
    }
}

/// What [`allocate`] did.
enum Allocated {
    /// The decision is recorded under the id, through the handle given back.
    Recorded(Box<Generated<Ports>>, DecisionId),
    /// The behaviour refused it, and nothing was recorded.
    Refused(Refusal),
    /// The id given by hand is already a decision's (`DecisionExists`).
    Held(DecisionId),
}

/// The records line `decided` of the decision `decision_id` recorded from `fields`, decided by
/// the operator, who `operator` names as the recording session and his words when given.
fn decided_line(
    fields: &Fields,
    decision_id: &DecisionId,
    operator: Option<(&str, &str)>,
) -> Result<DecisionLogLine> {
    let mut line = records::decision_line(
        DecisionLogEvent::Decided,
        fields
            .repository
            .as_ref()
            .map(|repository| repository.0.as_str()),
        &decision_id.0,
    )?;
    line.class = Some(fields.decision_class);
    line.decided_by = Some(Decider::Operator);
    line.question = Some(fields.question.clone());
    line.options = Some(fields.options.clone());
    line.choice = Some(fields.choice.clone());
    line.reason = Some(fields.reason.clone());
    line.evidence = Some(fields.evidence.clone());
    if let Some((recorded_by, operator_words)) = operator {
        line.recorded_by = Some(recorded_by.to_owned());
        line.operator_words = Some(operator_words.to_owned());
    }
    Ok(line)
}

/// Appends the operator's `decided` line of `decision_id` ([`decided_line`]).
fn decided(
    command: &str,
    fields: &Fields,
    decision_id: &DecisionId,
    operator: Option<(&str, &str)>,
) -> Result<()> {
    records::append_decision(&decided_line(fields, decision_id, operator)?)
        .with_context(|| format!("{command}: decision {}", decision_id.0))
}

/// Runs `run` for the id [`record_decision`] describes until it records or refuses: the id
/// given by hand, or the next one allocated, again while another process records it first. The
/// first attempt runs over `opened` when given, a handle that has only read.
fn allocate(
    command: &str,
    state: Option<&Path>,
    opened: Option<Generated<Ports>>,
    fields: &Fields,
    mut run: impl FnMut(&mut Generated<Ports>, DecisionId) -> Result<Answer>,
) -> Result<Allocated> {
    let date = utc_day(&fields.decided_at.0)
        .with_context(|| format!("{command}: --decided-at {:?}", fields.decided_at.0))?;
    let prefix = format!("{DECISION_PREFIX}{date}-");
    let written = written_highest(&prefix);
    let mut before = None;
    let mut opened = opened;
    for _ in 0..ATTEMPTS {
        // A fresh handle per attempt: a handle refuses every write after its first failure.
        let mut generated = match opened.take() {
            Some(opened) => opened,
            None => record(command, state)?,
        };
        let decision_id = match &fields.decision_id {
            Some(given) => given.clone(),
            None => {
                let highest = stored_highest(command, &generated.ports, &prefix)?.max(written);
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
            Answer::Recorded => return Ok(Allocated::Recorded(Box::new(generated), decision_id)),
            Answer::Refused(refusal) => return Ok(Allocated::Refused(refusal)),
            Answer::Held if fields.decision_id.is_some() => {
                return Ok(Allocated::Held(decision_id));
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

/// The highest number under `prefix` ([`highest`]) among the ids of the instance's hand-written
/// decision files: the `"id"` of each line of each `decisions/*.jsonl` in the records directory
/// the config file names (`story:decision-id-free-in-records`), 0 when there is none. Without a
/// config file nothing is read. A directory or file that cannot be read, and a line that is not a
/// JSON object with a string `"id"`, are passed over.
fn written_highest(prefix: &str) -> u128 {
    let active = crate::config::active();
    if !active.from_file {
        return 0;
    }
    let Ok(entries) = std::fs::read_dir(Path::new(&active.instance.records).join("decisions"))
    else {
        return 0;
    };
    let ids: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .filter_map(|path| std::fs::read(path).ok())
        .flat_map(|bytes| {
            bytes
                .split(|byte| *byte == b'\n')
                .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
                .filter_map(|line| line.get("id")?.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .collect();
    highest(prefix, ids.iter().map(String::as_str))
}

/// Prints the "Do not touch" and standing-rule lines of `repository`'s charter on standard error
/// (`story:decision-reads-charter`): a first line naming the repository and the charter, then
/// [`charter_lines`]. The charter is `charters/<repository>.md` in the records directory the
/// config file names; without a config file, without a charter that reads, or without such lines,
/// nothing is printed.
fn print_charter(command: &str, repository: &str) -> Result<()> {
    let active = crate::config::active();
    if !active.from_file {
        return Ok(());
    }
    let path = Path::new(&active.instance.records)
        .join("charters")
        .join(format!("{repository}.md"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let lines = charter_lines(&text);
    if lines.is_empty() {
        return Ok(());
    }
    let mut err = std::io::stderr().lock();
    writeln!(
        err,
        "{command}: check the decision against the \"Do not touch\" and standing-rule lines of \
         {repository}'s charter, {}:",
        path.display()
    )
    .and_then(|()| lines.iter().try_for_each(|line| writeln!(err, "{line}")))
    .with_context(|| format!("{command}: write to standard error"))
}

/// The "Do not touch" and standing-rule lines of a charter, in its order, blank lines left out.
///
/// A standing-rule line is a line of a section whose heading starts with "Standing", in any case
/// (`## Standing decisions`, `## Standing: …`), the heading included; the section ends at the next
/// heading. A "Do not touch" line is a line of a paragraph or list item that says "do not touch",
/// in any case, wrapped over lines or not; a paragraph or item ends at a blank line, a heading or
/// the start of a list item.
fn charter_lines(text: &str) -> Vec<&str> {
    /// A heading's text, without its `#`s.
    fn heading(line: &str) -> Option<&str> {
        let text = line.trim_start_matches('#');
        (text.len() < line.len()).then(|| text.trim())
    }
    /// Whether a line starts a list item: `-`, `*`, `+` or `<n>.` and a space.
    fn item(line: &str) -> bool {
        let line = line.trim_start();
        let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
        let rest = if rest.len() < line.len() {
            rest.strip_prefix('.')
        } else {
            rest.strip_prefix(['-', '*', '+'])
        };
        rest.is_some_and(|rest| rest.starts_with(' '))
    }
    let mut chosen = Vec::new();
    let mut standing = false;
    let mut entry: Vec<&str> = Vec::new();
    let flush = |entry: &mut Vec<&str>, chosen: &mut Vec<(usize, bool)>| {
        let words = entry
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if words.contains("do not touch") {
            for slot in chosen.iter_mut().rev().take(entry.len()) {
                slot.1 = true;
            }
        }
        entry.clear();
    };
    for (index, line) in text.lines().enumerate() {
        let blank = line.trim().is_empty();
        let title = heading(line);
        if blank || title.is_some() || item(line) {
            flush(&mut entry, &mut chosen);
        }
        if let Some(title) = title {
            standing = title
                .get(..8)
                .is_some_and(|start| start.eq_ignore_ascii_case("standing"));
        }
        if blank {
            continue;
        }
        chosen.push((index, standing));
        if title.is_none() {
            entry.push(line);
        }
    }
    flush(&mut entry, &mut chosen);
    let lines: Vec<&str> = text.lines().collect();
    chosen
        .into_iter()
        .filter(|(_, keep)| *keep)
        .map(|(index, _)| lines[index])
        .collect()
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

/// The value of the flag `name`, or an error naming it when it is missing or holds only white
/// space.
fn not_blank(command: &str, name: &str, value: Option<String>) -> Result<String> {
    let value = flag(command, name, value)?;
    if value.trim().is_empty() {
        bail!("{command}: {name} is blank; nothing was recorded");
    }
    Ok(value)
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
pub(crate) const fn class_name(class: DecisionClass) -> &'static str {
    match class {
        DecisionClass::C => "C",
        DecisionClass::O => "O",
        DecisionClass::H => "H",
    }
}

/// The decider's name as the specification spells it. Exhaustive, so a decider the specification
/// adds fails to compile here rather than rendering under a guessed name.
pub(crate) const fn decider_name(decider: Decider) -> &'static str {
    match decider {
        Decider::Conductor => "Conductor",
        Decider::Operator => "Operator",
    }
}
