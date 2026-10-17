//! `ToolCallStorage` on [`super::Store`] (`story:tool-broker-read`): each `conductor.tool.ToolCall`
//! the broker records is one stream, keyed by its id, written once and read back as every other
//! entity of the store is. It outlives the session that made the call.

use conductor_model::behaviour::ToolCallStorage;
use conductor_model::config::{InstanceName, SessionName};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::tool::{
    CallOutcome, ToolCallData, ToolCallId, ToolCallSnapshot, ToolCallState, Verb,
};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.tool.ToolCall`.
const TOOL_CALL: &str = "conductor.tool.ToolCall";

impl ToolCallStorage for Store {
    fn get(&self, identity: &ToolCallId) -> Option<ToolCallSnapshot> {
        let key = &identity.0.0;
        let body = self.read(TOOL_CALL, key)?;
        match decode(&body) {
            Ok(call) if call.data.tool_call_id == *identity => Some(call),
            Ok(call) => {
                self.fail(undecodable(
                    TOOL_CALL,
                    key,
                    format!("holds the call {:?}", call.data.tool_call_id.0.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(TOOL_CALL, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: ToolCallSnapshot) {
        let body = encode(&snapshot);
        self.write(TOOL_CALL, &snapshot.data.tool_call_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &ToolCallId) {
        self.write(TOOL_CALL, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ToolCallSnapshot> {
        self.rows(TOOL_CALL)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(call) => Some(call),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: TOOL_CALL,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

/// A verb as the specification spells it. Exhaustive, so a verb the specification adds fails to
/// compile here rather than being written under a guessed name.
pub(crate) fn verb_name(verb: Verb) -> &'static str {
    match verb {
        Verb::Show => "Show",
        Verb::Search => "Search",
        Verb::Diff => "Diff",
        Verb::Test => "Test",
        Verb::Check => "Check",
        Verb::Fmt => "Fmt",
        Verb::Commit => "Commit",
        Verb::Send => "Send",
        Verb::Report => "Report",
    }
}

/// An outcome as the specification spells it: [`conductor_broker::outcome_word`], exhaustive.
pub(crate) fn outcome_name(outcome: CallOutcome) -> &'static str {
    conductor_broker::outcome_word(outcome)
}

/// Every verb and every outcome a record may hold: the broker's own lists.
const VERBS: [Verb; conductor_broker::VERBS.len()] = conductor_broker::VERBS;
const OUTCOMES: [CallOutcome; conductor_broker::OUTCOMES.len()] = conductor_broker::OUTCOMES;

fn encode(call: &ToolCallSnapshot) -> Value {
    let data = &call.data;
    json!({
        "state": match call.state {
            ToolCallState::Recorded => "Recorded",
        },
        "tool_call_id": data.tool_call_id.0.0,
        "instance": data.instance.0,
        "session": data.session.0,
        "verb": verb_name(data.verb),
        "paths": data.paths,
        "outcome": outcome_name(data.outcome),
        "rule": data.rule,
        "duration_ms": data.duration_ms,
        "output_bytes": data.output_bytes,
        "called_at": data.called_at.0,
    })
}

fn decode(body: &Value) -> Result<ToolCallSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Recorded" {
        return Err(format!("unknown ToolCall state {state:?}"));
    }
    let verb = text(body, "verb")?;
    let outcome = text(body, "outcome")?;
    let integer = |field: &str| {
        body.get(field)
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("no integer field {field:?}"))
    };
    Ok(ToolCallSnapshot {
        state: ToolCallState::Recorded,
        data: ToolCallData {
            tool_call_id: ToolCallId(Uuid(text(body, "tool_call_id")?)),
            instance: InstanceName(text(body, "instance")?),
            session: SessionName(text(body, "session")?),
            verb: VERBS
                .into_iter()
                .find(|known| verb_name(*known) == verb)
                .ok_or_else(|| format!("unknown verb {verb:?}"))?,
            paths: body
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| "no list field \"paths\"".to_owned())?
                .iter()
                .map(|path| {
                    path.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "a path is not text".to_owned())
                })
                .collect::<Result<_, _>>()?,
            outcome: OUTCOMES
                .into_iter()
                .find(|known| outcome_name(*known) == outcome)
                .ok_or_else(|| format!("unknown outcome {outcome:?}"))?,
            rule: match body.get("rule") {
                Some(Value::Null) | None => None,
                Some(Value::String(rule)) => Some(rule.clone()),
                Some(_) => return Err("no text or null field \"rule\"".to_owned()),
            },
            duration_ms: integer("duration_ms")?,
            output_bytes: integer("output_bytes")?,
            called_at: Timestamp(text(body, "called_at")?),
        },
    })
}
