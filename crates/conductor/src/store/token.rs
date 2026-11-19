//! `PoolTokenStorage`, `TokenLeaseStorage` and `OperatorStorage` (the model's ports) on
//! [`super::Store`] (`story:token-pool`, `story:host-token-pool`): each `conductor.token.PoolToken`
//! of the host's pool is one stream keyed by its id, each `conductor.token.TokenLease` one stream
//! keyed by its id, and each `conductor.operator.Operator` one stream keyed by
//! `<instance>/<operator>`, in the daemon's tenant of the deployment's database. A token's record
//! holds its owner, seat, state and the name of the secret that holds its value, never the value:
//! the value is in the secrets source only.
//!
//! A token recorded before the pool was the host's (`story:host-token-pool`) names an instance and
//! the agent of its one lease, and may be `Leased`: it reads as a token of the host's pool as it
//! is, `Available` where it was `Leased`, its instance and agent left out; `conductor store
//! migrate` copies it so. The pick counts a token's live (`Held`) leases ([`Store::live_leases`]).

use std::collections::BTreeMap;

use conductor_model::agent::AgentName;
use conductor_model::behaviour::{OperatorStorage, PoolTokenStorage, TokenLeaseStorage};
use conductor_model::config::{InstanceName, ModelTokenKind};
use conductor_model::operator::{
    Grant, OperatorData, OperatorKey, OperatorName, OperatorSnapshot, OperatorState,
};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenLeaseData, TokenLeaseId,
    TokenLeaseSnapshot, TokenLeaseState, TokenState,
};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.token.PoolToken`.
const POOL_TOKEN: &str = "conductor.token.PoolToken";

/// `conductor.token.TokenLease`.
const TOKEN_LEASE: &str = "conductor.token.TokenLease";

/// `conductor.operator.Operator`.
const OPERATOR: &str = "conductor.operator.Operator";

/// The state's name as the specification spells it.
#[must_use]
pub const fn token_state_name(state: PoolTokenState) -> &'static str {
    match state {
        PoolTokenState::Available => "Available",
        PoolTokenState::Limited => "Limited",
        PoolTokenState::Withdrawn => "Withdrawn",
    }
}

/// The lease state's name as the specification spells it.
#[must_use]
pub const fn lease_state_name(state: TokenLeaseState) -> &'static str {
    match state {
        TokenLeaseState::Held => "Held",
        TokenLeaseState::Released => "Released",
    }
}

/// The token kind's name as the specification spells it
/// (`story:pool-token-kind-files-source`).
#[must_use]
pub const fn token_kind_name(kind: ModelTokenKind) -> &'static str {
    match kind {
        ModelTokenKind::ApiKey => "ApiKey",
        ModelTokenKind::SubscriptionToken => "SubscriptionToken",
    }
}

/// The token kind `name` spells ([`token_kind_name`]).
#[must_use]
pub fn token_kind_of(name: &str) -> Option<ModelTokenKind> {
    match name {
        "ApiKey" => Some(ModelTokenKind::ApiKey),
        "SubscriptionToken" => Some(ModelTokenKind::SubscriptionToken),
        _ => None,
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
        let body = encode_token(&snapshot);
        self.write(POOL_TOKEN, &snapshot.data.token_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &TokenId) {
        self.write(POOL_TOKEN, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<PoolTokenSnapshot> {
        self.pool_tokens()
    }
}

impl TokenLeaseStorage for Store {
    fn get(&self, identity: &TokenLeaseId) -> Option<TokenLeaseSnapshot> {
        let body = self.read(TOKEN_LEASE, &identity.0.0)?;
        match decode_lease(&body) {
            Ok(found) => Some(found),
            Err(detail) => {
                self.fail(undecodable(TOKEN_LEASE, &identity.0.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: TokenLeaseSnapshot) {
        let body = encode_lease(&snapshot);
        self.write(TOKEN_LEASE, &snapshot.data.lease_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &TokenLeaseId) {
        self.write(TOKEN_LEASE, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<TokenLeaseSnapshot> {
        self.token_leases()
    }
}

impl Store {
    /// Every token of the host's pool, withdrawn ones included.
    #[must_use]
    pub fn pool_tokens(&self) -> Vec<PoolTokenSnapshot> {
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

    /// Every lease of the host's pool, held or released.
    #[must_use]
    pub fn token_leases(&self) -> Vec<TokenLeaseSnapshot> {
        self.rows(TOKEN_LEASE)
            .into_iter()
            .filter_map(|(stream, body)| match decode_lease(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: TOKEN_LEASE,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }

    /// How many live (`Held`) leases each token holds, by its id: the pick's headroom
    /// (story:host-token-pool).
    #[must_use]
    pub fn live_leases(&self) -> BTreeMap<String, u64> {
        let mut live = BTreeMap::new();
        for lease in self.token_leases() {
            if lease.state == TokenLeaseState::Held {
                *live.entry(lease.data.token_id.0.0).or_insert(0) += 1;
            }
        }
        live
    }
}

fn encode_token(found: &PoolTokenSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": token_state_name(found.state),
        "token_id": data.token_id.0.0,
        "owner": data.owner.0,
        "seat": data.seat.0,
        "secret": data.secret.0,
        "token_state": usage_name(data.token_state),
        "limited_until": data.limited_until.as_ref().map(|at| &at.0),
        "contributed_at": data.contributed_at.0,
        "kind": data.kind.map(token_kind_name),
    })
}

fn decode_token(body: &Value) -> Result<PoolTokenSnapshot, String> {
    let state = match text(body, "state")?.as_str() {
        // A token of a per-instance pool held its one lease in its own state: of the host's
        // pool, it is a token as it is, its leases records of their own (story:host-token-pool).
        "Available" | "Leased" => PoolTokenState::Available,
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
    // A token recorded before the kind existed has none (`conductor.token.PoolToken`).
    let kind = match optional("kind") {
        None => None,
        Some(name) => {
            Some(token_kind_of(&name).ok_or_else(|| format!("unknown ModelTokenKind {name:?}"))?)
        }
    };
    Ok(PoolTokenSnapshot {
        state,
        data: PoolTokenData {
            token_id: TokenId(Uuid(text(body, "token_id")?)),
            owner: OperatorName(text(body, "owner")?),
            seat: Seat(text(body, "seat")?),
            secret: SecretName(text(body, "secret")?),
            token_state,
            limited_until: optional("limited_until").map(Timestamp),
            contributed_at: Timestamp(text(body, "contributed_at")?),
            actor: crate::actor::stored(body)?,
            kind,
        },
    })
}

fn encode_lease(found: &TokenLeaseSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": lease_state_name(found.state),
        "lease_id": data.lease_id.0.0,
        "token_id": data.token_id.0.0,
        "agent": data.agent.0,
        "instance": data.instance.0,
        "owner": data.owner.0,
        "preferred_owner": data.preferred_owner.as_ref().map(|owner| &owner.0),
        "leased_at": data.leased_at.0,
        "released_at": data.released_at.as_ref().map(|at| &at.0),
        "runtime": data.runtime,
    })
}

fn decode_lease(body: &Value) -> Result<TokenLeaseSnapshot, String> {
    let state = match text(body, "state")?.as_str() {
        "Held" => TokenLeaseState::Held,
        "Released" => TokenLeaseState::Released,
        other => return Err(format!("unknown TokenLease state {other:?}")),
    };
    let optional = |field: &str| body.get(field).and_then(Value::as_str).map(str::to_owned);
    Ok(TokenLeaseSnapshot {
        state,
        data: TokenLeaseData {
            lease_id: conductor_model::token::TokenLeaseId(Uuid(text(body, "lease_id")?)),
            token_id: TokenId(Uuid(text(body, "token_id")?)),
            agent: AgentName(text(body, "agent")?),
            instance: InstanceName(text(body, "instance")?),
            owner: OperatorName(text(body, "owner")?),
            preferred_owner: optional("preferred_owner").map(OperatorName),
            leased_at: Timestamp(text(body, "leased_at")?),
            released_at: optional("released_at").map(Timestamp),
            actor: crate::actor::stored(body)?,
            runtime: optional("runtime"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// story:host-token-pool: a token of a per-instance pool, recorded with its instance and the
    /// agent of its one lease, `Leased`, reads as a token of the host's pool as it is: Available,
    /// its instance and agent left out. `conductor store migrate` copies its record so.
    #[test]
    fn a_per_instance_token_reads_as_a_token_of_the_host_pool() {
        let older = json!({
            "actor": {"kind": "Human", "operator": "alice"},
            "state": "Leased",
            "token_id": "0001",
            "instance": "example-org",
            "owner": "alice",
            "seat": "alice@example-org.example",
            "secret": "claude-token/example-org/0001",
            "token_state": "Ok",
            "limited_until": null,
            "agent": "example-org-controller",
            "contributed_at": "2026-11-08T09:00:00Z",
            "kind": "SubscriptionToken",
            "leases": 3,
        });
        let token = decode_token(&older).expect("an older token reads");
        assert_eq!(token.state, PoolTokenState::Available);
        assert_eq!(token.data.owner.0, "alice");
        assert_eq!(token.data.secret.0, "claude-token/example-org/0001");
        let again = encode_token(&token);
        assert!(again.get("instance").is_none() && again.get("agent").is_none());
    }
}
