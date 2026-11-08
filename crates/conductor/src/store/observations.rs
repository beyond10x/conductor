//! The observations (`story:observation-retention`): which records they are, the tenant each one
//! is kept in, and how many snapshots' observations the store keeps.
//!
//! They are every `*Observation` entity of `spec/domains/observation.yaml` ([`ENTITIES`]), each
//! instance in the tenant of the snapshot its record names, `<instance>/observations/<snapshot>`
//! ([`tenant`]), the snapshot's id as the store writes a stream id. The `Snapshot` itself stays in
//! the instance's tenant, as every other record does.
//!
//! [`Store::retain`] runs when a snapshot completes. It keeps the observations of the newest
//! `keep` complete snapshots: newest by the instant they started, and of equal starts the one
//! stored later, as `repository activity` picks the newest. It drops the tenants of every other
//! complete snapshot, and of every failed snapshot that started before the oldest one kept. A
//! snapshot still collecting keeps its observations. A tenant is dropped with eventlog's own
//! erasure, `forget_tenant`, in one transaction. The `Snapshot` records stay.
//!
//! The tree before the database kept each snapshot's observations under `state/observations/`,
//! in a tenant named after the snapshot ([`tree_tenant`], [`tenants`]); `conductor store migrate`
//! reads them from there.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::behaviour::SnapshotStorage;
use conductor_model::observation::{SnapshotId, SnapshotState};
use eventlog_core::{EventLogError, TenantId};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::snapshot::{
    BLOCKER, COLLECTOR_RUN, MEMBER_FAILURE, MERGED_PULL_REQUEST, PULL_REQUEST, RELEASE, REPOSITORY,
    SECRET_SYNC, SESSION, SPECIFICATION, USAGE, WORKFLOW_RUN,
};
use super::{Store, backend, stream_id};

/// The entities kept per snapshot: every `*Observation` of the observation domain.
pub const ENTITIES: [&str; 12] = [
    REPOSITORY,
    PULL_REQUEST,
    RELEASE,
    MERGED_PULL_REQUEST,
    WORKFLOW_RUN,
    SESSION,
    BLOCKER,
    SPECIFICATION,
    MEMBER_FAILURE,
    COLLECTOR_RUN,
    SECRET_SYNC,
    USAGE,
];

/// Whether `entity`'s records are kept per snapshot.
#[must_use]
pub fn holds(entity: &str) -> bool {
    ENTITIES.contains(&entity)
}

/// The tenant the observations of the snapshot `snapshot` of `instance` are kept in.
///
/// # Errors
///
/// eventlog refuses the name: it is longer than a tenant may be.
pub fn tenant(instance: &str, snapshot: &str) -> Result<TenantId, EventLogError> {
    tenant_of_stream(instance, &stream_id(snapshot))
}

/// The tenant of the observations of the snapshot whose stream id is `stream`.
pub(super) fn tenant_of_stream(instance: &str, stream: &str) -> Result<TenantId, EventLogError> {
    TenantId::new(format!("{instance}/observations/{stream}"))
}

/// The tenant the tree before the database kept the observations of `snapshot` in.
///
/// # Errors
///
/// eventlog refuses the name; the form a stream id takes is always one it holds.
pub fn tree_tenant(snapshot: &str) -> Result<TenantId, EventLogError> {
    TenantId::new(stream_id(snapshot))
}

/// The tenants of the tree under `root`, by the names of their directories, in name order; none
/// where nothing is there. A directory whose name eventlog-tree would not have written is no
/// tenant.
pub(super) fn tenants(root: &Path) -> Result<Vec<TenantId>, EventLogError> {
    let entries = match fs::read_dir(root.join("tenants")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(backend(&error)),
    };
    let mut tenants = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| backend(&error))?;
        if entry.file_type().map_err(|error| backend(&error))?.is_dir()
            && let Some(tenant) = named(&entry.file_name())
        {
            tenants.push(tenant);
        }
    }
    tenants.sort_by(|one, other| one.as_str().cmp(other.as_str()));
    Ok(tenants)
}

/// The name eventlog-tree gives the directory of `value` (a tenant, a stream type or a stream
/// id): every byte outside `[A-Za-z0-9._-]` written `%XX`, and a leading `.` too.
pub(super) fn segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for (at, byte) in value.bytes().enumerate() {
        if (byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && !(at == 0 && byte == b'.')
        {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The tenant whose directory is named `name`, the inverse of [`segment`].
fn named(name: &OsStr) -> Option<TenantId> {
    let name = name.to_str()?;
    let mut bytes = Vec::with_capacity(name.len());
    let mut rest = name.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let hex = std::str::from_utf8(tail.get(..2)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    TenantId::new(String::from_utf8(bytes).ok()?).ok()
}

impl Store {
    /// Drops the observations retention does not keep, as the module says, keeping those of the
    /// newest `keep` complete snapshots, and answers the snapshots whose observations it dropped,
    /// oldest first. With `keep` 0 it drops nothing.
    ///
    /// # Errors
    ///
    /// The snapshots cannot be read for a reason other than a record this build cannot read (an
    /// unreadable `Snapshot` is left with its observations), or a tenant cannot be erased; the
    /// tenants erased before that stay erased.
    pub fn retain(&self, keep: usize) -> Result<Vec<SnapshotId>> {
        self.commit_observations()
            .context("commit the observations held back")?;
        let mut ordered: Vec<_> = SnapshotStorage::list(self)
            .into_iter()
            .enumerate()
            .collect();
        self.unreadable().context("read the snapshots")?;
        ordered.sort_by_key(|(stored, snapshot)| {
            (
                OffsetDateTime::parse(&snapshot.data.started_at.0, &Rfc3339).ok(),
                *stored,
            )
        });
        let complete: Vec<usize> = ordered
            .iter()
            .enumerate()
            .filter(|(_, (_, snapshot))| snapshot.state == SnapshotState::Complete)
            .map(|(at, _)| at)
            .collect();
        let kept = &complete[complete.len().saturating_sub(keep)..];
        let Some(&oldest) = kept.first() else {
            return Ok(Vec::new());
        };
        let dropped: Vec<SnapshotId> = ordered
            .into_iter()
            .enumerate()
            .filter(|(at, (_, snapshot))| match snapshot.state {
                SnapshotState::Complete => !kept.contains(at),
                SnapshotState::Failed => *at < oldest,
                SnapshotState::Collecting => false,
            })
            .map(|(_, (_, snapshot))| snapshot.data.snapshot_id)
            .collect();
        let mut erased = Vec::new();
        for snapshot in dropped {
            let tenant = self.observation_tenant(&snapshot.0.0)?;
            let first = self
                .runtime
                .block_on(self.log.read_feed(&tenant, 0, 1))
                .with_context(|| format!("read the observations of snapshot {}", snapshot.0.0))?;
            if first.events.is_empty() {
                continue;
            }
            self.runtime
                .block_on(self.log.forget_tenant(&tenant))
                .with_context(|| format!("drop the observations of snapshot {}", snapshot.0.0))?;
            erased.push(snapshot);
        }
        Ok(erased)
    }
}
