//! `HostGrantStorage` on [`super::Store`] (`story:build-run`): each `conductor.host.HostGrant` is
//! one stream, keyed by its grant id, kept and read back as every other entity of the store is.
//! Every move of a grant appends its new snapshot, so the stream is the grant's history: asked,
//! answered, started, its peak, released.

use conductor_model::behaviour::HostGrantStorage;
use conductor_model::config::{InstanceName, SessionName};
use conductor_model::host::{GrantId, HostGrantData, HostGrantSnapshot};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use conductor_model::session::SessionId;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};
use crate::host::{kind_name, kind_named, reason_name, reason_named, state_name, state_named};

/// `conductor.host.HostGrant`.
const HOST_GRANT: &str = "conductor.host.HostGrant";

impl HostGrantStorage for Store {
    fn get(&self, identity: &GrantId) -> Option<HostGrantSnapshot> {
        let key = &identity.0;
        let body = self.read(HOST_GRANT, key)?;
        match decode(&body) {
            Ok(grant) if grant.data.grant_id == *identity => Some(grant),
            Ok(grant) => {
                self.fail(undecodable(
                    HOST_GRANT,
                    key,
                    format!("holds grant {:?}", grant.data.grant_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(HOST_GRANT, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: HostGrantSnapshot) {
        let body = encode(&snapshot);
        self.write(HOST_GRANT, &snapshot.data.grant_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &GrantId) {
        self.write(HOST_GRANT, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<HostGrantSnapshot> {
        self.rows(HOST_GRANT)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(grant) => Some(grant),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: HOST_GRANT,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(grant: &HostGrantSnapshot) -> Value {
    let data = &grant.data;
    json!({
        "state": state_name(grant.state),
        "grant_id": data.grant_id.0,
        "instance": data.instance.0,
        "repository": data.repository.as_ref().map(|repository| &repository.0),
        "kind": kind_name(data.kind),
        "bytes": data.bytes,
        "peak_bytes": data.peak_bytes,
        "pauses": data.pauses,
        "command": data.command,
        "process_group": data.process_group,
        "session_name": data.session_name.as_ref().map(|name| &name.0),
        "session_id": data.session_id.as_ref().map(|id| &id.0),
        "reclaimed_bytes": data.reclaimed_bytes,
        "shortfall_bytes": data.shortfall_bytes,
        "requested_at": data.requested_at.0,
        "started_at": data.started_at.as_ref().map(|at| &at.0),
        "released_at": data.released_at.as_ref().map(|at| &at.0),
        "release_reason": data.release_reason.map(reason_name),
    })
}

fn decode(body: &Value) -> Result<HostGrantSnapshot, String> {
    let state = text(body, "state")?;
    let kind = text(body, "kind")?;
    Ok(HostGrantSnapshot {
        state: state_named(&state).ok_or_else(|| format!("unknown HostGrant state {state:?}"))?,
        data: HostGrantData {
            grant_id: GrantId(text(body, "grant_id")?),
            instance: InstanceName(text(body, "instance")?),
            repository: optional_text(body, "repository")?.map(RepositoryName),
            kind: kind_named(&kind).ok_or_else(|| format!("unknown GrantKind {kind:?}"))?,
            bytes: optional_integer(body, "bytes")?,
            peak_bytes: optional_integer(body, "peak_bytes")?,
            pauses: optional_integer(body, "pauses")?,
            command: optional_text(body, "command")?,
            process_group: optional_integer(body, "process_group")?,
            session_name: optional_text(body, "session_name")?.map(SessionName),
            session_id: optional_text(body, "session_id")?.map(SessionId),
            reclaimed_bytes: optional_integer(body, "reclaimed_bytes")?,
            shortfall_bytes: optional_integer(body, "shortfall_bytes")?,
            requested_at: Timestamp(text(body, "requested_at")?),
            started_at: optional_text(body, "started_at")?.map(Timestamp),
            released_at: optional_text(body, "released_at")?.map(Timestamp),
            release_reason: match optional_text(body, "release_reason")? {
                Some(reason) => Some(
                    reason_named(&reason)
                        .ok_or_else(|| format!("unknown ReleaseReason {reason:?}"))?,
                ),
                None => None,
            },
        },
    })
}

/// A field that is text or null; an absent field reads as null.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("no text or null field {field:?}")),
    }
}

/// A field that is an integer or null; an absent field reads as null.
fn optional_integer(body: &Value, field: &str) -> Result<Option<i64>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("no integer or null field {field:?}")),
    }
}
