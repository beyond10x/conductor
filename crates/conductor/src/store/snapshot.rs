//! `SnapshotStorage` and the observation storage ports on [`super::Store`], filled by
//! `story:snapshot-driver`: one for the snapshot and one for each observation a collector records,
//! `SpecificationObservation` included, so a collector story writes no storage of its own. The
//! release and merged-pull-request observations are `story:release-digest`'s.
//!
//! Each instance is one stream, as in `store.rs`: the snapshot's in `state/tree/`, an
//! observation's in its snapshot's tenant of `state/observations/` ([`super::observations`]),
//! which the store's `read`, `write` and `rows` choose by the entity. Its body holds the
//! instance's state and fields under the specification's names: an enum by the name the
//! specification gives its value, an absent optional as `null`. An observation's one state,
//! `Recorded`, is matched irrefutably, so a state the specification adds fails to compile here
//! rather than being written under a guess.
//!
//! One field is read under a second name: a repository observation's `in_catalog`, which the
//! builds before `story:catalog-source` wrote under an earlier name ([`EARLIER_IN_CATALOG`]) as
//! a Boolean. ESS declares no earlier name for a field, so the read is kept here, where the
//! bodies are; the records are not rewritten.

use conductor_model::behaviour::{
    BlockerObservationStorage, MergedPullRequestObservationStorage, PullRequestObservationStorage,
    ReleaseObservationStorage, RepositoryObservationStorage, SessionObservationStorage,
    SnapshotStorage, SpecificationObservationStorage, WorkflowRunObservationStorage,
};
use conductor_model::observation::{
    BlockerObservationData, BlockerObservationSnapshot, BlockerObservationState, CommitSha,
    MergedPullRequestObservationData, MergedPullRequestObservationSnapshot,
    MergedPullRequestObservationState, ObservationId, PullRequestObservationData,
    PullRequestObservationSnapshot, PullRequestObservationState, ReleaseObservationData,
    ReleaseObservationSnapshot, ReleaseObservationState, RepositoryName, RepositoryObservationData,
    RepositoryObservationSnapshot, RepositoryObservationState, SessionObservationData,
    SessionObservationSnapshot, SessionObservationState, SnapshotData, SnapshotId,
    SnapshotSnapshot, SpecificationObservationData, SpecificationObservationSnapshot,
    SpecificationObservationState, WorkflowRunObservationData, WorkflowRunObservationSnapshot,
    WorkflowRunObservationState,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};
use crate::snapshot::{
    harness_name, harness_named, mergeability_name, mergeability_named, run_conclusion_name,
    run_conclusion_named, session_state_name, session_state_named, snapshot_state_name,
    snapshot_state_named, specification_presence_name, specification_presence_named,
    validation_result_name, validation_result_named, visibility_name, visibility_named,
};

/// `conductor.observation.Snapshot`.
const SNAPSHOT: &str = "conductor.observation.Snapshot";

/// `conductor.observation.RepositoryObservation`.
pub(super) const REPOSITORY: &str = "conductor.observation.RepositoryObservation";

/// `conductor.observation.PullRequestObservation`.
pub(super) const PULL_REQUEST: &str = "conductor.observation.PullRequestObservation";

/// `conductor.observation.ReleaseObservation`.
pub(super) const RELEASE: &str = "conductor.observation.ReleaseObservation";

/// `conductor.observation.MergedPullRequestObservation`.
pub(super) const MERGED_PULL_REQUEST: &str = "conductor.observation.MergedPullRequestObservation";

/// `conductor.observation.WorkflowRunObservation`.
pub(super) const WORKFLOW_RUN: &str = "conductor.observation.WorkflowRunObservation";

/// `conductor.observation.SessionObservation`.
pub(super) const SESSION: &str = "conductor.observation.SessionObservation";

/// `conductor.observation.BlockerObservation`.
pub(super) const BLOCKER: &str = "conductor.observation.BlockerObservation";

/// `conductor.observation.SpecificationObservation`.
pub(super) const SPECIFICATION: &str = "conductor.observation.SpecificationObservation";

/// The one state of every observation, as the specification spells it.
const RECORDED: &str = "Recorded";

/// One storage port on [`Store`]: `$identity` is the field of `$snapshot`'s data that keys it.
/// A read that does not decode, or decodes into another instance, is kept for [`Store::check`]
/// and reads as absent, as `GoalStorage` does in `store.rs`.
macro_rules! storage {
    ($port:ident, $entity:ident, $id:ident, $snapshot:ident, $identity:ident, $encode:ident, $decode:ident) => {
        impl $port for Store {
            fn get(&self, identity: &$id) -> Option<$snapshot> {
                let key = &identity.0.0;
                let held = decoded(self, $entity, key, $decode)?;
                if held.data.$identity == *identity {
                    Some(held)
                } else {
                    self.fail(undecodable(
                        $entity,
                        key,
                        format!("holds {:?}", held.data.$identity.0.0),
                    ));
                    None
                }
            }

            fn put(&mut self, snapshot: $snapshot) {
                let body = $encode(&snapshot);
                self.write($entity, &snapshot.data.$identity.0.0, STORED, body);
            }

            fn delete(&mut self, identity: &$id) {
                self.write($entity, &identity.0.0, DELETED, json!({}));
            }

            fn list(&self) -> Vec<$snapshot> {
                listed(self, $entity, $decode)
            }
        }
    };
}

storage!(
    SnapshotStorage,
    SNAPSHOT,
    SnapshotId,
    SnapshotSnapshot,
    snapshot_id,
    encode_snapshot,
    decode_snapshot
);
storage!(
    RepositoryObservationStorage,
    REPOSITORY,
    ObservationId,
    RepositoryObservationSnapshot,
    observation_id,
    encode_repository,
    decode_repository
);
storage!(
    PullRequestObservationStorage,
    PULL_REQUEST,
    ObservationId,
    PullRequestObservationSnapshot,
    observation_id,
    encode_pull_request,
    decode_pull_request
);
storage!(
    ReleaseObservationStorage,
    RELEASE,
    ObservationId,
    ReleaseObservationSnapshot,
    observation_id,
    encode_release,
    decode_release
);
storage!(
    MergedPullRequestObservationStorage,
    MERGED_PULL_REQUEST,
    ObservationId,
    MergedPullRequestObservationSnapshot,
    observation_id,
    encode_merged_pull_request,
    decode_merged_pull_request
);
storage!(
    WorkflowRunObservationStorage,
    WORKFLOW_RUN,
    ObservationId,
    WorkflowRunObservationSnapshot,
    observation_id,
    encode_workflow_run,
    decode_workflow_run
);
storage!(
    SessionObservationStorage,
    SESSION,
    ObservationId,
    SessionObservationSnapshot,
    observation_id,
    encode_session,
    decode_session
);
storage!(
    BlockerObservationStorage,
    BLOCKER,
    ObservationId,
    BlockerObservationSnapshot,
    observation_id,
    encode_blocker,
    decode_blocker
);
storage!(
    SpecificationObservationStorage,
    SPECIFICATION,
    ObservationId,
    SpecificationObservationSnapshot,
    observation_id,
    encode_specification,
    decode_specification
);

/// The instance `key` of `entity`, decoded; `None` when absent, or when the read or the decoding
/// failed (kept for [`Store::check`]).
fn decoded<T>(
    store: &Store,
    entity: &'static str,
    key: &str,
    decode: fn(&Value) -> Result<T, String>,
) -> Option<T> {
    let body = store.read(entity, key)?;
    match decode(&body) {
        Ok(held) => Some(held),
        Err(detail) => {
            store.fail(undecodable(entity, key, detail));
            None
        }
    }
}

/// Every instance of `entity`, decoded, in the order they were first stored. One that does not
/// decode is left out and kept for [`Store::check`].
fn listed<T>(
    store: &Store,
    entity: &'static str,
    decode: fn(&Value) -> Result<T, String>,
) -> Vec<T> {
    store
        .rows(entity)
        .into_iter()
        .filter_map(|(stream, body)| match decode(&body) {
            Ok(held) => Some(held),
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

fn encode_snapshot(snapshot: &SnapshotSnapshot) -> Value {
    let data = &snapshot.data;
    json!({
        "state": snapshot_state_name(snapshot.state),
        "snapshot_id": data.snapshot_id.0.0,
        "started_at": data.started_at.0,
        "disk_free_bytes": data.disk_free_bytes,
        "failure_reason": data.failure_reason,
    })
}

fn decode_snapshot(body: &Value) -> Result<SnapshotSnapshot, String> {
    Ok(SnapshotSnapshot {
        state: named(body, "state", snapshot_state_named)?,
        data: SnapshotData {
            snapshot_id: SnapshotId(Uuid(text(body, "snapshot_id")?)),
            started_at: Timestamp(text(body, "started_at")?),
            disk_free_bytes: integer(body, "disk_free_bytes")?,
            failure_reason: optional_text(body, "failure_reason")?,
        },
    })
}

fn encode_repository(observation: &RepositoryObservationSnapshot) -> Value {
    let RepositoryObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "visibility": visibility_name(data.visibility),
        "archived": data.archived,
        "local_checkout": data.local_checkout,
        "main_head": data.main_head.0,
        "main_committed_at": data.main_committed_at.0,
        "main_commits_7d": data.main_commits_7d,
        "real_commits_7d": data.real_commits_7d,
        "behind_main": data.behind_main,
        "dirty_files": data.dirty_files,
        "worktrees": data.worktrees,
        "open_issues": data.open_issues,
        "oldest_open_issue_at": data.oldest_open_issue_at.as_ref().map(|at| &at.0),
        "latest_release": data.latest_release,
        "latest_release_at": data.latest_release_at.as_ref().map(|at| &at.0),
        "unreleased_commits": data.unreleased_commits,
        "planning_store_version": data.planning_store_version,
        "in_catalog": data.in_catalog,
    })
}

fn decode_repository(body: &Value) -> Result<RepositoryObservationSnapshot, String> {
    recorded(body)?;
    Ok(RepositoryObservationSnapshot {
        state: RepositoryObservationState::Recorded,
        data: RepositoryObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            visibility: named(body, "visibility", visibility_named)?,
            archived: boolean(body, "archived")?,
            local_checkout: boolean(body, "local_checkout")?,
            main_head: CommitSha(text(body, "main_head")?),
            main_committed_at: Timestamp(text(body, "main_committed_at")?),
            main_commits_7d: integer(body, "main_commits_7d")?,
            real_commits_7d: integer(body, "real_commits_7d")?,
            behind_main: integer(body, "behind_main")?,
            dirty_files: integer(body, "dirty_files")?,
            worktrees: integer(body, "worktrees")?,
            open_issues: integer(body, "open_issues")?,
            oldest_open_issue_at: optional_text(body, "oldest_open_issue_at")?.map(Timestamp),
            latest_release: optional_text(body, "latest_release")?,
            latest_release_at: optional_text(body, "latest_release_at")?.map(Timestamp),
            unreleased_commits: optional_integer(body, "unreleased_commits")?,
            planning_store_version: optional_text(body, "planning_store_version")?,
            in_catalog: in_catalog(body)?,
        },
    })
}

fn encode_pull_request(observation: &PullRequestObservationSnapshot) -> Value {
    let PullRequestObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "number": data.number,
        "title": data.title,
        "draft": data.draft,
        "mergeability": mergeability_name(data.mergeability),
        "failing_checks": data.failing_checks,
        "opened_at": data.opened_at.0,
        "updated_at": data.updated_at.0,
    })
}

fn decode_pull_request(body: &Value) -> Result<PullRequestObservationSnapshot, String> {
    recorded(body)?;
    Ok(PullRequestObservationSnapshot {
        state: PullRequestObservationState::Recorded,
        data: PullRequestObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            number: integer(body, "number")?,
            title: text(body, "title")?,
            draft: boolean(body, "draft")?,
            mergeability: named(body, "mergeability", mergeability_named)?,
            failing_checks: integer(body, "failing_checks")?,
            opened_at: Timestamp(text(body, "opened_at")?),
            updated_at: Timestamp(text(body, "updated_at")?),
        },
    })
}

fn encode_release(observation: &ReleaseObservationSnapshot) -> Value {
    let ReleaseObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "tag": data.tag,
        "published_at": data.published_at.0,
        "url": data.url,
    })
}

fn decode_release(body: &Value) -> Result<ReleaseObservationSnapshot, String> {
    recorded(body)?;
    Ok(ReleaseObservationSnapshot {
        state: ReleaseObservationState::Recorded,
        data: ReleaseObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            tag: text(body, "tag")?,
            published_at: Timestamp(text(body, "published_at")?),
            url: text(body, "url")?,
        },
    })
}

fn encode_merged_pull_request(observation: &MergedPullRequestObservationSnapshot) -> Value {
    let MergedPullRequestObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "number": data.number,
        "title": data.title,
        "merged_at": data.merged_at.0,
        "url": data.url,
    })
}

fn decode_merged_pull_request(
    body: &Value,
) -> Result<MergedPullRequestObservationSnapshot, String> {
    recorded(body)?;
    Ok(MergedPullRequestObservationSnapshot {
        state: MergedPullRequestObservationState::Recorded,
        data: MergedPullRequestObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            number: integer(body, "number")?,
            title: text(body, "title")?,
            merged_at: Timestamp(text(body, "merged_at")?),
            url: text(body, "url")?,
        },
    })
}

fn encode_workflow_run(observation: &WorkflowRunObservationSnapshot) -> Value {
    let WorkflowRunObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "workflow": data.workflow,
        "conclusion": run_conclusion_name(data.conclusion),
        "run_at": data.run_at.0,
    })
}

fn decode_workflow_run(body: &Value) -> Result<WorkflowRunObservationSnapshot, String> {
    recorded(body)?;
    Ok(WorkflowRunObservationSnapshot {
        state: WorkflowRunObservationState::Recorded,
        data: WorkflowRunObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            workflow: text(body, "workflow")?,
            conclusion: named(body, "conclusion", run_conclusion_named)?,
            run_at: Timestamp(text(body, "run_at")?),
        },
    })
}

fn encode_session(observation: &SessionObservationSnapshot) -> Value {
    let SessionObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "harness": harness_name(data.harness),
        "session_ref": data.session_ref,
        "name": data.name,
        "cwd": data.cwd,
        "repository": data.repository.as_ref().map(|repository| &repository.0),
        "role": data.role,
        "activity": session_state_name(data.activity),
    })
}

fn decode_session(body: &Value) -> Result<SessionObservationSnapshot, String> {
    recorded(body)?;
    Ok(SessionObservationSnapshot {
        state: SessionObservationState::Recorded,
        data: SessionObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            harness: named(body, "harness", harness_named)?,
            session_ref: text(body, "session_ref")?,
            name: optional_text(body, "name")?,
            cwd: text(body, "cwd")?,
            repository: optional_text(body, "repository")?.map(RepositoryName),
            role: optional_text(body, "role")?,
            activity: named(body, "activity", session_state_named)?,
        },
    })
}

fn encode_blocker(observation: &BlockerObservationSnapshot) -> Value {
    let BlockerObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "reference": data.reference,
        "blocker_kind": data.blocker_kind,
        "title": data.title,
    })
}

fn decode_blocker(body: &Value) -> Result<BlockerObservationSnapshot, String> {
    recorded(body)?;
    Ok(BlockerObservationSnapshot {
        state: BlockerObservationState::Recorded,
        data: BlockerObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            reference: text(body, "reference")?,
            blocker_kind: text(body, "blocker_kind")?,
            title: text(body, "title")?,
        },
    })
}

fn encode_specification(observation: &SpecificationObservationSnapshot) -> Value {
    let SpecificationObservationState::Recorded = observation.state;
    let data = &observation.data;
    json!({
        "state": RECORDED,
        "observation_id": data.observation_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "repository": data.repository.0,
        "presence": specification_presence_name(data.presence),
        "path": data.path,
        "format": data.format,
        "required_ess": data.required_ess,
        "validation": validation_result_name(data.validation),
        "validation_refusals": data.validation_refusals,
        "scenarios": data.scenarios,
        "synthesis_refusals": data.synthesis_refusals,
        "conformance_status": data.conformance_status,
    })
}

fn decode_specification(body: &Value) -> Result<SpecificationObservationSnapshot, String> {
    recorded(body)?;
    Ok(SpecificationObservationSnapshot {
        state: SpecificationObservationState::Recorded,
        data: SpecificationObservationData {
            observation_id: observation_id(body)?,
            snapshot_id: snapshot_id(body)?,
            repository: RepositoryName(text(body, "repository")?),
            presence: named(body, "presence", specification_presence_named)?,
            path: optional_text(body, "path")?,
            format: optional_text(body, "format")?,
            required_ess: optional_text(body, "required_ess")?,
            validation: named(body, "validation", validation_result_named)?,
            validation_refusals: integer(body, "validation_refusals")?,
            scenarios: optional_integer(body, "scenarios")?,
            synthesis_refusals: optional_integer(body, "synthesis_refusals")?,
            conformance_status: optional_text(body, "conformance_status")?,
        },
    })
}

/// The name the builds before `story:catalog-source` wrote a repository observation's catalog
/// membership under, a Boolean that the store's records still hold.
const EARLIER_IN_CATALOG: &str = "in_atlas_catalog";

/// A repository observation's `in_catalog`: the field when the body holds it, `null` as unknown;
/// else the Boolean an earlier build wrote under [`EARLIER_IN_CATALOG`]; else unknown.
fn in_catalog(body: &Value) -> Result<Option<bool>, String> {
    if body.get("in_catalog").is_some() {
        optional_boolean(body, "in_catalog")
    } else {
        optional_boolean(body, EARLIER_IN_CATALOG)
    }
}

/// Refuses a body whose state is not `Recorded`, the one state an observation has.
fn recorded(body: &Value) -> Result<(), String> {
    let state = text(body, "state")?;
    if state == RECORDED {
        Ok(())
    } else {
        Err(format!("unknown observation state {state:?}"))
    }
}

fn observation_id(body: &Value) -> Result<ObservationId, String> {
    Ok(ObservationId(Uuid(text(body, "observation_id")?)))
}

fn snapshot_id(body: &Value) -> Result<SnapshotId, String> {
    Ok(SnapshotId(Uuid(text(body, "snapshot_id")?)))
}

/// The enum value `field` names, by the specification's spelling.
fn named<T>(body: &Value, field: &str, named: fn(&str) -> Option<T>) -> Result<T, String> {
    let name = text(body, field)?;
    named(&name).ok_or_else(|| format!("unknown {field} {name:?}"))
}

fn integer(body: &Value, field: &str) -> Result<i64, String> {
    body.get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("no integer field {field:?}"))
}

fn boolean(body: &Value, field: &str) -> Result<bool, String> {
    body.get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("no boolean field {field:?}"))
}

/// An optional field: `null` or absent is `None`.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("field {field:?} is neither text nor null")),
    }
}

/// An optional field: `null` or absent is `None`.
fn optional_boolean(body: &Value, field: &str) -> Result<Option<bool>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("field {field:?} is neither a boolean nor null")),
    }
}

/// An optional field: `null` or absent is `None`.
fn optional_integer(body: &Value, field: &str) -> Result<Option<i64>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("field {field:?} is neither an integer nor null")),
    }
}
