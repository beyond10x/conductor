//! `HostGrantStorage` on [`super::Store`] (`story:build-run`): each `conductor.host.HostGrant` is
//! one stream, keyed by its grant id, kept and read back as every other entity of the store is.
//! Every move of a grant appends its new snapshot, so the stream is the grant's history: asked,
//! answered, started, its peak, released.
//!
//! **The open grants** (`story:build-queue-convoy`). Beside the host store, `ledger.open` holds the
//! grants still open (`Requested`, `Granted`, `Running`), each with its snapshot and the length of
//! its stream when the snapshot was written, and how many grants the store holds. A pass of the
//! ledger reads that file ([`Store::open_grants`]), never the closed grants, which outnumber the
//! open ones by thousands on a host of long standing. Each write of a grant through the host's
//! ports keeps the file ([`Store::opened_put`]), written whole and renamed over the old one. The file
//! is trusted only as far as the store's files bear it out: an entry whose stream has grown since
//! is read from the store; a store holding another number of grants (one created by a writer that
//! keeps no file, an older binary's) makes the pass read every grant once and write the file
//! anew. A grant leaves the file only when a write closes it, and a closed grant never opens
//! again, so a grant the file does not name, while the count holds, is closed.

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

use super::observations::segment;
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
    /// The grants the store held when the file was written: its streams of grants.
    streams: u64,
    /// Each open grant, by its id.
    grants: BTreeMap<String, Entry>,
}

/// One open grant of `ledger.open`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Entry {
    /// The length of the grant's stream when `grant` was written: its events.
    events: u64,
    /// The grant's snapshot, as the store keeps it.
    grant: Value,
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

/// Writes `open` to `path` whole: a new file, renamed over the old.
fn write_open(path: &Path, open: &Open) {
    let Ok(text) = serde_json::to_vec(open) else {
        return;
    };
    let _ = replace(path, &text);
}

/// `n` as a count of the store's files.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Removes `ledger.open` at `path`: the next pass reads every grant and writes it anew.
fn drop_open(path: &Path) {
    let _ = fs::remove_file(path);
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
    /// The directory of the grants' streams in `tree/`.
    fn grant_streams_dir(&self) -> PathBuf {
        self.tree
            .root
            .join("tenants")
            .join(segment(self.tenant.as_str()))
            .join("streams")
            .join(segment(HOST_GRANT))
    }

    /// How many grants the store holds: its streams of grants, each a directory.
    fn grant_streams(&self) -> io::Result<u64> {
        match fs::read_dir(self.grant_streams_dir()) {
            Ok(entries) => Ok(count(entries.filter(Result::is_ok).count())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error),
        }
    }

    /// The length of the stream of grant `id` in the store's files: its event files, one per event
    /// committed; `None` when the directory does not read.
    fn grant_events(&self, id: &str) -> Option<u64> {
        let directory = self.grant_streams_dir().join(segment(&stream_id(id)));
        match fs::read_dir(directory) {
            Ok(entries) => Some(count(
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                    .count(),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Some(0),
            Err(_) => None,
        }
    }

    /// `ledger.open` of this store: beside `tree/`.
    fn open_file(&self) -> PathBuf {
        self.state_dir().join(OPEN)
    }

    /// Keeps `ledger.open` after this handle kept the write of `grant`: the grant is named with its
    /// snapshot while open, and leaves the file once closed. A file that is not there is left to
    /// the next pass; one that counts another number of grants than the store holds, less one this
    /// write created, is removed, for the next pass to write anew.
    pub(crate) fn opened_put(&self, grant: &HostGrantSnapshot) {
        let id = &grant.data.grant_id.0;
        let path = self.open_file();
        let version = self
            .versions
            .borrow()
            .get(&(HOST_GRANT, id.clone()))
            .copied();
        let (Some(version), Some(mut open)) = (version, read_open(&path)) else {
            return;
        };
        let created = u64::from(version == 1);
        match self.grant_streams() {
            Ok(streams) if open.streams + created == streams => open.streams = streams,
            _ => {
                drop_open(&path);
                return;
            }
        }
        if open_state(grant.state) {
            open.grants.insert(
                id.clone(),
                Entry {
                    events: version,
                    grant: encode(grant),
                },
            );
        } else {
            open.grants.remove(id);
        }
        write_open(&path, &open);
    }

    /// Keeps `ledger.open` after this handle deleted grant `id`.
    pub(crate) fn opened_delete(&self, id: &GrantId) {
        let path = self.open_file();
        if let Some(mut open) = read_open(&path)
            && open.grants.remove(&id.0).is_some()
        {
            write_open(&path, &open);
        }
    }

    /// The open grants of the store, from `ledger.open` where the store's files bear it out, with
    /// how many grants were read for them: each entry of the file, and every grant of the store
    /// when the file is not there or counts another number of grants than the store holds (it is
    /// then written anew). An entry whose stream has grown since it was written is read from the
    /// store, and the file is written again.
    pub(crate) fn open_grants(&self) -> (Vec<HostGrantSnapshot>, usize) {
        let path = self.open_file();
        let streams = self.grant_streams();
        if let (Ok(streams), Some(open)) = (&streams, read_open(&path))
            && open.streams == *streams
        {
            let mut grants = Vec::new();
            let mut kept = BTreeMap::new();
            let mut behind = false;
            for (id, entry) in &open.grants {
                let current = self.grant_events(id) == Some(entry.events);
                let held = current
                    .then(|| decode(&entry.grant).ok())
                    .flatten()
                    .filter(|grant| grant.data.grant_id.0 == *id);
                let (grant, events) = match held {
                    Some(grant) => (Some(grant), entry.events),
                    None => {
                        behind = true;
                        let grant = HostGrantStorage::get(self, &GrantId(id.clone()));
                        let events = self
                            .versions
                            .borrow()
                            .get(&(HOST_GRANT, id.clone()))
                            .copied()
                            .unwrap_or(0);
                        (grant, events)
                    }
                };
                if let Some(grant) = grant.filter(|grant| open_state(grant.state)) {
                    kept.insert(
                        id.clone(),
                        Entry {
                            events,
                            grant: encode(&grant),
                        },
                    );
                    grants.push(grant);
                }
            }
            if behind && self.check().is_ok() {
                write_open(
                    &path,
                    &Open {
                        streams: *streams,
                        grants: kept,
                    },
                );
            }
            return (grants, open.grants.len());
        }
        let all = HostGrantStorage::list(self);
        let read = all.len();
        let grants: Vec<HostGrantSnapshot> = all
            .into_iter()
            .filter(|grant| open_state(grant.state))
            .collect();
        if let Ok(streams) = streams
            && self.check().is_ok()
        {
            let mut open = Open {
                streams,
                grants: BTreeMap::new(),
            };
            for grant in &grants {
                let id = &grant.data.grant_id.0;
                let Some(events) = self.grant_events(id) else {
                    return (grants, read);
                };
                open.grants.insert(
                    id.clone(),
                    Entry {
                        events,
                        grant: encode(grant),
                    },
                );
            }
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
