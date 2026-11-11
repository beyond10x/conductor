//! `HostGrantStorage` on [`super::Store`] (`story:build-run`): each `conductor.host.HostGrant` is
//! one stream, keyed by its grant id, kept and read back as every other entity of the store is.
//! Every move of a grant appends its new snapshot, so the stream is the grant's history: asked,
//! answered, started, its peak, released.
//!
//! **The open grants** (`story:build-queue-convoy`, `story:status-current-snapshot`). Beside the
//! host store, `ledger.open` is the index of the grants still open (`Requested`, `Granted`,
//! `Running`): each with its newest snapshot, and the position of the store's feed the file is
//! as of. A pass of the ledger reads that file and what the feed committed after its position
//! ([`Store::open_grants`]), never the closed grants, which outnumber the open ones by thousands
//! on a host of long standing, nor the history of any grant: of each grant written since, its
//! newest record alone decides whether it is open. The file is then written again at the new
//! position, whole and renamed over the old one. Each write of a grant through the host's ports
//! keeps the file too ([`Store::opened_put`]), leaving its position where it was, so a write the
//! file names is applied again, to the same effect, by the next pass. A writer that keeps no file
//! (an older binary) is caught up by that pass all the same. Where the file is not there or does
//! not read, a pass reads the feed whole once and writes it anew.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use conductor_model::behaviour::HostGrantStorage;
use conductor_model::config::{InstanceName, SessionName};
use conductor_model::host::{GrantId, HostGrantData, HostGrantSnapshot, HostGrantState};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use conductor_model::session::SessionId;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, stream_id, text, undecodable};
use crate::host::{
    kind_name, kind_named, lane_name, lane_named, reason_name, reason_named, state_name,
    state_named,
};

/// `conductor.host.HostGrant`.
const HOST_GRANT: &str = "conductor.host.HostGrant";

/// The open grants of the ledger, beside its store.
pub const OPEN: &str = "ledger.open";

/// What `ledger.open` holds.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Open {
    /// The position of the store's feed the file is as of: every grant written up to it that is
    /// open is below, and what was committed after it is still to apply.
    position: u64,
    /// Each open grant's snapshot, as the store keeps it, by the id of its stream.
    grants: BTreeMap<String, Value>,
}

/// Whether `state` is a grant's open state: it still holds, or waits for, space.
fn open_state(state: HostGrantState) -> bool {
    matches!(
        state,
        HostGrantState::Requested | HostGrantState::Granted | HostGrantState::Running
    )
}

/// `ledger.open` at `path`; `None` when it is not there or does not read.
fn read_open(path: &Path) -> Option<Open> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// How many disk requests wait in the build ledger of the host directory `host`
/// (`story:daemon-probes-and-metrics`): the grants of [`OPEN`] that are still `Requested`, read
/// without the ledger's lock and without opening its store, as the file was last written. `0`
/// for a host directory that holds no ledger; `None` when the ledger's store is there and the
/// file is not, or when the file or a grant in it does not read: a count nobody read is no `0`.
#[must_use]
pub fn queued(host: &Path) -> Option<usize> {
    let path = host.join(OPEN);
    if !path.exists() && !host.join(super::STORE_FILE).exists() {
        return Some(0);
    }
    let mut waiting = 0;
    for grant in read_open(&path)?.grants.values() {
        let state = state_named(&text(grant, "state").ok()?)?;
        let kind = kind_named(&text(grant, "kind").ok()?)?;
        if state == HostGrantState::Requested && kind == conductor_model::host::GrantKind::Disk {
            waiting += 1;
        }
    }
    Some(waiting)
}

/// Writes `open` to `path` whole: a new file, renamed over the old.
fn write_open(path: &Path, open: &Open) {
    let Ok(text) = serde_json::to_vec(open) else {
        return;
    };
    let _ = replace(path, &text);
}

/// Writes `bytes` to `path` whole, through a file of this writer's own beside it renamed over it,
/// so a reader without a lock reads the old file or the new one, never a part.
///
/// # Errors
///
/// The file cannot be written or renamed.
fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(format!(
        ".{}.{}.next",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let next = path.with_file_name(name);
    let written = fs::write(&next, bytes).and_then(|()| fs::rename(&next, path));
    if written.is_err() {
        let _ = fs::remove_file(&next);
    }
    written
}

impl Store {
    /// `ledger.open` of this store: in the state directory it was opened on.
    fn open_file(&self) -> PathBuf {
        self.state_dir().join(OPEN)
    }

    /// Keeps `ledger.open` after this handle kept the write of `grant`: the grant is named with its
    /// snapshot while open, and leaves the file once closed. The file's position stays: the write
    /// is after it, and the next pass applies it again. A file that is not there is left to the
    /// next pass.
    pub(crate) fn opened_put(&self, grant: &HostGrantSnapshot) {
        let path = self.open_file();
        let Some(mut open) = read_open(&path) else {
            return;
        };
        let stream = stream_id(&grant.data.grant_id.0);
        if open_state(grant.state) {
            open.grants.insert(stream, encode(grant));
        } else {
            open.grants.remove(&stream);
        }
        write_open(&path, &open);
    }

    /// Keeps `ledger.open` after this handle deleted grant `id`.
    pub(crate) fn opened_delete(&self, id: &GrantId) {
        let path = self.open_file();
        if let Some(mut open) = read_open(&path)
            && open.grants.remove(&stream_id(&id.0)).is_some()
        {
            write_open(&path, &open);
        }
    }

    /// The open grants of the store, with how many grants were read for them: each grant of
    /// `ledger.open`, and of each grant the feed committed after the file's position its newest
    /// record; the whole feed's grants where the file is not there or does not read. The file is
    /// written again at the position read up to when that moved. A grant whose record this build
    /// cannot read is kept for [`Store::check`].
    pub(crate) fn open_grants(&self) -> (Vec<HostGrantSnapshot>, usize) {
        let path = self.open_file();
        let held = read_open(&path);
        let from = held.as_ref().map_or(0, |open| open.position);
        let mut open = held.unwrap_or_default();
        let mut read = open.grants.len();
        let mut after = Vec::new();
        let position = self.feed(&self.tenant, from, HOST_GRANT, |event| {
            if event.stream_type == HOST_GRANT {
                after.push(event);
            }
        });
        let position = match position {
            Ok(position) => position,
            Err(failure) => {
                self.fail(failure);
                return (Vec::new(), read);
            }
        };
        let mut newest: BTreeMap<String, Result<Option<Value>, StoreError>> = BTreeMap::new();
        for event in after {
            newest.insert(event.stream_id.clone(), super::body(HOST_GRANT, event));
        }
        read += newest.len();
        for (stream, held) in newest {
            match held {
                Ok(Some(body)) => match decode(&body) {
                    Ok(grant) if open_state(grant.state) => {
                        open.grants.insert(stream, body);
                    }
                    Ok(_) => {
                        open.grants.remove(&stream);
                    }
                    Err(detail) => {
                        self.fail(StoreError::Undecodable {
                            entity: HOST_GRANT,
                            stream,
                            detail,
                        });
                    }
                },
                Ok(None) => {
                    open.grants.remove(&stream);
                }
                Err(unreadable) => self.fail(unreadable),
            }
        }
        let mut grants = Vec::new();
        for (stream, body) in &open.grants {
            match decode(body) {
                Ok(grant) => grants.push(grant),
                Err(detail) => self.fail(StoreError::Undecodable {
                    entity: HOST_GRANT,
                    stream: stream.clone(),
                    detail,
                }),
            }
        }
        if (position != from || !path.exists()) && self.check().is_ok() {
            open.position = position;
            write_open(&path, &open);
        }
        (grants, read)
    }
}

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
        "actor": crate::actor::to_json(&data.actor),
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
        "cache_hits": data.cache_hits,
        "cache_misses": data.cache_misses,
        "lane": data.lane.map(lane_name),
        "lane_overrun": data.lane_overrun,
    })
}

fn decode(body: &Value) -> Result<HostGrantSnapshot, String> {
    let state = text(body, "state")?;
    let kind = text(body, "kind")?;
    Ok(HostGrantSnapshot {
        state: state_named(&state).ok_or_else(|| format!("unknown HostGrant state {state:?}"))?,
        data: HostGrantData {
            actor: crate::actor::stored(body)?,
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
            cache_hits: optional_integer(body, "cache_hits")?,
            cache_misses: optional_integer(body, "cache_misses")?,
            lane: match optional_text(body, "lane")? {
                Some(lane) => {
                    Some(lane_named(&lane).ok_or_else(|| format!("unknown BuildLane {lane:?}"))?)
                }
                None => None,
            },
            lane_overrun: match body.get("lane_overrun") {
                None | Some(Value::Null) => None,
                Some(Value::Bool(overrun)) => Some(*overrun),
                Some(_) => return Err("no boolean or null field \"lane_overrun\"".to_owned()),
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
