//! `DecisionStorage` and `DecisionRequestStorage` on [`super::Store`] (`story:decision-commands`),
//! kept the way `impl GoalStorage for Store` keeps a goal: one stream per instance, keyed by its
//! identity, holding the instance's snapshot after each write.

use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage};
use conductor_model::decision::{
    Decider, DecisionClass, DecisionData, DecisionId, DecisionRequestData, DecisionRequestSnapshot,
    DecisionRequestState, DecisionSnapshot, DecisionState, RequestId,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.decision.Decision`.
const DECISION: &str = "conductor.decision.Decision";

/// `conductor.decision.DecisionRequest`.
const DECISION_REQUEST: &str = "conductor.decision.DecisionRequest";

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

impl DecisionStorage for Store {
    fn get(&self, identity: &DecisionId) -> Option<DecisionSnapshot> {
        decoded(self, DECISION, &identity.0, decode_decision, |decision| {
            &decision.data.decision_id.0
        })
    }

    fn put(&mut self, snapshot: DecisionSnapshot) {
        let body = encode_decision(&snapshot);
        self.write(DECISION, &snapshot.data.decision_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &DecisionId) {
        self.write(DECISION, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<DecisionSnapshot> {
        listed(self, DECISION, decode_decision)
    }
}

fn encode_decision(decision: &DecisionSnapshot) -> Value {
    let data = &decision.data;
    json!({
        "state": decision_state_name(decision.state),
        "decision_id": data.decision_id.0,
        "decision_class": class_name(data.decision_class),
        "decided_by": decider_name(data.decided_by),
        "question": data.question,
        "options": data.options,
        "choice": data.choice,
        "reason": data.reason,
        "evidence": data.evidence,
        "decided_at": data.decided_at.0,
    })
}

fn decode_decision(body: &Value) -> Result<DecisionSnapshot, String> {
    Ok(DecisionSnapshot {
        state: named(body, "state", decision_state_named)?,
        data: DecisionData {
            decision_id: DecisionId(text(body, "decision_id")?),
            decision_class: named(body, "decision_class", class_named)?,
            decided_by: named(body, "decided_by", decider_named)?,
            question: text(body, "question")?,
            options: text(body, "options")?,
            choice: text(body, "choice")?,
            reason: text(body, "reason")?,
            evidence: text(body, "evidence")?,
            decided_at: Timestamp(text(body, "decided_at")?),
        },
    })
}

impl DecisionRequestStorage for Store {
    fn get(&self, identity: &RequestId) -> Option<DecisionRequestSnapshot> {
        decoded(
            self,
            DECISION_REQUEST,
            &identity.0,
            decode_request,
            |request| &request.data.request_id.0,
        )
    }

    fn put(&mut self, snapshot: DecisionRequestSnapshot) {
        let body = encode_request(&snapshot);
        self.write(DECISION_REQUEST, &snapshot.data.request_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &RequestId) {
        self.write(DECISION_REQUEST, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<DecisionRequestSnapshot> {
        listed(self, DECISION_REQUEST, decode_request)
    }
}

fn encode_request(request: &DecisionRequestSnapshot) -> Value {
    let data = &request.data;
    json!({
        "state": request_state_name(request.state),
        "request_id": data.request_id.0,
        "repository": data.repository.0,
        "question": data.question,
        "options": data.options,
        "recommendation": data.recommendation,
        "raised_at": data.raised_at.0,
        "blocker": data.blocker,
        "escalation_class": data.escalation_class.map(class_name),
        "escalation_reason": data.escalation_reason,
        "decision_id": data.decision_id.as_ref().map(|id| &id.0),
    })
}

fn decode_request(body: &Value) -> Result<DecisionRequestSnapshot, String> {
    Ok(DecisionRequestSnapshot {
        state: named(body, "state", request_state_named)?,
        data: DecisionRequestData {
            request_id: RequestId(text(body, "request_id")?),
            repository: RepositoryName(text(body, "repository")?),
            question: text(body, "question")?,
            options: text(body, "options")?,
            recommendation: text(body, "recommendation")?,
            raised_at: Timestamp(text(body, "raised_at")?),
            blocker: optional_text(body, "blocker")?,
            escalation_class: optional_text(body, "escalation_class")?
                .map(|class| {
                    class_named(&class).ok_or_else(|| format!("unknown DecisionClass {class:?}"))
                })
                .transpose()?,
            escalation_reason: optional_text(body, "escalation_reason")?,
            decision_id: optional_text(body, "decision_id")?.map(DecisionId),
        },
    })
}

/// The value `field` names through `named`, which answers `None` for a name it does not know.
fn named<T>(body: &Value, field: &str, named: fn(&str) -> Option<T>) -> Result<T, String> {
    let name = text(body, field)?;
    named(&name).ok_or_else(|| format!("unknown {field} {name:?}"))
}

/// A text field that may be `null`; a field that is absent or holds anything else does not decode.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        _ => Err(format!("no text or null field {field:?}")),
    }
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn decision_state_name(state: DecisionState) -> &'static str {
    match state {
        DecisionState::InForce => "InForce",
        DecisionState::Reversed => "Reversed",
    }
}

fn decision_state_named(name: &str) -> Option<DecisionState> {
    [DecisionState::InForce, DecisionState::Reversed]
        .into_iter()
        .find(|state| decision_state_name(*state) == name)
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn request_state_name(state: DecisionRequestState) -> &'static str {
    match state {
        DecisionRequestState::Answered => "Answered",
        DecisionRequestState::Escalated => "Escalated",
        DecisionRequestState::Open => "Open",
        DecisionRequestState::Withdrawn => "Withdrawn",
    }
}

fn request_state_named(name: &str) -> Option<DecisionRequestState> {
    [
        DecisionRequestState::Answered,
        DecisionRequestState::Escalated,
        DecisionRequestState::Open,
        DecisionRequestState::Withdrawn,
    ]
    .into_iter()
    .find(|state| request_state_name(*state) == name)
}

/// The class's name as the specification spells it. Exhaustive, so a class the specification adds
/// fails to compile here rather than being written under a guessed name.
fn class_name(class: DecisionClass) -> &'static str {
    match class {
        DecisionClass::C => "C",
        DecisionClass::O => "O",
        DecisionClass::H => "H",
    }
}

fn class_named(name: &str) -> Option<DecisionClass> {
    [DecisionClass::C, DecisionClass::O, DecisionClass::H]
        .into_iter()
        .find(|class| class_name(*class) == name)
}

/// The decider's name as the specification spells it. Exhaustive, so a decider the specification
/// adds fails to compile here rather than being written under a guessed name.
fn decider_name(decider: Decider) -> &'static str {
    match decider {
        Decider::Conductor => "Conductor",
        Decider::Operator => "Operator",
    }
}

fn decider_named(name: &str) -> Option<Decider> {
    [Decider::Conductor, Decider::Operator]
        .into_iter()
        .find(|decider| decider_name(*decider) == name)
}
