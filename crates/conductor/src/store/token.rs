//! `PoolTokenStorage` and `OperatorStorage` (the model's ports) on [`super::Store`]
//! (`story:token-pool`): each `conductor.token.PoolToken` is one stream keyed by its id, and each
//! `conductor.operator.Operator` one stream keyed by `<instance>/<operator>`, in the daemon's tenant
//! of the deployment's database. A token's record holds its owner, seat, state and the name of the
//! secret that holds its value, never the value: the value is in the secrets source only.
//!
//! A token's record also counts its leases (`leases`), one more each time a put moves it into
//! `Leased`: the pool's pick reads it as the token's headroom (decision K3).

use conductor_model::agent::AgentName;
use conductor_model::behaviour::{OperatorStorage, PoolTokenStorage};
use conductor_model::config::InstanceName;
use conductor_model::operator::{
    Grant, OperatorData, OperatorKey, OperatorName, OperatorSnapshot, OperatorState,
};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenState,
};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.token.PoolToken`.
const POOL_TOKEN: &str = "conductor.token.PoolToken";

/// `conductor.operator.Operator`.
const OPERATOR: &str = "conductor.operator.Operator";

/// The state's name as the specification spells it.
#[must_use]
pub const fn token_state_name(state: PoolTokenState) -> &'static str {
    match state {
        PoolTokenState::Available => "Available",
        PoolTokenState::Leased => "Leased",
        PoolTokenState::Limited => "Limited",
        PoolTokenState::Withdrawn => "Withdrawn",
    }
}

/// The usage state's name as the specification spells it.
#[must_use]
pub const fn usage_name(state: TokenState) -> &'static str {
    match state {
        TokenState::Ok => "Ok",
        TokenState::Limited => "Limited",
    }
}

impl Store {
    /// How many times the token `id` was leased, as its record counts them.
    #[must_use]
    pub fn token_leases(&self, id: &TokenId) -> u64 {
        self.read(POOL_TOKEN, &id.0.0)
            .and_then(|body| body.get("leases").and_then(Value::as_u64))
            .unwrap_or(0)
    }
}

impl PoolTokenStorage for Store {
    fn get(&self, identity: &TokenId) -> Option<PoolTokenSnapshot> {
        let body = self.read(POOL_TOKEN, &identity.0.0)?;
        match decode_token(&body) {
            Ok(found) => Some(found),
            Err(detail) => {
                self.fail(undecodable(POOL_TOKEN, &identity.0.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: PoolTokenSnapshot) {
        let before = self.read(POOL_TOKEN, &snapshot.data.token_id.0.0);
        let was_leased = before
            .as_ref()
            .and_then(|body| body.get("state").and_then(Value::as_str))
            == Some("Leased");
        let mut leases = before
            .as_ref()
            .and_then(|body| body.get("leases").and_then(Value::as_u64))
            .unwrap_or(0);
        if snapshot.state == PoolTokenState::Leased && !was_leased {
            leases += 1;
        }
        let mut body = encode_token(&snapshot);
        body["leases"] = json!(leases);
        self.write(POOL_TOKEN, &snapshot.data.token_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &TokenId) {
        self.write(POOL_TOKEN, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<PoolTokenSnapshot> {
        self.rows(POOL_TOKEN)
            .into_iter()
            .filter_map(|(stream, body)| match decode_token(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: POOL_TOKEN,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_token(found: &PoolTokenSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": token_state_name(found.state),
        "token_id": data.token_id.0.0,
        "instance": data.instance.0,
        "owner": data.owner.0,
        "seat": data.seat.0,
        "secret": data.secret.0,
        "token_state": usage_name(data.token_state),
        "limited_until": data.limited_until.as_ref().map(|at| &at.0),
        "agent": data.agent.as_ref().map(|agent| &agent.0),
        "contributed_at": data.contributed_at.0,
    })
}

fn decode_token(body: &Value) -> Result<PoolTokenSnapshot, String> {
    let state = match text(body, "state")?.as_str() {
        "Available" => PoolTokenState::Available,
        "Leased" => PoolTokenState::Leased,
        "Limited" => PoolTokenState::Limited,
        "Withdrawn" => PoolTokenState::Withdrawn,
        other => return Err(format!("unknown PoolToken state {other:?}")),
    };
    let token_state = match text(body, "token_state")?.as_str() {
        "Ok" => TokenState::Ok,
        "Limited" => TokenState::Limited,
        other => return Err(format!("unknown TokenState {other:?}")),
    };
    let optional = |field: &str| body.get(field).and_then(Value::as_str).map(str::to_owned);
    Ok(PoolTokenSnapshot {
        state,
        data: PoolTokenData {
            token_id: TokenId(Uuid(text(body, "token_id")?)),
            instance: InstanceName(text(body, "instance")?),
            owner: OperatorName(text(body, "owner")?),
            seat: Seat(text(body, "seat")?),
            secret: SecretName(text(body, "secret")?),
            token_state,
            limited_until: optional("limited_until").map(Timestamp),
            agent: optional("agent").map(AgentName),
            contributed_at: Timestamp(text(body, "contributed_at")?),
            actor: crate::actor::stored(body)?,
        },
    })
}

/// The stream of the operator `key`.
fn operator_stream(key: &OperatorKey) -> String {
    format!("{}/{}", key.instance.0, key.operator.0)
}

/// The grant's name as the specification spells it.
#[must_use]
pub const fn grant_name(grant: Grant) -> &'static str {
    match grant {
        Grant::AnswerDecisions => "AnswerDecisions",
        Grant::ChangeSettings => "ChangeSettings",
        Grant::StartInstances => "StartInstances",
        Grant::StopInstances => "StopInstances",
        Grant::UpgradeInstances => "UpgradeInstances",
        Grant::Purge => "Purge",
        Grant::Join => "Join",
    }
}

fn grant_of(name: &str) -> Option<Grant> {
    Some(match name {
        "AnswerDecisions" => Grant::AnswerDecisions,
        "ChangeSettings" => Grant::ChangeSettings,
        "StartInstances" => Grant::StartInstances,
        "StopInstances" => Grant::StopInstances,
        "UpgradeInstances" => Grant::UpgradeInstances,
        "Purge" => Grant::Purge,
        "Join" => Grant::Join,
        _ => return None,
    })
}

impl OperatorStorage for Store {
    fn get(&self, identity: &OperatorKey) -> Option<OperatorSnapshot> {
        let stream = operator_stream(identity);
        let body = self.read(OPERATOR, &stream)?;
        match decode_operator(&body) {
            Ok(found) => Some(found),
            Err(detail) => {
                self.fail(undecodable(OPERATOR, &stream, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: OperatorSnapshot) {
        let stream = operator_stream(&snapshot.data.key);
        self.write(OPERATOR, &stream, STORED, encode_operator(&snapshot));
    }

    fn delete(&mut self, identity: &OperatorKey) {
        self.write(OPERATOR, &operator_stream(identity), DELETED, json!({}));
    }

    fn list(&self) -> Vec<OperatorSnapshot> {
        self.rows(OPERATOR)
            .into_iter()
            .filter_map(|(stream, body)| match decode_operator(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: OPERATOR,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_operator(found: &OperatorSnapshot) -> Value {
    let data = &found.data;
    json!({
        "state": match found.state {
            OperatorState::Active => "Active",
            OperatorState::Left => "Left",
        },
        "operator": data.key.operator.0,
        "instance": data.key.instance.0,
        "grants": data.grants.iter().map(|grant| grant_name(*grant)).collect::<Vec<_>>(),
        "joined_at": data.joined_at.0,
        "actor": crate::actor::to_json(&data.joined_by),
        "left_at": data.left_at.as_ref().map(|at| &at.0),
        "left_by": data.left_by.as_ref().map(crate::actor::to_json),
    })
}

fn decode_operator(body: &Value) -> Result<OperatorSnapshot, String> {
    let state = match text(body, "state")?.as_str() {
        "Active" => OperatorState::Active,
        "Left" => OperatorState::Left,
        other => return Err(format!("unknown Operator state {other:?}")),
    };
    let left_by = match body.get("left_by") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            crate::actor::parse(&value.to_string()).map_err(|error| format!("left_by: {error}"))?,
        ),
    };
    Ok(OperatorSnapshot {
        state,
        data: OperatorData {
            key: OperatorKey {
                operator: OperatorName(text(body, "operator")?),
                instance: InstanceName(text(body, "instance")?),
            },
            grants: body
                .get("grants")
                .and_then(Value::as_array)
                .map(|grants| {
                    grants
                        .iter()
                        .filter_map(Value::as_str)
                        .filter_map(grant_of)
                        .collect()
                })
                .unwrap_or_default(),
            joined_at: Timestamp(text(body, "joined_at")?),
            joined_by: crate::actor::stored(body)?,
            left_at: body
                .get("left_at")
                .and_then(Value::as_str)
                .map(|at| Timestamp(at.to_owned())),
            left_by,
        },
    })
}
