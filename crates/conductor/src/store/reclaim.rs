//! `ReclaimRunStorage` on [`super::Store`] (`story:disk-reclaim`): each `conductor.host.ReclaimRun`
//! is one stream, keyed by its id, holding the run's snapshot, kept and read back as every other
//! entity of the store is. A run is recorded once and never changes.

use conductor_model::behaviour::ReclaimRunStorage;
use conductor_model::config::InstanceName;
use conductor_model::host::{
    LeftOver, ReclaimKind, ReclaimResult, ReclaimRunData, ReclaimRunId, ReclaimRunSnapshot,
    ReclaimRunState, ReclaimStep,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.host.ReclaimRun`.
const RECLAIM_RUN: &str = "conductor.host.ReclaimRun";

impl ReclaimRunStorage for Store {
    fn get(&self, identity: &ReclaimRunId) -> Option<ReclaimRunSnapshot> {
        let key = &identity.0;
        let body = self.read(RECLAIM_RUN, key)?;
        match decode(&body) {
            Ok(run) if run.data.reclaim_run_id == *identity => Some(run),
            Ok(run) => {
                self.fail(undecodable(
                    RECLAIM_RUN,
                    key,
                    format!("holds the reclaim run {:?}", run.data.reclaim_run_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(RECLAIM_RUN, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: ReclaimRunSnapshot) {
        let body = encode(&snapshot);
        self.write(RECLAIM_RUN, &snapshot.data.reclaim_run_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &ReclaimRunId) {
        self.write(RECLAIM_RUN, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ReclaimRunSnapshot> {
        self.rows(RECLAIM_RUN)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(run) => Some(run),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: RECLAIM_RUN,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(run: &ReclaimRunSnapshot) -> Value {
    let data = &run.data;
    json!({
        "state": state_name(run.state),
        "reclaim_run_id": data.reclaim_run_id.0,
        "instance": data.instance.0,
        "result": result_name(data.result),
        "steps": data.steps.iter().map(|step| json!({
            "what": step.what,
            "kind": kind_name(step.kind),
            "bytes": step.bytes,
        })).collect::<Vec<_>>(),
        "reclaimed_bytes": data.reclaimed_bytes,
        "shortfall_bytes": data.shortfall_bytes,
        "left": data.left.iter().map(|left| json!({
            "path": left.path,
            "bytes": left.bytes,
        })).collect::<Vec<_>>(),
        "ran_at": data.ran_at.0,
    })
}

fn decode(body: &Value) -> Result<ReclaimRunSnapshot, String> {
    let state = text(body, "state")?;
    let result = text(body, "result")?;
    let steps = list(body, "steps")?
        .iter()
        .map(|step| {
            let kind = text(step, "kind")?;
            Ok(ReclaimStep {
                what: text(step, "what")?,
                kind: kind_named(&kind).ok_or_else(|| format!("unknown ReclaimKind {kind:?}"))?,
                bytes: integer(step, "bytes")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let left = list(body, "left")?
        .iter()
        .map(|left| {
            Ok(LeftOver {
                path: text(left, "path")?,
                bytes: integer(left, "bytes")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ReclaimRunSnapshot {
        state: state_named(&state).ok_or_else(|| format!("unknown ReclaimRun state {state:?}"))?,
        data: ReclaimRunData {
            reclaim_run_id: ReclaimRunId(text(body, "reclaim_run_id")?),
            instance: InstanceName(text(body, "instance")?),
            result: result_named(&result)
                .ok_or_else(|| format!("unknown ReclaimResult {result:?}"))?,
            steps,
            reclaimed_bytes: integer(body, "reclaimed_bytes")?,
            shortfall_bytes: match body.get("shortfall_bytes") {
                Some(Value::Null) => None,
                Some(bytes) => Some(
                    bytes
                        .as_i64()
                        .ok_or_else(|| "no integer or null field \"shortfall_bytes\"".to_owned())?,
                ),
                None => return Err("no integer or null field \"shortfall_bytes\"".to_owned()),
            },
            left,
            ran_at: Timestamp(text(body, "ran_at")?),
        },
    })
}

fn integer(body: &Value, field: &str) -> Result<i64, String> {
    body.get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("no integer field {field:?}"))
}

fn list<'a>(body: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    body.get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("no list field {field:?}"))
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than being written under a guessed name.
fn state_name(state: ReclaimRunState) -> &'static str {
    match state {
        ReclaimRunState::Recorded => "Recorded",
    }
}

fn state_named(name: &str) -> Option<ReclaimRunState> {
    [ReclaimRunState::Recorded]
        .into_iter()
        .find(|state| state_name(*state) == name)
}

fn result_name(result: ReclaimResult) -> &'static str {
    match result {
        ReclaimResult::Reclaimed => "Reclaimed",
        ReclaimResult::FloorNotReached => "FloorNotReached",
    }
}

fn result_named(name: &str) -> Option<ReclaimResult> {
    [ReclaimResult::Reclaimed, ReclaimResult::FloorNotReached]
        .into_iter()
        .find(|result| result_name(*result) == name)
}

/// The kind's name as the specification spells it.
fn kind_name(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::IdleTreeBuildOutput => "IdleTreeBuildOutput",
        ReclaimKind::ArchiveBuildOutput => "ArchiveBuildOutput",
        ReclaimKind::Cache => "Cache",
        ReclaimKind::Snapshot => "Snapshot",
        ReclaimKind::FinishedTree => "FinishedTree",
        ReclaimKind::StaleTreeBuildOutput => "StaleTreeBuildOutput",
    }
}

fn kind_named(name: &str) -> Option<ReclaimKind> {
    [
        ReclaimKind::IdleTreeBuildOutput,
        ReclaimKind::ArchiveBuildOutput,
        ReclaimKind::Cache,
        ReclaimKind::Snapshot,
        ReclaimKind::FinishedTree,
        ReclaimKind::StaleTreeBuildOutput,
    ]
    .into_iter()
    .find(|kind| kind_name(*kind) == name)
}
