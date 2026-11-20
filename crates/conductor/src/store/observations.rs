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
//! complete snapshot, and of every failed snapshot that started before the oldest one kept. It
//! keeps besides them the newest complete snapshot whose forge collectors all ended `Completed`
//! ([`forge_complete`]): a collector that did not answer fails only its own rows
//! (`story:collector-failure-stays-its-own`), and the readers that would take missing forge rows
//! for none (the dispatch cycle's review fullness, the `github` collector's reuse base) read that
//! one. A snapshot still collecting keeps its observations. A tenant is dropped with eventlog's own
//! erasure, `forget_tenant`, in one transaction. The `Snapshot` records stay.
//!
//! The tree before the database kept each snapshot's observations under `state/observations/`,
//! in a tenant named after the snapshot ([`tree_tenant`], [`tenants`]); `conductor store migrate`
//! reads them from there.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::behaviour::{CollectorRunObservationStorage, SnapshotStorage};
use conductor_model::observation::{
    CollectorRunOutcome, SnapshotId, SnapshotSnapshot, SnapshotState,
};
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

/// The collectors that read the forges, whose missing rows a reader could take for none.
pub const FORGE_COLLECTORS: [&str; 2] = ["github", "gitlab"];

impl Store {
    /// The ids, as text, of the snapshots holding a run of a forge collector ([`FORGE_COLLECTORS`]) that
    /// did not end `Completed`, among the runs the store keeps. A record that does not read is
    /// left out, as every port leaves it out.
    #[must_use]
    pub fn forge_unanswered(&self) -> BTreeSet<String> {
        CollectorRunObservationStorage::list(self)
            .into_iter()
            .filter(|held| FORGE_COLLECTORS.contains(&held.data.collector.as_str()))
            .filter(|held| held.data.outcome != CollectorRunOutcome::Completed)
            .map(|held| held.data.snapshot_id.0.0)
            .collect()
    }
}

/// Whether `snapshot` is complete and every forge collector of it ended `Completed`: it names
/// none of them among its `failed_collectors`, and `unanswered` ([`Store::forge_unanswered`])
/// does not hold it (`story:collector-failure-stays-its-own`).
#[must_use]
pub fn forge_complete(snapshot: &SnapshotSnapshot, unanswered: &BTreeSet<String>) -> bool {
    snapshot.state == SnapshotState::Complete
        && !snapshot
            .data
            .failed_collectors
            .iter()
            .any(|name| FORGE_COLLECTORS.contains(&name.as_str()))
        && !unanswered.contains(&snapshot.data.snapshot_id.0.0)
}

impl Store {
    /// Drops the observations retention does not keep, as the module says, keeping those of the
    /// newest `keep` complete snapshots and of the newest complete one whose forge collectors all
    /// ended `Completed` ([`forge_complete`]), and answers the snapshots whose observations it
    /// dropped, oldest first. With `keep` 0 it drops nothing.
    ///
    /// # Errors
    ///
    /// The snapshots cannot be read for a reason other than a record this build cannot read (an
    /// unreadable `Snapshot` is left with its observations), or a tenant cannot be erased; the
    /// tenants erased before that stay erased.
    pub fn retain(&self, keep: usize) -> Result<Vec<SnapshotId>> {
        self.commit_observations()
            .context("commit the observations held back")?;
        // Each snapshot brings the index of the instance's streams up to date, so a list reads
        // at most one cycle's records past it (`story:status-current-snapshot`). The index is
        // optional: a list reads the feed where it is behind or missing.
        let _ = self.index();
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
        let unanswered = self.forge_unanswered();
        let forge_base = complete
            .iter()
            .rev()
            .copied()
            .find(|&at| forge_complete(&ordered[at].1, &unanswered));
        let dropped: Vec<SnapshotId> = ordered
            .into_iter()
            .enumerate()
            .filter(|(at, (_, snapshot))| match snapshot.state {
                SnapshotState::Complete => !kept.contains(at) && forge_base != Some(*at),
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
            self.forgot(&tenant);
            erased.push(snapshot);
        }
        Ok(erased)
    }
}
