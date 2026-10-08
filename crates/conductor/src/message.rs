//! `conductor message`: every message between conductor and a controller (design § 6).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else `state/`; [`crate::state::open`]), runs the generated behaviour, and checks
//! that the store kept what the behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0. A refused command exits 1 with one line on standard error: the
//! command, the error the specification declares, and what it means. A flag left out is named on
//! standard error (exit 1) before any store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! `receive-message` reads the first line itself ([`read`]): whether it has one of the § 6 forms
//! ([`FORMAT`]), and the kind its tag names, the first word after `[` with or without the closing
//! `]`. A `<dispatch-id>` in the brackets starts with `DSP-` and a `(<decision-id>)` with `DEC-`,
//! the prefixes the specification declares, or the line does not parse. A line that parses is
//! recorded `Received` and the command prints the message's id, which every other command takes.
//! A line that does not parse is recorded straight into `Rejected` (`parsed` false), with the kind
//! its tag names; the command prints [`FORMAT`] on standard output for the sender, names the
//! message on standard error, and exits 1. `--kind` and `--parsed`, when given, must agree with the
//! line, or nothing is written. A line whose tag names
//! no kind (`hello`) is received as kind `Unknown`, which never parses, unless `--kind` names one.
//!
//! The view opens only an existing store ([`crate::state::open_existing`]) and creates nothing. It
//! lists every row it can read, in the order the messages were first stored, and answers through
//! [`crate::store::show`].

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, MessageStorage, TryContext, unmet_context};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::obligations::{
    HandleMessageBehavior, MessagesQuery, ReceiveMessageBehavior, RejectMessageBehavior,
    RouteNeedBehavior,
};
use conductor_model::dispatch::{
    DispatchId, GuardDecisionId, HandleMessage, HandleMessageOutcome, MessageId, MessageKind,
    MessageSnapshot, MessageState, ReceiveMessage, ReceiveMessageOutcome, RejectMessage,
    RejectMessageOutcome, ResourceRequestId, RouteNeed, RouteNeedOutcome, Transport,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::Value;

use crate::cli::{
    HandleMessageArgs, JsonInput, ReceiveMessageArgs, RejectMessageArgs, RouteNeedArgs, ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// What `receive-message` prints for the sender of a first line that does not parse: the § 6
/// forms.
pub const FORMAT: &str = "\
The first line of a message to conductor is one of:
[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>
[REPORT <repo> <dispatch-id>] <started|progress|blocked|unblocked|done|failed> <one line>
[DECISION-REQUEST <repo> <request-id>] <question>
[DECISION <request-id>] <option> <one-line reason> (<decision-id>)
[NEED <repo> <request-id>] <other-repo> <what is needed>
[RESOURCE <repo>] <build-slot|disk|usage> <amount>
[ESCALATION <request-id>] <O|H> <question>
";

/// The prefix `conductor.dispatch.DispatchId` declares (`spec/domains/dispatch.yaml`).
const DISPATCH_ID: &str = "DSP-";

/// The prefix `conductor.decision.DecisionId` declares (`spec/domains/decision.yaml`).
const DECISION_ID: &str = "DEC-";

/// One § 6 form: the tag that opens it, the kind it records, the fields that follow the tag
/// inside the brackets, and whether the words after the brackets fit it. Each field is the prefix
/// its type declares, or `""` for a type that declares none.
struct Form {
    tag: &'static str,
    kind: MessageKind,
    fields: &'static [&'static str],
    rest: fn(&[&str]) -> bool,
}

/// The § 6 forms the specification has a kind for. `[RESTART …]` is design only (§ 6) and has
/// no `MessageKind`, so it does not parse.
const FORMS: [Form; 7] = [
    Form {
        tag: "DISPATCH",
        kind: MessageKind::Dispatch,
        fields: &["", DISPATCH_ID],
        rest: |words| words.len() >= 2,
    },
    Form {
        tag: "REPORT",
        kind: MessageKind::Report,
        fields: &["", DISPATCH_ID],
        rest: |words| {
            words.len() >= 2
                && [
                    "started",
                    "progress",
                    "blocked",
                    "unblocked",
                    "done",
                    "failed",
                ]
                .contains(&words[0])
        },
    },
    Form {
        tag: "DECISION-REQUEST",
        kind: MessageKind::DecisionRequest,
        fields: &["", ""],
        rest: |words| !words.is_empty(),
    },
    Form {
        tag: "DECISION",
        kind: MessageKind::Decision,
        fields: &[""],
        rest: |words| {
            words.len() >= 3
                && words.last().is_some_and(|last| {
                    last.strip_prefix('(')
                        .and_then(|last| last.strip_suffix(')'))
                        .is_some_and(|id| id.starts_with(DECISION_ID))
                })
        },
    },
    Form {
        tag: "NEED",
        kind: MessageKind::Need,
        fields: &["", ""],
        rest: |words| words.len() >= 2,
    },
    Form {
        tag: "RESOURCE",
        kind: MessageKind::Resource,
        fields: &[""],
        rest: |words| words.len() == 2 && ["build-slot", "disk", "usage"].contains(&words[0]),
    },
    Form {
        tag: "ESCALATION",
        kind: MessageKind::Escalation,
        fields: &[""],
        rest: |words| words.len() >= 2 && ["O", "H"].contains(&words[0]),
    },
];

/// What a first line says: the kind its tag names, if it names one, and whether the whole line
/// has that kind's § 6 form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// The kind the line's tag names: `[NEED …` is a need even when the rest breaks the form.
    pub kind: Option<MessageKind>,
    /// Whether the line has its kind's form, with nothing missing.
    pub parsed: bool,
}

/// Reads `line` as a § 6 first line. The tag is the first word after `[`, ended by white space
/// or `]`, so a line that names a kind with its tag is of that kind even when its `]` is missing.
/// A field inside the brackets whose type declares a prefix holds a value with that prefix. A line
/// holding a line break is no first line, and does not parse.
#[must_use]
pub fn read(line: &str) -> Reading {
    let Some(opened) = line.trim().strip_prefix('[') else {
        return Reading {
            kind: None,
            parsed: false,
        };
    };
    let tag = opened
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == ']')
        .next()
        .unwrap_or_default();
    let Some(form) = FORMS.iter().find(|form| form.tag == tag) else {
        return Reading {
            kind: None,
            parsed: false,
        };
    };
    let parsed = opened.split_once(']').is_some_and(|(inside, rest)| {
        let fields: Vec<&str> = inside.split_whitespace().skip(1).collect();
        let words: Vec<&str> = rest.split_whitespace().collect();
        !line.contains(['\n', '\r'])
            && fields.len() == form.fields.len()
            && fields
                .iter()
                .zip(form.fields)
                .all(|(field, prefix)| field.starts_with(prefix))
            && rest.starts_with(' ')
            && (form.rest)(&words)
    });
    Reading {
        kind: Some(form.kind),
        parsed,
    }
}

/// `conductor message receive-message`: the command `conductor.dispatch.ReceiveMessage`. Prints
/// the message's id when its first line parses; otherwise prints [`FORMAT`] and exits 1, with the
/// message recorded `Rejected`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)),
/// `--kind` or `--parsed` disagrees with the first line, the store does not open or keep the
/// message, or the first line does not parse.
pub fn receive_message(state: Option<&Path>, args: ReceiveMessageArgs) -> Result<ExitCode> {
    const COMMAND: &str = "message receive-message";
    flags_only(args.input_json, "message receive-message --input-json")?;
    let sender = flag(COMMAND, "--sender", args.sender)?;
    let recipient = flag(COMMAND, "--recipient", args.recipient)?;
    let first_line = flag(COMMAND, "--first-line", args.first_line)?;
    let transport = flag(COMMAND, "--transport", args.transport)?;
    let transport = transport_named(&transport)
        .ok_or_else(|| anyhow!("{COMMAND}: --transport {transport:?} is no Transport"))?;
    let received_at = Timestamp(flag(COMMAND, "--received-at", args.received_at)?);
    let reading = read(&first_line);
    let given = match args.kind {
        Some(kind) => Some(
            kind_named(&kind)
                .ok_or_else(|| anyhow!("{COMMAND}: --kind {kind:?} is no MessageKind"))?,
        ),
        None => None,
    };
    let kind = match (reading.kind, given) {
        (Some(read), Some(given)) if read != given => bail!(
            "{COMMAND}: --kind {} disagrees with the first line, whose tag names {}; nothing was \
             received",
            kind_name(given),
            kind_name(read)
        ),
        (Some(kind), _) | (None, Some(kind)) => kind,
        (None, None) => MessageKind::Unknown,
    };
    if let Some(parsed) = args.parsed
        && parsed != reading.parsed
    {
        bail!(
            "{COMMAND}: --parsed {parsed} disagrees with the first line, which {}; nothing was \
             received",
            if reading.parsed {
                "parses"
            } else {
                "does not parse"
            }
        );
    }
    let input = ReceiveMessage {
        sender,
        recipient,
        kind,
        first_line,
        transport,
        received_at,
        body_path: args.body_path,
        parsed: reading.parsed,
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .receive_message(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        ReceiveMessageOutcome::Received { message_received } => {
            print(COMMAND, &format!("{}\n", message_received.message_id.0.0))
        }
        ReceiveMessageOutcome::Unparsed { message_received } => {
            print(COMMAND, FORMAT)?;
            bail!(
                "{COMMAND}: the first line does not parse, so message {} is recorded as rejected; \
                 the format for its sender is on standard output",
                message_received.message_id.0.0
            )
        }
    }
}

/// `conductor message handle-message`: the command `conductor.dispatch.HandleMessage`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `NeedNotRouted`,
/// `MessageNotFound`, `MessageStateConflict`.
pub fn handle_message(state: Option<&Path>, args: HandleMessageArgs) -> Result<ExitCode> {
    const COMMAND: &str = "message handle-message";
    flags_only(args.input_json, "message handle-message --input-json")?;
    let input = HandleMessage {
        message_id: message_id(COMMAND, args.message_id)?,
    };
    match run(state, COMMAND, |generated| generated.handle_message(input))? {
        HandleMessageOutcome::Handled { .. } => Ok(ExitCode::SUCCESS),
        HandleMessageOutcome::NeedUnrouted { error } => refused(
            COMMAND,
            "NeedNotRouted",
            &format!(
                "message {} is a [NEED]; route it to the dispatch it opened with `message \
                 route-need`",
                error.message_id.0.0
            ),
        ),
        HandleMessageOutcome::NoSuchMessage { error } => no_message(COMMAND, &error.message_id),
        HandleMessageOutcome::WrongState { error } => answered(COMMAND, &error.message_id),
    }
}

/// `conductor message reject-message`: the command `conductor.dispatch.RejectMessage`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `MessageNotFound`,
/// `MessageStateConflict`.
pub fn reject_message(state: Option<&Path>, args: RejectMessageArgs) -> Result<ExitCode> {
    const COMMAND: &str = "message reject-message";
    flags_only(args.input_json, "message reject-message --input-json")?;
    let input = RejectMessage {
        message_id: message_id(COMMAND, args.message_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    match run(state, COMMAND, |generated| generated.reject_message(input))? {
        RejectMessageOutcome::Rejected { .. } => Ok(ExitCode::SUCCESS),
        RejectMessageOutcome::NoSuchMessage { error } => no_message(COMMAND, &error.message_id),
        RejectMessageOutcome::WrongState { error } => answered(COMMAND, &error.message_id),
    }
}

/// `conductor message route-need`: the command `conductor.dispatch.RouteNeed`. The need is
/// handled, and records the dispatch it opened; that dispatch is sent first, with `dispatch
/// send-dispatch` naming the repository the need asks.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `NotANeed`,
/// `MessageNotFound`, `MessageStateConflict`.
pub fn route_need(state: Option<&Path>, args: RouteNeedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "message route-need";
    flags_only(args.input_json, "message route-need --input-json")?;
    let input = RouteNeed {
        message_id: message_id(COMMAND, args.message_id)?,
        dispatch_id: DispatchId(flag(COMMAND, "--dispatch-id", args.dispatch_id)?),
    };
    match run(state, COMMAND, |generated| generated.route_need(input))? {
        RouteNeedOutcome::Routed { .. } => Ok(ExitCode::SUCCESS),
        RouteNeedOutcome::NotANeed { error } => refused(
            COMMAND,
            "NotANeed",
            &format!(
                "message {} is not a [NEED], so it opens no dispatch",
                error.message_id.0.0
            ),
        ),
        RouteNeedOutcome::NoSuchMessage { error } => no_message(COMMAND, &error.message_id),
        RouteNeedOutcome::WrongState { error } => answered(COMMAND, &error.message_id),
    }
}

/// `conductor message messages`: the view `conductor.dispatch.Messages`, served by the generated
/// query over the existing store under the state directory and rendered as `--format` asks, in the
/// order the messages were first stored. `--format jsonl` is the export of the message records;
/// two runs over one store give the same bytes. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn messages(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 10] = [
        "message_id",
        "state",
        "sender",
        "recipient",
        "kind",
        "first_line",
        "transport",
        "received_at",
        "body_path",
        "dispatch_id",
    ];

    /// The state's name as the specification spells it. Exhaustive, so a state the specification
    /// adds fails to compile here rather than rendering under a guessed name.
    fn state_name(state: MessageState) -> &'static str {
        match state {
            MessageState::Handled => "Handled",
            MessageState::Received => "Received",
            MessageState::Rejected => "Rejected",
        }
    }

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .messages()
        .map_err(|unmet| anyhow!("message messages: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.message_id.0.0),
                Value::from(state_name(row.state)),
                Value::from(row.sender),
                Value::from(row.recipient),
                Value::from(kind_name(row.kind)),
                Value::from(row.first_line),
                Value::from(transport_name(row.transport)),
                Value::from(row.received_at.0),
                Value::from(row.body_path),
                Value::from(row.dispatch_id.map(|dispatch| dispatch.0)),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "messages", &view.render(&COLUMNS, &rows))
}

/// The kind's name as the specification spells it. Exhaustive, so a kind the specification adds
/// fails to compile here rather than being read or rendered under a guessed name.
fn kind_name(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Dispatch => "Dispatch",
        MessageKind::Report => "Report",
        MessageKind::DecisionRequest => "DecisionRequest",
        MessageKind::Decision => "Decision",
        MessageKind::Need => "Need",
        MessageKind::Resource => "Resource",
        MessageKind::Escalation => "Escalation",
        MessageKind::Unknown => "Unknown",
    }
}

fn kind_named(name: &str) -> Option<MessageKind> {
    [
        MessageKind::Dispatch,
        MessageKind::Report,
        MessageKind::DecisionRequest,
        MessageKind::Decision,
        MessageKind::Need,
        MessageKind::Resource,
        MessageKind::Escalation,
        MessageKind::Unknown,
    ]
    .into_iter()
    .find(|kind| kind_name(*kind) == name)
}

/// The transport's name as the specification spells it. Exhaustive, so a transport the
/// specification adds fails to compile here rather than being read or rendered under a guessed
/// name.
fn transport_name(transport: Transport) -> &'static str {
    match transport {
        Transport::SendMessage => "SendMessage",
        Transport::Mailbox => "Mailbox",
    }
}

fn transport_named(name: &str) -> Option<Transport> {
    [Transport::SendMessage, Transport::Mailbox]
        .into_iter()
        .find(|transport| transport_name(*transport) == name)
}

/// The ports `ReceiveMessage` runs over: the store, and the message id it assigns.
struct Ports {
    store: Store,
}

impl MessageStorage for Ports {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        MessageStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        MessageStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        MessageStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        MessageStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Ok(MessageId(Uuid(eventlog_core::new_event_id())))
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

/// `--message-id`, which the command line has already read as a UUID.
fn message_id(command: &str, value: Option<String>) -> Result<MessageId> {
    Ok(MessageId(Uuid(flag(command, "--message-id", value)?)))
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

fn no_message(command: &str, message: &MessageId) -> Result<ExitCode> {
    refused(
        command,
        "MessageNotFound",
        &format!("no message has the id {}", message.0.0),
    )
}

fn answered(command: &str, message: &MessageId) -> Result<ExitCode> {
    refused(
        command,
        "MessageStateConflict",
        &format!("message {} was already handled or rejected", message.0.0),
    )
}

/// Writes `text` to standard output.
fn print(command: &str, text: &str) -> Result<ExitCode> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}
