//! Adversary case for `story:snapshot-driver`, wave 05, pass 1, against `story:store-hardening`'s
//! change to `store.rs` (both on `wave/05`): one observation record the store cannot read at all
//! (a later schema version) hides only itself in each of the six generated queries the snapshot
//! views run.
//!
//! On the `impl/snapshot-driver` branch alone this case is red, because the store-hardening change
//! is not under it; it is meant for `wave/05`.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store::{self, StoreError};
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::{
    BlockersQuery, PullRequestsQuery, RepositoriesQuery, SessionsQuery, SnapshotsQuery,
    WorkflowRunsQuery,
};
use conductor_model::observation::{
    CommitSha, Harness, Mergeability, RecordBlocker, RecordPullRequest, RecordRepository,
    RecordSession, RecordWorkflowRun, RepositoryName, RunConclusion, SessionState, SnapshotState,
    Visibility,
};
use conductor_model::primitives::Timestamp;
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, NewEvent, StreamId, TenantId, new_event_id,
    request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_snapshot_store")
        .join(case)
        .join("state");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(&dir).expect("create the case's directory");
    dir
}

/// Appends one `stored` event at schema `version` with `body` to the new stream `key` of `entity`,
/// where the store keeps it (`story:observation-retention`): an observation in the tenant of the
/// snapshot its body names under `state/observations/`, every other record in `state/tree/`.
fn append_raw(state: &Path, entity: &str, key: &str, version: u32, body: Value) {
    let observed = entity.starts_with("conductor.observation.") && entity.ends_with("Observation");
    let (dir, tenant) = if observed {
        (
            state.join("observations"),
            body["snapshot_id"]
                .as_str()
                .expect("an observation names its snapshot")
                .to_owned(),
        )
    } else {
        (state.join("tree"), "conductor".to_owned())
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the raw log");
    runtime.block_on(async {
        let log = TreeEventStore::open(&dir).await.expect("open the raw log");
        let tenant = TenantId::new(tenant).expect("tenant");
        let stream = StreamId::new(tenant, entity, key).expect("stream");
        let events = [NewEvent::new("stored", version, body).expect("event")];
        let request = new_event_id();
        let meta = CommandMeta {
            idempotency_key: request.clone(),
            request_hash: request_hash(&events).expect("hash"),
            subject: "conductor".to_owned(),
            actor: "conductor".to_owned(),
            request_id: request.clone(),
            trace_id: request,
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::now_utc(),
            claim: None,
        };
        log.append(&stream, Expected::NoStream, &events, &meta)
            .await
            .expect("append the raw record");
    });
}

/// Records one observation of each kind the six views show.
fn everything() -> Collector {
    Collector::new("everything", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let alpha = RepositoryName("alpha".to_owned());
        record.repository(RecordRepository {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            visibility: Visibility::Public,
            archived: false,
            local_checkout: true,
            main_head: CommitSha("0a1b2c3".to_owned()),
            main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
            main_commits_7d: 1,
            real_commits_7d: 1,
            behind_main: 0,
            dirty_files: 0,
            worktrees: 1,
            open_issues: 0,
            oldest_open_issue_at: None,
            latest_release: None,
            latest_release_at: None,
            unreleased_commits: None,
            planning_store_version: None,
            in_catalog: Some(true),
        })?;
        record.pull_request(RecordPullRequest {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            number: 7,
            title: "Seven".to_owned(),
            draft: false,
            mergeability: Mergeability::Mergeable,
            failing_checks: 0,
            opened_at: Timestamp("2026-09-01T00:00:00Z".to_owned()),
            updated_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        })?;
        record.workflow_run(RecordWorkflowRun {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            workflow: "check".to_owned(),
            conclusion: RunConclusion::Success,
            run_at: Timestamp("2026-10-02T00:00:00Z".to_owned()),
        })?;
        record.session(RecordSession {
            snapshot_id: snapshot.clone(),
            harness: Harness::Claude,
            session_ref: "session-1".to_owned(),
            name: None,
            cwd: "~/example-org/alpha".to_owned(),
            repository: Some(alpha.clone()),
            activity: SessionState::Idle,
        })?;
        record.blocker(RecordBlocker {
            snapshot_id: snapshot,
            repository: alpha,
            reference: "blocker:one".to_owned(),
            blocker_kind: "decision".to_owned(),
            title: "One".to_owned(),
        })?;
        Ok(())
    })
}

/// A later build's record of each kind: well-formed, at schema 2, which this build cannot read.
fn later_schema(snapshot: &str) -> [(&'static str, &'static str, Value); 6] {
    let key = |n: u8| format!("0199f000-0000-7000-8000-0000000000b{n}");
    [
        (
            "conductor.observation.Snapshot",
            "0199f000-0000-7000-8000-0000000000b0",
            json!({"state": "Complete", "snapshot_id": key(0),
                   "started_at": "2026-10-07T00:00:00Z", "disk_free_bytes": 1}),
        ),
        (
            "conductor.observation.RepositoryObservation",
            "0199f000-0000-7000-8000-0000000000b1",
            json!({"state": "Recorded", "observation_id": key(1), "snapshot_id": snapshot}),
        ),
        (
            "conductor.observation.PullRequestObservation",
            "0199f000-0000-7000-8000-0000000000b2",
            json!({"state": "Recorded", "observation_id": key(2), "snapshot_id": snapshot}),
        ),
        (
            "conductor.observation.WorkflowRunObservation",
            "0199f000-0000-7000-8000-0000000000b3",
            json!({"state": "Recorded", "observation_id": key(3), "snapshot_id": snapshot}),
        ),
        (
            "conductor.observation.SessionObservation",
            "0199f000-0000-7000-8000-0000000000b4",
            json!({"state": "Recorded", "observation_id": key(4), "snapshot_id": snapshot}),
        ),
        (
            "conductor.observation.BlockerObservation",
            "0199f000-0000-7000-8000-0000000000b5",
            json!({"state": "Recorded", "observation_id": key(5), "snapshot_id": snapshot}),
        ),
    ]
}

/// Each of the six queries answers the readable row beside one record at a later schema version,
/// and the handle reports a record that did not decode.
#[test]
fn adv_a_later_schema_record_hides_only_itself_in_each_query() {
    let state = state_dir("later-schema");
    let taken = snapshot::take(
        store::open(&state).expect("open the store"),
        &[everything()],
    )
    .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    for (entity, key, body) in later_schema(&taken.snapshot_id.0.0) {
        append_raw(&state, entity, key, 2, body);
    }

    let generated = Generated::new(store::open_existing(&state).expect("reopen the store"));
    let answered = [
        ("Snapshots", generated.snapshots().map(|rows| rows.len())),
        (
            "Repositories",
            generated.repositories().map(|rows| rows.len()),
        ),
        (
            "PullRequests",
            generated.pull_requests().map(|rows| rows.len()),
        ),
        (
            "WorkflowRuns",
            generated.workflow_runs().map(|rows| rows.len()),
        ),
        ("Sessions", generated.sessions().map(|rows| rows.len())),
        ("Blockers", generated.blockers().map(|rows| rows.len())),
    ];
    let hidden: Vec<String> = answered
        .iter()
        .filter(|(_, rows)| rows.as_ref().ok() != Some(&1))
        .map(|(query, rows)| format!("{query} answered {rows:?} rows, not the 1 readable"))
        .collect();
    assert!(
        hidden.is_empty(),
        "one later-schema record hid more than itself:\n  - {}",
        hidden.join("\n  - ")
    );
    let check = generated.ports.check();
    assert!(
        matches!(check, Err(StoreError::Undecodable { .. })),
        "the unreadable record is reported: {check:?}"
    );
}

/// `story:catalog-source`: a repository observation an earlier build wrote names its catalog
/// membership `in_atlas_catalog`, a Boolean; the field is `in_catalog` now, and optional. Such a
/// record still reads, its Boolean as the membership; one this build writes reads back what it
/// holds, `null` or absent as unknown. No record is reported undecodable.
#[test]
fn a_repository_observation_an_earlier_build_wrote_reads_its_catalog_membership() {
    let state = state_dir("catalog-membership");
    let taken = snapshot::take(
        store::open(&state).expect("open the store"),
        &[everything()],
    )
    .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = taken.snapshot_id.0.0.clone();
    let cases = [
        (
            "c1",
            Some(("in_atlas_catalog", Value::Bool(true))),
            Some(true),
        ),
        (
            "c2",
            Some(("in_atlas_catalog", Value::Bool(false))),
            Some(false),
        ),
        ("c3", Some(("in_catalog", Value::Bool(false))), Some(false)),
        ("c4", Some(("in_catalog", Value::Null)), None),
        ("c5", None, None),
    ];
    let key = |n: &str| format!("0199f000-0000-7000-8000-0000000000{n}");
    for (n, membership, _) in &cases {
        let mut body = json!({
            "state": "Recorded", "observation_id": key(n), "snapshot_id": snapshot,
            "repository": format!("repository-{n}"), "visibility": "Public",
            "archived": false, "local_checkout": true, "main_head": "0a1b2c3",
            "main_committed_at": "2026-10-01T00:00:00Z", "main_commits_7d": 1,
            "real_commits_7d": 1, "behind_main": 0, "dirty_files": 0, "worktrees": 1,
            "open_issues": 0, "oldest_open_issue_at": null, "latest_release": null,
            "latest_release_at": null, "unreleased_commits": null,
            "planning_store_version": null,
        });
        if let Some((field, value)) = membership {
            body[*field] = value.clone();
        }
        append_raw(
            &state,
            "conductor.observation.RepositoryObservation",
            &key(n),
            1,
            body,
        );
    }

    let generated = Generated::new(store::open_existing(&state).expect("reopen the store"));
    let rows = generated
        .repositories()
        .expect("the repositories view answers");
    let read: Vec<(String, Option<bool>)> = rows
        .iter()
        .map(|row| (row.repository.0.clone(), row.in_catalog))
        .collect();
    let mut expected = vec![("alpha".to_owned(), Some(true))];
    expected.extend(
        cases
            .iter()
            .map(|(n, _, membership)| (format!("repository-{n}"), *membership)),
    );
    assert_eq!(read, expected);
    let check = generated.ports.check();
    assert!(check.is_ok(), "every record decodes: {check:?}");
}
