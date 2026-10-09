//! `ControllerStorage` on [`super::Store`]: each `conductor.dispatch.Controller` is one stream,
//! keyed by its `controller_id`, kept and read back as every other entity of the store is.

use conductor_model::behaviour::ControllerStorage;
use conductor_model::config::SessionName;
use conductor_model::dispatch::{
    ControllerData, ControllerId, ControllerSnapshot, ControllerState,
};
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::primitives::Uuid;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.dispatch.Controller`.
const CONTROLLER: &str = "conductor.dispatch.Controller";

impl ControllerStorage for Store {
    fn get(&self, identity: &ControllerId) -> Option<ControllerSnapshot> {
        let key = &identity.0.0;
        let body = self.read(CONTROLLER, key)?;
        match decode(&body) {
            Ok(controller) if controller.data.controller_id == *identity => Some(controller),
            Ok(controller) => {
                self.fail(undecodable(
                    CONTROLLER,
                    key,
                    format!("holds controller {:?}", controller.data.controller_id.0.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(CONTROLLER, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: ControllerSnapshot) {
        let body = encode(&snapshot);
        self.write(CONTROLLER, &snapshot.data.controller_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &ControllerId) {
        self.write(CONTROLLER, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ControllerSnapshot> {
        self.rows(CONTROLLER)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(controller) => Some(controller),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: CONTROLLER,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(controller: &ControllerSnapshot) -> Value {
    let data = &controller.data;
    json!({
        "state": state_name(controller.state),
        "controller_id": data.controller_id.0.0,
        "repository": data.repository.0,
        "harness": harness_name(data.harness),
        "session_name": data.session_name.0,
        "charter_revision": data.charter_revision,
    })
}

fn decode(body: &Value) -> Result<ControllerSnapshot, String> {
    let state = text(body, "state")?;
    let harness = text(body, "harness")?;
    Ok(ControllerSnapshot {
        state: state_named(&state).ok_or_else(|| format!("unknown Controller state {state:?}"))?,
        data: ControllerData {
            controller_id: ControllerId(Uuid(text(body, "controller_id")?)),
            repository: RepositoryName(text(body, "repository")?),
            harness: harness_named(&harness)
                .ok_or_else(|| format!("unknown Harness {harness:?}"))?,
            session_name: SessionName(text(body, "session_name")?),
            charter_revision: body
                .get("charter_revision")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"charter_revision\"".to_owned())?,
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn state_name(state: ControllerState) -> &'static str {
    match state {
        ControllerState::Paused => "Paused",
        ControllerState::Running => "Running",
        ControllerState::Stopped => "Stopped",
    }
}

fn state_named(name: &str) -> Option<ControllerState> {
    [
        ControllerState::Paused,
        ControllerState::Running,
        ControllerState::Stopped,
    ]
    .into_iter()
    .find(|state| state_name(*state) == name)
}

/// The harness's name as the specification spells it. Exhaustive, so a harness the specification
/// adds fails to compile here rather than being written under a guessed name.
fn harness_name(harness: Harness) -> &'static str {
    match harness {
        Harness::Claude => "Claude",
        Harness::Codex => "Codex",
    }
}

fn harness_named(name: &str) -> Option<Harness> {
    [Harness::Claude, Harness::Codex]
        .into_iter()
        .find(|harness| harness_name(*harness) == name)
}
