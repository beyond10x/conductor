//! A failed snapshot, for a case that reads past one. The driver completes a snapshot whatever
//! its collectors answer (`story:collector-failure-stays-its-own`), so a case stores a failed one
//! through the store's own ports, as a run that stopped and `fail-snapshot` leave it, with the
//! observations it recorded before it stopped.

#![allow(dead_code)]

use std::path::Path;

use conductor_cli::store::{self, Store};
use conductor_model::behaviour::{
    MergedPullRequestObservationStorage, ReleaseObservationStorage, RepositoryObservationStorage,
    SnapshotStorage,
};
use conductor_model::observation::{
    MergedPullRequestObservationData, MergedPullRequestObservationSnapshot,
    MergedPullRequestObservationState, ObservationId, RecordMergedPullRequest, RecordRelease,
    RecordRepository, ReleaseObservationData, ReleaseObservationSnapshot, ReleaseObservationState,
    RepositoryObservationData, RepositoryObservationSnapshot, RepositoryObservationState,
    SnapshotData, SnapshotId, SnapshotSnapshot, SnapshotState,
};
use conductor_model::primitives::{Timestamp, Uuid};

/// A fresh id for a stored record.
fn fresh() -> Uuid {
    Uuid(eventlog_core::new_event_id())
}

/// Stores a snapshot that started at `started_at` and failed with `reason` in the store under
/// `state`, with what `record` stores into it ([`repository`], [`release`],
/// [`merged_pull_request`]); answers the snapshot.
pub fn snapshot(
    state: &Path,
    started_at: &str,
    reason: &str,
    record: impl FnOnce(&mut Store, &SnapshotId),
) -> SnapshotId {
    let mut store = store::open(state).expect("open the store");
    let snapshot = SnapshotId(fresh());
    SnapshotStorage::put(
        &mut store,
        SnapshotSnapshot {
            state: SnapshotState::Failed,
            data: SnapshotData {
                actor: conductor_cli::actor::daemon(),
                snapshot_id: snapshot.clone(),
                started_at: Timestamp(started_at.to_owned()),
                disk_free_bytes: 1_000_000,
                failure_reason: Some(reason.to_owned()),
                failed_collectors: Vec::new(),
            },
        },
    );
    record(&mut store, &snapshot);
    store.check().expect("the store kept the failed snapshot");
    snapshot
}

/// Stores `input` as an observation of `snapshot`.
pub fn repository(store: &mut Store, snapshot: &SnapshotId, input: RecordRepository) {
    RepositoryObservationStorage::put(
        store,
        RepositoryObservationSnapshot {
            state: RepositoryObservationState::Recorded,
            data: RepositoryObservationData {
                actor: input.actor,
                observation_id: ObservationId(fresh()),
                snapshot_id: snapshot.clone(),
                repository: input.repository,
                visibility: input.visibility,
                archived: input.archived,
                local_checkout: input.local_checkout,
                main_head: input.main_head,
                main_committed_at: input.main_committed_at,
                main_commits_7d: input.main_commits_7d,
                real_commits_7d: input.real_commits_7d,
                behind_main: input.behind_main,
                dirty_files: input.dirty_files,
                worktrees: input.worktrees,
                open_issues: input.open_issues,
                oldest_open_issue_at: input.oldest_open_issue_at,
                latest_release: input.latest_release,
                latest_release_at: input.latest_release_at,
                unreleased_commits: input.unreleased_commits,
                planning_store_version: input.planning_store_version,
                in_catalog: input.in_catalog,
                gates_hooks_version: input.gates_hooks_version,
                gates_version: input.gates_version,
                wave_width: input.wave_width,
                wave_ready: input.wave_ready,
                wave_reason: input.wave_reason,
            },
        },
    );
}

/// Stores `input` as an observation of `snapshot`.
pub fn release(store: &mut Store, snapshot: &SnapshotId, input: RecordRelease) {
    ReleaseObservationStorage::put(
        store,
        ReleaseObservationSnapshot {
            state: ReleaseObservationState::Recorded,
            data: ReleaseObservationData {
                actor: input.actor,
                observation_id: ObservationId(fresh()),
                snapshot_id: snapshot.clone(),
                repository: input.repository,
                tag: input.tag,
                published_at: input.published_at,
                url: input.url,
            },
        },
    );
}

/// Stores `input` as an observation of `snapshot`.
pub fn merged_pull_request(
    store: &mut Store,
    snapshot: &SnapshotId,
    input: RecordMergedPullRequest,
) {
    MergedPullRequestObservationStorage::put(
        store,
        MergedPullRequestObservationSnapshot {
            state: MergedPullRequestObservationState::Recorded,
            data: MergedPullRequestObservationData {
                actor: input.actor,
                observation_id: ObservationId(fresh()),
                snapshot_id: snapshot.clone(),
                repository: input.repository,
                number: input.number,
                title: input.title,
                merged_at: input.merged_at,
                url: input.url,
            },
        },
    );
}
