//! `ReconciliationStorage` on [`super::Store`] (`story:daemon-reconcile`): each
//! `conductor.instance.Reconciliation` conductord records is one stream, keyed by its id, written
//! once when the daemon has taken the action, and read back as every other entity of the store is.

use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::config::InstanceName;
use conductor_model::instance::{
    ReconcileAction, ReconcileReason, ReconcileResult, ReconciliationData, ReconciliationId,
    ReconciliationSnapshot, ReconciliationState,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.instance.Reconciliation`.
const RECONCILIATION: &str = "conductor.instance.Reconciliation";

impl ReconciliationStorage for Store {
    fn get(&self, identity: &ReconciliationId) -> Option<ReconciliationSnapshot> {
        let key = &identity.0.0;
        let body = self.read(RECONCILIATION, key)?;
        match decode(&body) {
            Ok(found) if found.data.reconciliation_id == *identity => Some(found),
            Ok(found) => {
                self.fail(undecodable(
                    RECONCILIATION,
                    key,
                    format!(
                        "holds the reconciliation {:?}",
                        found.data.reconciliation_id.0.0
                    ),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(RECONCILIATION, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: ReconciliationSnapshot) {
        let body = encode(&snapshot);
        self.write(
            RECONCILIATION,
            &snapshot.data.reconciliation_id.0.0,
            STORED,
            body,
        );
    }

    fn delete(&mut self, identity: &ReconciliationId) {
        self.write(RECONCILIATION, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ReconciliationSnapshot> {
        self.rows(RECONCILIATION)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: RECONCILIATION,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

/// The action's name as the specification spells it.
#[must_use]
pub const fn action_name(action: ReconcileAction) -> &'static str {
    match action {
        ReconcileAction::Start => "Start",
        ReconcileAction::Stop => "Stop",
        ReconcileAction::Restart => "Restart",
        ReconcileAction::Upgrade => "Upgrade",
        ReconcileAction::Nothing => "Nothing",
        ReconcileAction::Purge => "Purge",
    }
}

/// The reason's name as the specification spells it.
#[must_use]
pub const fn reason_name(reason: ReconcileReason) -> &'static str {
    match reason {
        ReconcileReason::ConfigChange => "ConfigChange",
        ReconcileReason::LostRuntime => "LostRuntime",
        ReconcileReason::Cadence => "Cadence",
        ReconcileReason::OperatorCommand => "OperatorCommand",
        ReconcileReason::RunBound => "RunBound",
    }
}

/// The result's name as the specification spells it.
#[must_use]
pub const fn result_name(result: ReconcileResult) -> &'static str {
    match result {
        ReconcileResult::Succeeded => "Succeeded",
        ReconcileResult::Refused => "Refused",
        ReconcileResult::RolledBack => "RolledBack",
    }
}

fn action_named(name: &str) -> Option<ReconcileAction> {
    [
        ReconcileAction::Start,
        ReconcileAction::Stop,
        ReconcileAction::Restart,
        ReconcileAction::Upgrade,
        ReconcileAction::Nothing,
        ReconcileAction::Purge,
    ]
    .into_iter()
    .find(|action| action_name(*action) == name)
}

fn reason_named(name: &str) -> Option<ReconcileReason> {
    [
        ReconcileReason::ConfigChange,
        ReconcileReason::LostRuntime,
        ReconcileReason::Cadence,
        ReconcileReason::OperatorCommand,
        ReconcileReason::RunBound,
    ]
    .into_iter()
    .find(|reason| reason_name(*reason) == name)
}

fn result_named(name: &str) -> Option<ReconcileResult> {
    [
        ReconcileResult::Succeeded,
        ReconcileResult::Refused,
        ReconcileResult::RolledBack,
    ]
    .into_iter()
    .find(|result| result_name(*result) == name)
}

fn encode(found: &ReconciliationSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": "Recorded",
        "reconciliation_id": data.reconciliation_id.0.0,
        "instance": data.instance.0,
        "action": action_name(data.action),
        "reason": reason_name(data.reason),
        "result": result_name(data.result),
        "detail": data.detail,
        "at": data.at.0,
    })
}

fn decode(body: &Value) -> Result<ReconciliationSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Recorded" {
        return Err(format!("unknown Reconciliation state {state:?}"));
    }
    let action = text(body, "action")?;
    let reason = text(body, "reason")?;
    let result = text(body, "result")?;
    Ok(ReconciliationSnapshot {
        state: ReconciliationState::Recorded,
        data: ReconciliationData {
            reconciliation_id: ReconciliationId(Uuid(text(body, "reconciliation_id")?)),
            instance: InstanceName(text(body, "instance")?),
            action: action_named(&action)
                .ok_or_else(|| format!("unknown ReconcileAction {action:?}"))?,
            reason: reason_named(&reason)
                .ok_or_else(|| format!("unknown ReconcileReason {reason:?}"))?,
            result: result_named(&result)
                .ok_or_else(|| format!("unknown ReconcileResult {result:?}"))?,
            detail: body
                .get("detail")
                .and_then(Value::as_str)
                .map(str::to_owned),
            actor: crate::actor::stored(body)?,
            at: Timestamp(text(body, "at")?),
        },
    })
}
