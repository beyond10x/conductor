//! `BoxStorage` and `BoxRefusalStorage` on [`super::Store`] (`story:placement-substrate-local`):
//! each `conductor.placement.Box` conductord opens and each `conductor.placement.BoxRefusal`
//! substrate answered is one stream, keyed by its box id.

use conductor_model::behaviour::{BoxRefusalStorage, BoxStorage};
use conductor_model::config::InstanceName;
use conductor_model::placement::{
    BoxData, BoxId, BoxRefusalData, BoxRefusalSnapshot, BoxRefusalState, BoxSnapshot, BoxState,
    PlacementName, RefusalReason,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.placement.Box`.
const BOX: &str = "conductor.placement.Box";

/// `conductor.placement.BoxRefusal`.
const REFUSAL: &str = "conductor.placement.BoxRefusal";

fn box_state_name(state: BoxState) -> &'static str {
    match state {
        BoxState::Opened => "Opened",
        BoxState::Closed => "Closed",
    }
}

fn encode_box(found: &BoxSnapshot) -> Value {
    let data = &found.data;
    json!({
        "state": box_state_name(found.state),
        "box_id": data.box_id.0.0,
        "placement": data.placement.0,
        "instance": data.instance.0,
        "actor": crate::actor::to_json(&data.actor),
        "opened_at": data.opened_at.0,
        "closed_at": data.closed_at.as_ref().map(|at| at.0.clone()),
        "closed_by": data.closed_by.as_ref().map(crate::actor::to_json),
    })
}

fn decode_box(body: &Value) -> Result<BoxSnapshot, String> {
    let state = match text(body, "state")?.as_str() {
        "Opened" => BoxState::Opened,
        "Closed" => BoxState::Closed,
        other => return Err(format!("unknown Box state {other:?}")),
    };
    let closed_by = match body.get("closed_by") {
        Some(Value::Null) | None => None,
        Some(actor) => Some(crate::actor::stored(&json!({ "actor": actor }))?),
    };
    Ok(BoxSnapshot {
        state,
        data: BoxData {
            box_id: BoxId(Uuid(text(body, "box_id")?)),
            placement: PlacementName(text(body, "placement")?),
            instance: InstanceName(text(body, "instance")?),
            actor: crate::actor::stored(body)?,
            opened_at: Timestamp(text(body, "opened_at")?),
            closed_at: body
                .get("closed_at")
                .and_then(Value::as_str)
                .map(|at| Timestamp(at.to_owned())),
            closed_by,
        },
    })
}

fn encode_refusal(found: &BoxRefusalSnapshot) -> Value {
    let data = &found.data;
    json!({
        "state": "Refused",
        "box_id": data.box_id.0.0,
        "placement": data.placement.0,
        "instance": data.instance.0,
        "actor": crate::actor::to_json(&data.actor),
        "reason": data.reason.0,
        "refused_at": data.refused_at.0,
    })
}

fn decode_refusal(body: &Value) -> Result<BoxRefusalSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Refused" {
        return Err(format!("unknown BoxRefusal state {state:?}"));
    }
    Ok(BoxRefusalSnapshot {
        state: BoxRefusalState::Refused,
        data: BoxRefusalData {
            box_id: BoxId(Uuid(text(body, "box_id")?)),
            placement: PlacementName(text(body, "placement")?),
            instance: InstanceName(text(body, "instance")?),
            actor: crate::actor::stored(body)?,
            reason: RefusalReason(text(body, "reason")?),
            refused_at: Timestamp(text(body, "refused_at")?),
        },
    })
}

/// Every decodable row of `entity`, each undecodable one reported.
fn listed<T>(
    store: &Store,
    entity: &'static str,
    decode: fn(&Value) -> Result<T, String>,
) -> Vec<T> {
    store
        .rows(entity)
        .into_iter()
        .filter_map(|(stream, body)| match decode(&body) {
            Ok(found) => Some(found),
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

impl BoxStorage for Store {
    fn get(&self, identity: &BoxId) -> Option<BoxSnapshot> {
        let key = &identity.0.0;
        let body = self.read(BOX, key)?;
        match decode_box(&body) {
            Ok(found) => Some(found),
            Err(detail) => {
                self.fail(undecodable(BOX, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: BoxSnapshot) {
        let body = encode_box(&snapshot);
        self.write(BOX, &snapshot.data.box_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &BoxId) {
        self.write(BOX, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<BoxSnapshot> {
        listed(self, BOX, decode_box)
    }
}

impl BoxRefusalStorage for Store {
    fn get(&self, identity: &BoxId) -> Option<BoxRefusalSnapshot> {
        let key = &identity.0.0;
        let body = self.read(REFUSAL, key)?;
        match decode_refusal(&body) {
            Ok(found) => Some(found),
            Err(detail) => {
                self.fail(undecodable(REFUSAL, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: BoxRefusalSnapshot) {
        let body = encode_refusal(&snapshot);
        self.write(REFUSAL, &snapshot.data.box_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &BoxId) {
        self.write(REFUSAL, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<BoxRefusalSnapshot> {
        listed(self, REFUSAL, decode_refusal)
    }
}
