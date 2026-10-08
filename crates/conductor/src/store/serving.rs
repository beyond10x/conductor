//! `GoalServingStorage` and `ResourceRequestStorage` on [`super::Store`]
//! (`story:goal-and-resource-commands`), kept the way `impl GoalStorage for Store` keeps a goal: one
//! stream per instance, keyed by its identity, holding the instance's snapshot after each write.

use conductor_model::behaviour::{GoalServingStorage, ResourceRequestStorage};
use conductor_model::direction::{
    GoalId, GoalServingData, GoalServingSnapshot, GoalServingState, ServingId,
};
use conductor_model::dispatch::{
    ResourceKind, ResourceRequestData, ResourceRequestId, ResourceRequestSnapshot,
    ResourceRequestState,
};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.direction.GoalServing`.
const GOAL_SERVING: &str = "conductor.direction.GoalServing";

/// `conductor.dispatch.ResourceRequest`.
const RESOURCE_REQUEST: &str = "conductor.dispatch.ResourceRequest";

/// The instance `key` of `entity` in `store`, decoded, when the store holds it under that key. A
/// record that does not decode, or that holds another instance, is kept for [`Store::check`].
///
/// A free function rather than a method, so no name is added to `Store` that a sibling module
/// could also add.
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

impl GoalServingStorage for Store {
    fn get(&self, identity: &ServingId) -> Option<GoalServingSnapshot> {
        decoded(
            self,
            GOAL_SERVING,
            &identity.0.0,
            decode_goal_serving,
            |serving| &serving.data.serving_id.0.0,
        )
    }

    fn put(&mut self, snapshot: GoalServingSnapshot) {
        let body = encode_goal_serving(&snapshot);
        self.write(GOAL_SERVING, &snapshot.data.serving_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &ServingId) {
        self.write(GOAL_SERVING, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<GoalServingSnapshot> {
        listed(self, GOAL_SERVING, decode_goal_serving)
    }
}

fn encode_goal_serving(serving: &GoalServingSnapshot) -> Value {
    json!({
        "state": goal_serving_state_name(serving.state),
        "serving_id": serving.data.serving_id.0.0,
        "goal_id": serving.data.goal_id.0,
        "repository": serving.data.repository.0,
    })
}

fn decode_goal_serving(body: &Value) -> Result<GoalServingSnapshot, String> {
    let state = text(body, "state")?;
    Ok(GoalServingSnapshot {
        state: goal_serving_state_named(&state)
            .ok_or_else(|| format!("unknown GoalServing state {state:?}"))?,
        data: GoalServingData {
            serving_id: ServingId(Uuid(text(body, "serving_id")?)),
            goal_id: GoalId(text(body, "goal_id")?),
            repository: RepositoryName(text(body, "repository")?),
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn goal_serving_state_name(state: GoalServingState) -> &'static str {
    match state {
        GoalServingState::Removed => "Removed",
        GoalServingState::Serving => "Serving",
    }
}

fn goal_serving_state_named(name: &str) -> Option<GoalServingState> {
    [GoalServingState::Removed, GoalServingState::Serving]
        .into_iter()
        .find(|state| goal_serving_state_name(*state) == name)
}

impl ResourceRequestStorage for Store {
    fn get(&self, identity: &ResourceRequestId) -> Option<ResourceRequestSnapshot> {
        decoded(
            self,
            RESOURCE_REQUEST,
            &identity.0.0,
            decode_resource_request,
            |request| &request.data.resource_request_id.0.0,
        )
    }

    fn put(&mut self, snapshot: ResourceRequestSnapshot) {
        let body = encode_resource_request(&snapshot);
        self.write(
            RESOURCE_REQUEST,
            &snapshot.data.resource_request_id.0.0,
            STORED,
            body,
        );
    }

    fn delete(&mut self, identity: &ResourceRequestId) {
        self.write(RESOURCE_REQUEST, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ResourceRequestSnapshot> {
        listed(self, RESOURCE_REQUEST, decode_resource_request)
    }
}

fn encode_resource_request(request: &ResourceRequestSnapshot) -> Value {
    let data = &request.data;
    json!({
        "state": resource_request_state_name(request.state),
        "resource_request_id": data.resource_request_id.0.0,
        "repository": data.repository.0,
        "resource": resource_kind_name(data.resource),
        "amount": data.amount,
        "requested_at": data.requested_at.0,
    })
}

fn decode_resource_request(body: &Value) -> Result<ResourceRequestSnapshot, String> {
    let state = text(body, "state")?;
    let resource = text(body, "resource")?;
    Ok(ResourceRequestSnapshot {
        state: resource_request_state_named(&state)
            .ok_or_else(|| format!("unknown ResourceRequest state {state:?}"))?,
        data: ResourceRequestData {
            resource_request_id: ResourceRequestId(Uuid(text(body, "resource_request_id")?)),
            repository: RepositoryName(text(body, "repository")?),
            resource: resource_kind_named(&resource)
                .ok_or_else(|| format!("unknown ResourceKind {resource:?}"))?,
            amount: body
                .get("amount")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"amount\"".to_owned())?,
            requested_at: Timestamp(text(body, "requested_at")?),
        },
    })
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn resource_request_state_name(state: ResourceRequestState) -> &'static str {
    match state {
        ResourceRequestState::Granted => "Granted",
        ResourceRequestState::Refused => "Refused",
        ResourceRequestState::Released => "Released",
        ResourceRequestState::Requested => "Requested",
    }
}

fn resource_request_state_named(name: &str) -> Option<ResourceRequestState> {
    [
        ResourceRequestState::Granted,
        ResourceRequestState::Refused,
        ResourceRequestState::Released,
        ResourceRequestState::Requested,
    ]
    .into_iter()
    .find(|state| resource_request_state_name(*state) == name)
}

/// The kind's name as the specification spells it. Exhaustive, so a kind the specification adds
/// fails to compile here rather than being written under a guessed name.
fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::BuildSlot => "BuildSlot",
        ResourceKind::Disk => "Disk",
        ResourceKind::Usage => "Usage",
    }
}

fn resource_kind_named(name: &str) -> Option<ResourceKind> {
    [
        ResourceKind::BuildSlot,
        ResourceKind::Disk,
        ResourceKind::Usage,
    ]
    .into_iter()
    .find(|kind| resource_kind_name(*kind) == name)
}
