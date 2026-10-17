//! `RunTickStorage` on [`super::Store`] (`story:run-daemon`): each `conductor.session.RunTick`
//! `conductor run` records is one stream, keyed by its id, written once at the end of its tick and
//! read back as every other entity of the store is. A tick's `actions` are kept as the events the
//! records' dispatch log spells (`handover-sent`, `relayed`, …), so a tick and the log read alike.

use conductor_model::behaviour::RunTickStorage;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::session::{RunTickData, RunTickSnapshot, RunTickState, TickId, TickKind};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};
use crate::records::{dispatch_event, dispatch_event_named};

/// `conductor.session.RunTick`.
const RUN_TICK: &str = "conductor.session.RunTick";

impl RunTickStorage for Store {
    fn get(&self, identity: &TickId) -> Option<RunTickSnapshot> {
        let key = &identity.0.0;
        let body = self.read(RUN_TICK, key)?;
        match decode(&body) {
            Ok(tick) if tick.data.tick_id == *identity => Some(tick),
            Ok(tick) => {
                self.fail(undecodable(
                    RUN_TICK,
                    key,
                    format!("holds the tick {:?}", tick.data.tick_id.0.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(RUN_TICK, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: RunTickSnapshot) {
        let body = encode(&snapshot);
        self.write(RUN_TICK, &snapshot.data.tick_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &TickId) {
        self.write(RUN_TICK, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<RunTickSnapshot> {
        self.rows(RUN_TICK)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(tick) => Some(tick),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: RUN_TICK,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

/// The kind's name as the specification spells it. Exhaustive, so a kind the specification adds
/// fails to compile here rather than being written under a guessed name.
#[must_use]
pub const fn kind_name(kind: TickKind) -> &'static str {
    match kind {
        TickKind::Watch => "watch",
        TickKind::Cycle => "cycle",
        TickKind::Daily => "daily",
    }
}

/// The kind `name` spells, if any.
#[must_use]
pub fn kind_named(name: &str) -> Option<TickKind> {
    [TickKind::Watch, TickKind::Cycle, TickKind::Daily]
        .into_iter()
        .find(|kind| kind_name(*kind) == name)
}

/// The state's name as the specification spells it.
#[must_use]
pub const fn state_name(state: RunTickState) -> &'static str {
    match state {
        RunTickState::Recorded => "Recorded",
    }
}

fn encode(tick: &RunTickSnapshot) -> Value {
    let data = &tick.data;
    json!({
        "state": state_name(tick.state),
        "tick_id": data.tick_id.0.0,
        "kind": kind_name(data.kind),
        "started_at": data.started_at.0,
        "lines_handled": data.lines_handled,
        "actions": data.actions.iter().map(|event| dispatch_event(*event)).collect::<Vec<_>>(),
    })
}

fn decode(body: &Value) -> Result<RunTickSnapshot, String> {
    let state = text(body, "state")?;
    if state != state_name(RunTickState::Recorded) {
        return Err(format!("unknown RunTick state {state:?}"));
    }
    let kind = text(body, "kind")?;
    Ok(RunTickSnapshot {
        state: RunTickState::Recorded,
        data: RunTickData {
            tick_id: TickId(Uuid(text(body, "tick_id")?)),
            kind: kind_named(&kind).ok_or_else(|| format!("unknown TickKind {kind:?}"))?,
            started_at: Timestamp(text(body, "started_at")?),
            lines_handled: body
                .get("lines_handled")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"lines_handled\"".to_owned())?,
            actions: body
                .get("actions")
                .and_then(Value::as_array)
                .ok_or_else(|| "no list field \"actions\"".to_owned())?
                .iter()
                .map(|event| {
                    let name = event.as_str().ok_or("an action is not text")?;
                    dispatch_event_named(name).ok_or_else(|| format!("unknown action {name:?}"))
                })
                .collect::<Result<_, String>>()?,
        },
    })
}
