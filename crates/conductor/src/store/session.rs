//! `LeaseStorage` on [`super::Store`] (`story:single-conductor-lease`): each
//! `conductor.session.Lease` is one stream, keyed by its instance's name, kept and read back as
//! every other entity of the store is. Every claim that changes the lease appends its new
//! snapshot, so the stream is the lease's history: who took it, and from whom.

use conductor_model::behaviour::LeaseStorage;
use conductor_model::config::InstanceName;
use conductor_model::primitives::Timestamp;
use conductor_model::session::{LeaseData, LeaseSnapshot, LeaseState, SessionId};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.session.Lease`.
const LEASE: &str = "conductor.session.Lease";

impl LeaseStorage for Store {
    fn get(&self, identity: &InstanceName) -> Option<LeaseSnapshot> {
        let key = &identity.0;
        let body = self.read(LEASE, key)?;
        match decode(&body) {
            Ok(lease) if lease.data.instance == *identity => Some(lease),
            Ok(lease) => {
                self.fail(undecodable(
                    LEASE,
                    key,
                    format!("holds the lease of instance {:?}", lease.data.instance.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(LEASE, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: LeaseSnapshot) {
        let body = encode(&snapshot);
        self.write(LEASE, &snapshot.data.instance.0, STORED, body);
    }

    fn delete(&mut self, identity: &InstanceName) {
        self.write(LEASE, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<LeaseSnapshot> {
        self.rows(LEASE)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(lease) => Some(lease),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: LEASE,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(lease: &LeaseSnapshot) -> Value {
    let data = &lease.data;
    json!({
        "state": state_name(lease.state),
        "instance": data.instance.0,
        "session_id": data.session_id.0,
        "pid": data.pid,
        "taken_at": data.taken_at.0,
        "taken_over_session_id": data.taken_over_session_id.as_ref().map(|id| &id.0),
        "taken_over_pid": data.taken_over_pid,
    })
}

fn decode(body: &Value) -> Result<LeaseSnapshot, String> {
    let state = text(body, "state")?;
    Ok(LeaseSnapshot {
        state: state_named(&state).ok_or_else(|| format!("unknown Lease state {state:?}"))?,
        data: LeaseData {
            instance: InstanceName(text(body, "instance")?),
            session_id: SessionId(text(body, "session_id")?),
            pid: body
                .get("pid")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"pid\"".to_owned())?,
            taken_at: Timestamp(text(body, "taken_at")?),
            taken_over_session_id: match body.get("taken_over_session_id") {
                Some(Value::Null) => None,
                Some(Value::String(id)) => Some(SessionId(id.clone())),
                _ => return Err("no text or null field \"taken_over_session_id\"".to_owned()),
            },
            taken_over_pid: match body.get("taken_over_pid") {
                Some(Value::Null) => None,
                Some(pid) => Some(
                    pid.as_i64()
                        .ok_or_else(|| "no integer or null field \"taken_over_pid\"".to_owned())?,
                ),
                None => return Err("no integer or null field \"taken_over_pid\"".to_owned()),
            },
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn state_name(state: LeaseState) -> &'static str {
    match state {
        LeaseState::Held => "Held",
    }
}

fn state_named(name: &str) -> Option<LeaseState> {
    [LeaseState::Held]
        .into_iter()
        .find(|state| state_name(*state) == name)
}
