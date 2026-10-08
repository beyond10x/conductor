//! `DispatchStorage` and `MessageStorage` on [`super::Store`] (`story:decision-commands`, wave 06,
//! U2): each `conductor.dispatch.Dispatch` and each `conductor.dispatch.Message` is one stream,
//! keyed by its identity, kept and read back as every other entity of the store is.

use conductor_model::behaviour::{DispatchStorage, MessageStorage};
use conductor_model::direction::GoalId;
use conductor_model::dispatch::{
    DispatchData, DispatchId, DispatchSnapshot, DispatchState, MessageData, MessageId, MessageKind,
    MessageSnapshot, MessageState, Transport,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.dispatch.Dispatch`.
const DISPATCH: &str = "conductor.dispatch.Dispatch";

/// `conductor.dispatch.Message`.
const MESSAGE: &str = "conductor.dispatch.Message";

/// The instance `key` of `entity` in `store`, decoded, when the store holds it under that key. A
/// record that does not decode, or that holds another instance, is kept for [`Store::check`].
fn decoded<T>(
    store: &Store,
    entity: &'static str,
    key: &str,
    decode: fn(&Value) -> Result<T, String>,
    held: fn(&T) -> &str,
) -> Option<T> {
    let body = store.read(entity, key)?;
    match decode(&body) {
        Ok(instance) if held(&instance) == key => Some(instance),
        Ok(instance) => {
            store.fail(undecodable(
                entity,
                key,
                format!("holds {:?}", held(&instance)),
            ));
            None
        }
        Err(detail) => {
            store.fail(undecodable(entity, key, detail));
            None
        }
    }
}

/// Every instance of `entity` in `store`, decoded, in the order they were first stored.
fn listed<T>(
    store: &Store,
    entity: &'static str,
    decode: fn(&Value) -> Result<T, String>,
) -> Vec<T> {
    store
        .rows(entity)
        .into_iter()
        .filter_map(|(stream, body)| match decode(&body) {
            Ok(instance) => Some(instance),
            Err(detail) => {
                store.fail(StoreError::Undecodable {
                    entity,
                    stream,
                    detail,
                });
                None
            }
        })
        .collect()
}

impl DispatchStorage for Store {
    fn get(&self, identity: &DispatchId) -> Option<DispatchSnapshot> {
        decoded(self, DISPATCH, &identity.0, decode_dispatch, |dispatch| {
            &dispatch.data.dispatch_id.0
        })
    }

    fn put(&mut self, snapshot: DispatchSnapshot) {
        let body = encode_dispatch(&snapshot);
        self.write(DISPATCH, &snapshot.data.dispatch_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &DispatchId) {
        self.write(DISPATCH, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<DispatchSnapshot> {
        listed(self, DISPATCH, decode_dispatch)
    }
}

fn encode_dispatch(dispatch: &DispatchSnapshot) -> Value {
    let data = &dispatch.data;
    json!({
        "state": dispatch_state_name(dispatch.state),
        "dispatch_id": data.dispatch_id.0,
        "repository": data.repository.0,
        "goal_id": data.goal_id.0,
        "brief": data.brief,
        "sent_at": data.sent_at.0,
        "priority": data.priority,
        "waits_on": data.waits_on.as_ref().map(|waits_on| &waits_on.0),
    })
}

fn decode_dispatch(body: &Value) -> Result<DispatchSnapshot, String> {
    let state = text(body, "state")?;
    Ok(DispatchSnapshot {
        state: dispatch_state_named(&state)
            .ok_or_else(|| format!("unknown Dispatch state {state:?}"))?,
        data: DispatchData {
            dispatch_id: DispatchId(text(body, "dispatch_id")?),
            repository: RepositoryName(text(body, "repository")?),
            goal_id: GoalId(text(body, "goal_id")?),
            brief: text(body, "brief")?,
            sent_at: Timestamp(text(body, "sent_at")?),
            priority: body
                .get("priority")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"priority\"".to_owned())?,
            waits_on: optional_text(body, "waits_on")?.map(DispatchId),
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn dispatch_state_name(state: DispatchState) -> &'static str {
    match state {
        DispatchState::Blocked => "Blocked",
        DispatchState::Cancelled => "Cancelled",
        DispatchState::Done => "Done",
        DispatchState::Failed => "Failed",
        DispatchState::Sent => "Sent",
        DispatchState::Started => "Started",
    }
}

fn dispatch_state_named(name: &str) -> Option<DispatchState> {
    [
        DispatchState::Blocked,
        DispatchState::Cancelled,
        DispatchState::Done,
        DispatchState::Failed,
        DispatchState::Sent,
        DispatchState::Started,
    ]
    .into_iter()
    .find(|state| dispatch_state_name(*state) == name)
}

impl MessageStorage for Store {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        decoded(self, MESSAGE, &identity.0.0, decode_message, |message| {
            &message.data.message_id.0.0
        })
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        let body = encode_message(&snapshot);
        self.write(MESSAGE, &snapshot.data.message_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &MessageId) {
        self.write(MESSAGE, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        listed(self, MESSAGE, decode_message)
    }
}

fn encode_message(message: &MessageSnapshot) -> Value {
    let data = &message.data;
    json!({
        "state": message_state_name(message.state),
        "message_id": data.message_id.0.0,
        "sender": data.sender,
        "recipient": data.recipient,
        "kind": kind_name(data.kind),
        "first_line": data.first_line,
        "transport": transport_name(data.transport),
        "received_at": data.received_at.0,
        "body_path": data.body_path,
        "dispatch_id": data.dispatch_id.as_ref().map(|dispatch| &dispatch.0),
    })
}

fn decode_message(body: &Value) -> Result<MessageSnapshot, String> {
    let state = text(body, "state")?;
    let kind = text(body, "kind")?;
    let transport = text(body, "transport")?;
    Ok(MessageSnapshot {
        state: message_state_named(&state)
            .ok_or_else(|| format!("unknown Message state {state:?}"))?,
        data: MessageData {
            message_id: MessageId(Uuid(text(body, "message_id")?)),
            sender: text(body, "sender")?,
            recipient: text(body, "recipient")?,
            kind: kind_named(&kind).ok_or_else(|| format!("unknown MessageKind {kind:?}"))?,
            first_line: text(body, "first_line")?,
            transport: transport_named(&transport)
                .ok_or_else(|| format!("unknown Transport {transport:?}"))?,
            received_at: Timestamp(text(body, "received_at")?),
            body_path: optional_text(body, "body_path")?,
            dispatch_id: optional_text(body, "dispatch_id")?.map(DispatchId),
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn message_state_name(state: MessageState) -> &'static str {
    match state {
        MessageState::Handled => "Handled",
        MessageState::Received => "Received",
        MessageState::Rejected => "Rejected",
    }
}

fn message_state_named(name: &str) -> Option<MessageState> {
    [
        MessageState::Handled,
        MessageState::Received,
        MessageState::Rejected,
    ]
    .into_iter()
    .find(|state| message_state_name(*state) == name)
}

/// The kind's name as the specification spells it. Exhaustive, so a kind the specification adds
/// fails to compile here rather than being written under a guessed name.
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
/// specification adds fails to compile here rather than being written under a guessed name.
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

/// The text of an optional field: `null` is absent, and the field itself is always written.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        _ => Err(format!("no optional text field {field:?}")),
    }
}
