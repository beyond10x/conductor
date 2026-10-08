//! Adversary cases for `story:snapshot-driver`, wave 05, pass 1: the collector seam (a collector
//! that panics, one that records into a snapshot it was not given, the registered order), the
//! lifecycle through the binary (a complete snapshot, a snapshot a crash left collecting), the
//! views (an unreadable record, a missing state directory, stable output) and the free-disk read
//! (a `df` that cannot answer, which is never run; a full disk).
//!
//! Each case keeps its store under `state/` in its own directory in this test target's temporary
//! directory, and runs the binary from that directory's empty `work/` with `--state-dir`.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::Result;
use conductor_cli::collect::{self, Collector};
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::{
    BlockerObservationStorage, PullRequestObservationStorage, RepositoryObservationStorage,
    SessionObservationStorage, SpecificationObservationStorage, WorkflowRunObservationStorage,
};
use conductor_model::observation::{
    CommitSha, Harness, Mergeability, RecordBlocker, RecordPullRequest, RecordRepository,
    RecordSession, RecordSpecification, RecordWorkflowRun, RepositoryName, RunConclusion,
    SessionState, SnapshotId, SnapshotState, SpecificationPresence, ValidationResult, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, NewEvent, StreamId, TenantId, new_event_id,
    request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

/// `conductor.observation.Snapshot`, as the store types its streams.
const SNAPSHOT: &str = "conductor.observation.Snapshot";

/// A snapshot a crashed run left `Collecting`.
const STALE: &str = "0199f000-0000-7000-8000-000000000001";

/// The six views of this group that `story:snapshot-driver` implements.
const VIEWS: [&str; 6] = [
    "snapshots",
    "repositories",
    "pull-requests",
    "workflow-runs",
    "sessions",
    "blockers",
];

fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_snapshot")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("work")).expect("create the case's directories");
    dir
}

fn state(root: &Path) -> PathBuf {
    root.join("state")
}

/// A `PATH` for `start-snapshot` that holds, for each program the collectors start, a fake that
/// prints `offline` on standard error and exits 1: the free disk is read through `statvfs`, the
/// start-time check finds every program (`story:install-prerequisites`), and the registered
/// collectors reach no live source. The `repositories` collector fails at its first command, so
/// nothing asks GitHub and nothing runs `git fetch` in a real checkout.
fn offline(root: &Path) -> PathBuf {
    let bin = root.join("offline");
    fs::create_dir_all(&bin).expect("create the offline PATH");
    for program in ["git", "gh", "aep", "ess", "claude"] {
        let fake = bin.join(program);
        fs::write(&fake, "#!/bin/sh
echo offline >&2
exit 1
").expect("write a fake");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }
    bin
}

/// Takes one snapshot into the store under `root` through `collectors`.
fn take(root: &Path, collectors: &[Collector]) -> Result<Taken> {
    snapshot::take(
        store::open(&state(root)).expect("open the store"),
        collectors,
    )
}

/// Runs the binary from `root`'s `work/` with `--state-dir` naming `root`'s `state/`, and with
/// `PATH` replaced when `path_env` is given.
fn conductor_with(root: &Path, path_env: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
    command
        .current_dir(root.join("work"))
        // Never the operator's config: the built-in instance, with this case as home.
        .env("HOME", root)
        .env_remove("CONDUCTOR_CONFIG")
        .env_remove("CONDUCTOR_INSTANCE")
        .arg("--state-dir")
        .arg(state(root))
        .args(args)
        .stdin(Stdio::null());
    if let Some(path) = path_env {
        command.env("PATH", path);
    }
    command.output().expect("the conductor binary runs")
}

fn conductor(root: &Path, args: &[&str]) -> Output {
    conductor_with(root, None, args)
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The rows of `conductor snapshot <name> --format json`; asserts the view answered.
fn view(root: &Path, name: &str) -> Vec<Value> {
    let output = conductor(root, &["snapshot", name, "--format", "json"]);
    assert!(
        output.status.success(),
        "`snapshot {name}`: {}",
        describe(&output)
    );
    match serde_json::from_slice(&output.stdout) {
        Ok(Value::Array(rows)) => rows,
        other => panic!("`snapshot {name}` printed {}: {other:?}", describe(&output)),
    }
}

fn id(snapshot: &SnapshotId) -> &str {
    &snapshot.0.0
}

fn snapshot_id(text: &str) -> SnapshotId {
    SnapshotId(Uuid(text.to_owned()))
}

fn repository(snapshot: &SnapshotId, name: &str) -> RecordRepository {
    RecordRepository {
        snapshot_id: snapshot.clone(),
        repository: RepositoryName(name.to_owned()),
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
    }
}

/// A fake collector that records the repository `name` into the snapshot it is given.
fn records(name: &'static str) -> Collector {
    Collector::new("records", move |record: &mut Recorder<'_>| {
        let input = repository(record.snapshot(), name);
        record.repository(input)?;
        Ok(())
    })
}

/// A fake collector that records one observation of each kind the driver's views show.
fn everything() -> Collector {
    Collector::new("everything", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let alpha = RepositoryName("alpha".to_owned());
        record.repository(repository(&snapshot, "alpha"))?;
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
            name: Some("one".to_owned()),
            cwd: "~/example-org/alpha".to_owned(),
            repository: Some(alpha.clone()),
            role: None,
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

/// Appends one `stored` event at schema `version` with `body` to the new stream `key` of `entity`,
/// with the raw log: what a crashed run or another build leaves in the store. It is kept where the
/// store keeps it (`story:observation-retention`): an observation in the tenant of the snapshot
/// its body names under `state/observations/`, every other record in `state/tree/`.
fn append_raw(root: &Path, entity: &str, key: &str, version: u32, body: Value) {
    let observed = entity.starts_with("conductor.observation.") && entity.ends_with("Observation");
    let (dir, tenant) = if observed {
        (
            state(root).join("observations"),
            body["snapshot_id"]
                .as_str()
                .expect("an observation names its snapshot")
                .to_owned(),
        )
    } else {
        (state(root).join("tree"), "conductor".to_owned())
    };
    fs::create_dir_all(&dir).expect("create the store's directory");
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

/// Leaves snapshot `snapshot` `Collecting` in the store, as a run killed mid-way leaves it.
fn left_collecting(root: &Path, snapshot: &str) {
    append_raw(
        root,
        SNAPSHOT,
        snapshot,
        1,
        json!({
            "state": "Collecting",
            "snapshot_id": snapshot,
            "started_at": "2026-10-07T00:00:00Z",
            "disk_free_bytes": 1,
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// The collector seam
// ---------------------------------------------------------------------------------------------

/// A collector that panics did not answer: the snapshot ends `Failed` naming it, as one returning
/// an error does (`snapshot::take`: "`Failed` naming the first that did not [answer]"). The
/// collectors parse what `gh`, `git`, `aep` and the session lists print, where an index or an
/// `expect` on an answer they did not foresee panics.
#[test]
fn adv_a_collector_that_panics_fails_the_snapshot_naming_it() {
    let root = case_dir("panics");
    let panics = Collector::new("panics", |record: &mut Recorder<'_>| -> Result<()> {
        let answered: Vec<&str> = Vec::new();
        record.repository(repository(record.snapshot(), answered[0]))?;
        Ok(())
    });
    let answered = panic::catch_unwind(AssertUnwindSafe(|| {
        take(&root, &[records("alpha"), panics])
    }));

    let snapshots = view(&root, "snapshots");
    assert_eq!(snapshots.len(), 1, "{snapshots:?}");
    assert_eq!(
        snapshots[0]["state"],
        "Failed",
        "a collector that panicked left snapshot {} {}, and the driver {}",
        snapshots[0]["snapshot_id"],
        snapshots[0]["state"],
        if answered.is_ok() {
            "answered"
        } else {
            "unwound past it without ending it"
        }
    );
    let taken = answered
        .expect("the driver answers")
        .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    assert!(
        taken
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("panics: ")),
        "the reason names the collector that panicked: {taken:?}"
    );
}

/// A collector records into the snapshot its `Recorder` holds (`collect::Collect`: "it records
/// into the snapshot `Recorder` holds"). An input naming another snapshot that is still
/// `Collecting` — one a crashed run left — is refused or recorded under the run's own snapshot,
/// never filed under the other one.
#[test]
fn adv_a_collector_records_only_into_the_snapshot_it_is_given() {
    let root = case_dir("other-snapshot");
    left_collecting(&root, STALE);
    let stale = snapshot_id(STALE);
    let into_stale = Collector::new("stale", move |record: &mut Recorder<'_>| {
        record.repository(repository(&stale, "alpha"))?;
        Ok(())
    });
    let taken = take(&root, &[into_stale]).expect("the driver ends the snapshot");

    let misfiled: Vec<Value> = view(&root, "repositories")
        .into_iter()
        .filter(|row| row["snapshot_id"] == STALE)
        .collect();
    assert!(
        misfiled.is_empty(),
        "the collector of snapshot {} ({:?}) recorded into snapshot {STALE}, which its Recorder \
         does not hold: {misfiled:?}",
        id(&taken.snapshot_id),
        taken.state
    );
}

/// Wave 05 U6, the class of the case above: every `Recorder` method, not only `repository`,
/// records into the snapshot its `Recorder` holds, whatever snapshot its input names.
#[test]
fn adv_every_recorder_method_records_only_into_the_snapshot_it_is_given() {
    let root = case_dir("every-recorder-method");
    left_collecting(&root, STALE);
    let stale = snapshot_id(STALE);
    let into_stale = Collector::new("stale", move |record: &mut Recorder<'_>| {
        let alpha = RepositoryName("alpha".to_owned());
        record.repository(repository(&stale, "alpha"))?;
        record.pull_request(RecordPullRequest {
            snapshot_id: stale.clone(),
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
            snapshot_id: stale.clone(),
            repository: alpha.clone(),
            workflow: "check".to_owned(),
            conclusion: RunConclusion::Success,
            run_at: Timestamp("2026-10-02T00:00:00Z".to_owned()),
        })?;
        record.session(RecordSession {
            snapshot_id: stale.clone(),
            harness: Harness::Claude,
            session_ref: "session-1".to_owned(),
            name: None,
            cwd: "~/example-org/alpha".to_owned(),
            repository: Some(alpha.clone()),
            role: None,
            activity: SessionState::Idle,
        })?;
        record.blocker(RecordBlocker {
            snapshot_id: stale.clone(),
            repository: alpha.clone(),
            reference: "blocker:one".to_owned(),
            blocker_kind: "decision".to_owned(),
            title: "One".to_owned(),
        })?;
        record.specification(RecordSpecification {
            snapshot_id: stale.clone(),
            repository: alpha,
            presence: SpecificationPresence::Present,
            path: Some("spec/".to_owned()),
            format: None,
            required_ess: None,
            validation: ValidationResult::Valid,
            validation_refusals: 0,
            scenarios: None,
            synthesis_refusals: None,
            conformance_status: None,
        })?;
        Ok(())
    });
    let taken = take(&root, &[into_stale]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");

    let held = store::open_existing(&state(&root)).expect("reopen the store");
    let snapshots = |ids: Vec<SnapshotId>| -> Vec<String> {
        ids.into_iter().map(|snapshot| snapshot.0.0).collect()
    };
    let filed = [
        (
            "repository",
            snapshots(
                RepositoryObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
        (
            "pull_request",
            snapshots(
                PullRequestObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
        (
            "workflow_run",
            snapshots(
                WorkflowRunObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
        (
            "session",
            snapshots(
                SessionObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
        (
            "blocker",
            snapshots(
                BlockerObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
        (
            "specification",
            snapshots(
                SpecificationObservationStorage::list(&held)
                    .into_iter()
                    .map(|held| held.data.snapshot_id)
                    .collect(),
            ),
        ),
    ];
    held.check().expect("every observation reads back");
    let own = id(&taken.snapshot_id).to_owned();
    let misfiled: Vec<String> = filed
        .iter()
        .filter(|(_, ids)| *ids != [own.clone()])
        .map(|(method, ids)| format!("Recorder::{method} filed under {ids:?}"))
        .collect();
    assert!(
        misfiled.is_empty(),
        "the collector of snapshot {own} recorded outside it (stale {STALE}):\n  - {}",
        misfiled.join("\n  - ")
    );
}

/// `collect::registered` runs in the order its documentation gives, and `start-snapshot` with a
/// `gh` that fails fails naming the first of them, stopped at its first command.
#[test]
fn adv_the_registered_collectors_run_in_the_documented_order() {
    let names: Vec<&str> = collect::registered().iter().map(Collector::name).collect();
    assert_eq!(
        names,
        [
            "repositories",
            "github",
            "blockers",
            "specifications",
            "sessions"
        ],
        "the blockers run before the specifications, whose status reads the blockers' listing"
    );

    let root = case_dir("registered-order");
    let output = conductor_with(
        &root,
        Some(&offline(&root)),
        &["snapshot", "start-snapshot"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "snapshot {} failed: repositories: ",
            stdout(&output)
        )) && stderr.contains("`gh repo list")
            && stderr.contains("exited 1: offline"),
        "the first registered collector is the one named, stopped at its first command: {}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// The lifecycle through the binary
// ---------------------------------------------------------------------------------------------

/// The flags of one `record-*` command, without `--snapshot-id`.
fn record_flags() -> [(&'static str, Vec<&'static str>); 5] {
    [
        (
            "record-repository",
            vec![
                "--repository",
                "beta",
                "--visibility",
                "Private",
                "--archived",
                "true",
                "--local-checkout",
                "false",
                "--main-head",
                "ffeedd0",
                "--main-committed-at",
                "2026-10-03T04:05:06Z",
                "--main-commits-7d",
                "11",
                "--real-commits-7d",
                "12",
                "--behind-main",
                "13",
                "--dirty-files",
                "14",
                "--worktrees",
                "15",
                "--open-issues",
                "16",
                "--latest-release",
                "1.2.3",
                "--unreleased-commits",
                "17",
                "--planning-store-version",
                "aep.project/6",
                "--in-catalog",
                "false",
            ],
        ),
        (
            "record-pull-request",
            vec![
                "--repository",
                "beta",
                "--number",
                "42",
                "--title",
                "Forty-two",
                "--draft",
                "true",
                "--mergeability",
                "Conflicting",
                "--failing-checks",
                "3",
                "--opened-at",
                "2026-09-01T01:02:03Z",
                "--updated-at",
                "2026-10-04T05:06:07Z",
            ],
        ),
        (
            "record-workflow-run",
            vec![
                "--repository",
                "beta",
                "--workflow",
                "release",
                "--conclusion",
                "Failure",
                "--run-at",
                "2026-10-05T06:07:08Z",
            ],
        ),
        (
            "record-session",
            vec![
                "--harness",
                "Codex",
                "--session-ref",
                "session-9",
                "--name",
                "nine",
                "--cwd",
                "~/example-org/beta",
                "--repository",
                "beta",
                "--role",
                "conductor",
                "--activity",
                "Waiting",
            ],
        ),
        (
            "record-blocker",
            vec![
                "--repository",
                "beta",
                "--reference",
                "blocker:nine",
                "--blocker-kind",
                "question",
                "--title",
                "Nine",
            ],
        ),
    ]
}

/// Each `record-*` command run through the binary, into a snapshot a crash left `Collecting`,
/// prints the observation's id, and its view reads every flag back under the field it names.
/// `tests/snapshot.rs` runs only `record-repository` through the binary, while
/// `tests/adv_cli.rs` lists all five as decided by it.
#[test]
fn adv_each_record_command_reads_back_through_its_view() {
    let root = case_dir("record-commands");
    left_collecting(&root, STALE);
    let mut printed = Vec::new();
    for (command, flags) in record_flags() {
        let args = [&["snapshot", command, "--snapshot-id", STALE], &flags[..]].concat();
        let output = conductor(&root, &args);
        assert!(output.status.success(), "{command}: {}", describe(&output));
        printed.push((command, stdout(&output)));
    }
    let expected = [
        (
            "repositories",
            json!({
                "snapshot_id": STALE, "repository": "beta", "visibility": "Private",
                "archived": true, "local_checkout": false, "main_head": "ffeedd0",
                "main_committed_at": "2026-10-03T04:05:06Z", "main_commits_7d": 11,
                "real_commits_7d": 12, "behind_main": 13, "dirty_files": 14, "worktrees": 15,
                "open_issues": 16, "oldest_open_issue_at": null, "latest_release": "1.2.3",
                "latest_release_at": null, "unreleased_commits": 17,
                "planning_store_version": "aep.project/6", "in_catalog": false,
            }),
        ),
        (
            "pull-requests",
            json!({
                "snapshot_id": STALE, "repository": "beta", "number": 42, "title": "Forty-two",
                "draft": true, "mergeability": "Conflicting", "failing_checks": 3,
                "opened_at": "2026-09-01T01:02:03Z", "updated_at": "2026-10-04T05:06:07Z",
            }),
        ),
        (
            "workflow-runs",
            json!({
                "snapshot_id": STALE, "repository": "beta", "workflow": "release",
                "conclusion": "Failure", "run_at": "2026-10-05T06:07:08Z",
            }),
        ),
        (
            "sessions",
            json!({
                "snapshot_id": STALE, "harness": "Codex", "session_ref": "session-9",
                "name": "nine", "cwd": "~/example-org/beta", "repository": "beta",
                "role": "conductor", "activity": "Waiting",
            }),
        ),
        (
            "blockers",
            json!({
                "snapshot_id": STALE, "repository": "beta", "reference": "blocker:nine",
                "blocker_kind": "question", "title": "Nine",
            }),
        ),
    ];
    for ((name, mut want), (command, observation)) in expected.into_iter().zip(printed) {
        want["observation_id"] = Value::from(observation);
        let rows = view(&root, name);
        assert_eq!(rows, vec![want], "`{command}` read back through `{name}`");
    }
}

/// A complete snapshot takes no more observations through any `record-*` command and cannot be
/// completed or failed again; nothing it holds changes.
#[test]
fn adv_a_complete_snapshot_is_closed_to_every_command() {
    let root = case_dir("complete-closed");
    let taken = take(&root, &[records("alpha")]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = id(&taken.snapshot_id);

    let mut problems = Vec::new();
    let mut refused = |args: Vec<&str>| {
        let output = conductor(&root, &args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() != Some(1)
            || !stderr.contains("SnapshotNotCollecting")
            || stderr.lines().count() != 1
            || !output.stdout.is_empty()
        {
            problems.push(format!("`{}`: {}", args[1], describe(&output)));
        }
    };
    for (command, flags) in record_flags() {
        refused(
            [
                &["snapshot", command, "--snapshot-id", snapshot],
                &flags[..],
            ]
            .concat(),
        );
    }
    refused(vec![
        "snapshot",
        "complete-snapshot",
        "--snapshot-id",
        snapshot,
    ]);
    refused(vec![
        "snapshot",
        "fail-snapshot",
        "--snapshot-id",
        snapshot,
        "--reason",
        "late",
    ]);
    assert!(problems.is_empty(), "{problems:#?}");

    assert_eq!(view(&root, "snapshots")[0]["state"], "Complete");
    assert_eq!(view(&root, "repositories").len(), 1);
    for name in ["pull-requests", "workflow-runs", "sessions", "blockers"] {
        assert_eq!(view(&root, name), Vec::<Value>::new(), "{name}");
    }
}

/// A snapshot a crash left `Collecting` is listed as `Collecting` beside the next one, the next
/// `start-snapshot` still runs (here on a full disk, 0 bytes free, which is an observation like
/// any other), and `fail-snapshot` closes the stale one by hand.
#[test]
fn adv_a_snapshot_left_collecting_is_listed_and_closed_by_hand() {
    let root = case_dir("left-collecting");
    left_collecting(&root, STALE);
    let started = conductor_with(
        &root,
        Some(&offline(&root)),
        &["snapshot", "start-snapshot", "--disk-free-bytes", "0"],
    );
    let next = stdout(&started);
    let snapshots = view(&root, "snapshots");
    assert_eq!(snapshots.len(), 2, "{snapshots:?}; {}", describe(&started));
    assert_eq!(snapshots[0]["snapshot_id"], STALE);
    assert_eq!(snapshots[0]["state"], "Collecting");
    assert_eq!(snapshots[1]["snapshot_id"], next.as_str());
    assert_eq!(snapshots[1]["disk_free_bytes"], 0, "{snapshots:?}");
    assert_ne!(snapshots[1]["state"], "Collecting", "{snapshots:?}");

    let failed = conductor(
        &root,
        &[
            "snapshot",
            "fail-snapshot",
            "--snapshot-id",
            STALE,
            "--reason",
            "the run was killed",
        ],
    );
    assert!(failed.status.success(), "{}", describe(&failed));
    assert_eq!(view(&root, "snapshots")[0]["state"], "Failed");
}

// ---------------------------------------------------------------------------------------------
// The views
// ---------------------------------------------------------------------------------------------

/// One record per view that this build cannot read, each as a later specification would write
/// it: `(view, entity, key, body)`. Every body is a `stored` record at schema 1 whose one unknown
/// value is a state or a variant a later specification adds (GitHub's `internal` visibility, its
/// `timed_out` conclusion, a third harness).
fn unreadable(snapshot: &str) -> [(&'static str, &'static str, &'static str, Value); 6] {
    [
        (
            "snapshots",
            SNAPSHOT,
            "0199f000-0000-7000-8000-0000000000a0",
            json!({
                "state": "Abandoned", "snapshot_id": "0199f000-0000-7000-8000-0000000000a0",
                "started_at": "2026-10-07T00:00:00Z", "disk_free_bytes": 1,
            }),
        ),
        (
            "repositories",
            "conductor.observation.RepositoryObservation",
            "0199f000-0000-7000-8000-0000000000a1",
            json!({
                "state": "Recorded", "observation_id": "0199f000-0000-7000-8000-0000000000a1",
                "snapshot_id": snapshot, "repository": "gamma", "visibility": "Internal",
                "archived": false, "local_checkout": true, "main_head": "0a1b2c3",
                "main_committed_at": "2026-10-01T00:00:00Z", "main_commits_7d": 1,
                "real_commits_7d": 1, "behind_main": 0, "dirty_files": 0, "worktrees": 1,
                "open_issues": 0, "latest_release": null, "unreleased_commits": null,
                "planning_store_version": null, "in_catalog": true,
            }),
        ),
        (
            "pull-requests",
            "conductor.observation.PullRequestObservation",
            "0199f000-0000-7000-8000-0000000000a2",
            json!({
                "state": "Recorded", "observation_id": "0199f000-0000-7000-8000-0000000000a2",
                "snapshot_id": snapshot, "repository": "gamma", "number": 1, "title": "t",
                "draft": false, "mergeability": "Blocked", "failing_checks": 0,
                "opened_at": "2026-09-01T00:00:00Z", "updated_at": "2026-10-01T00:00:00Z",
            }),
        ),
        (
            "workflow-runs",
            "conductor.observation.WorkflowRunObservation",
            "0199f000-0000-7000-8000-0000000000a3",
            json!({
                "state": "Recorded", "observation_id": "0199f000-0000-7000-8000-0000000000a3",
                "snapshot_id": snapshot, "repository": "gamma", "workflow": "check",
                "conclusion": "TimedOut", "run_at": "2026-10-02T00:00:00Z",
            }),
        ),
        (
            "sessions",
            "conductor.observation.SessionObservation",
            "0199f000-0000-7000-8000-0000000000a4",
            json!({
                "state": "Recorded", "observation_id": "0199f000-0000-7000-8000-0000000000a4",
                "snapshot_id": snapshot, "harness": "Gemini", "session_ref": "s", "name": null,
                "cwd": "~/", "repository": null, "activity": "Idle",
            }),
        ),
        (
            "blockers",
            "conductor.observation.BlockerObservation",
            "0199f000-0000-7000-8000-0000000000a5",
            json!({
                "state": "Superseded", "observation_id": "0199f000-0000-7000-8000-0000000000a5",
                "snapshot_id": snapshot, "repository": "gamma", "reference": "blocker:x",
                "blocker_kind": "decision", "title": "x",
            }),
        ),
    ]
}

/// One record a view cannot read hides only itself (`story:store-hardening`: "only that goal is
/// hidden, and the view says one record was unreadable"): each of the six views still lists the
/// readable row, and names the unreadable record on standard error.
#[test]
fn adv_an_unreadable_record_hides_only_itself_in_each_view() {
    let root = case_dir("unreadable");
    let taken = take(&root, &[everything()]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = id(&taken.snapshot_id).to_owned();
    let records = unreadable(&snapshot);
    for (_, entity, key, body) in &records {
        append_raw(&root, entity, key, 1, body.clone());
    }

    let mut problems = Vec::new();
    for (name, _, key, _) in &records {
        let output = conductor(&root, &["snapshot", name, "--format", "json"]);
        let listed: Option<Vec<Value>> = serde_json::from_slice(&output.stdout).ok();
        let readable = listed
            .as_ref()
            .is_some_and(|rows| rows.len() == 1 && rows[0]["snapshot_id"] == snapshot.as_str());
        let named = String::from_utf8_lossy(&output.stderr).contains(key);
        if !readable || !named {
            problems.push(format!(
                "`snapshot {name}` (readable row listed: {readable}, unreadable {key} named: \
                 {named}): {}",
                describe(&output)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of 6 views do not list the readable row beside one unreadable record:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// A view over a state directory that is not there answers an error and creates nothing.
#[test]
fn adv_a_view_on_a_missing_state_dir_creates_nothing() {
    let root = case_dir("missing-state");
    let absent = root.join("absent");
    let flag = absent.to_str().expect("UTF-8 path");
    let mut problems = Vec::new();
    for name in VIEWS {
        let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(root.join("work"))
            .args(["snapshot", name, "--state-dir", flag])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        if output.status.code() != Some(1) || !output.stdout.is_empty() || absent.exists() {
            problems.push(format!("`snapshot {name}`: {}", describe(&output)));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
    assert_eq!(
        fs::read_dir(root.join("work")).expect("read work/").count(),
        0
    );
}

/// Every view prints the same bytes on every run over the same store, in the order recorded,
/// each row under the snapshot it was recorded in.
#[test]
fn adv_views_are_byte_identical_across_runs() {
    let root = case_dir("stable");
    let first = take(&root, &[everything()]).expect("the first snapshot ends");
    let second = take(&root, &[everything(), records("beta")]).expect("the second snapshot ends");
    for name in VIEWS {
        let once = conductor(&root, &["snapshot", name, "--format", "jsonl"]);
        let again = conductor(&root, &["snapshot", name, "--format", "jsonl"]);
        assert!(once.status.success(), "{name}: {}", describe(&once));
        assert_eq!(
            once.stdout, again.stdout,
            "`snapshot {name} --format jsonl`"
        );
        let snapshots: Vec<String> = String::from_utf8_lossy(&once.stdout)
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).expect("one JSON object per line")
                    ["snapshot_id"]
                    .as_str()
                    .expect("a snapshot id")
                    .to_owned()
            })
            .collect();
        let mut expected = vec![
            id(&first.snapshot_id).to_owned(),
            id(&second.snapshot_id).to_owned(),
        ];
        if name == "repositories" {
            expected.push(id(&second.snapshot_id).to_owned());
        }
        assert_eq!(
            snapshots, expected,
            "`snapshot {name}`: rows in recorded order"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The free disk
// ---------------------------------------------------------------------------------------------

/// `story:portable-free-space`: the free disk is read through `statvfs`, so a `df` that is
/// missing, fails, refuses GNU's options as macOS's does (`df: invalid option -- B`, exit 64), or
/// prints no number changes nothing: `start-snapshot` starts the snapshot with the bytes free,
/// runs no `df`, and stops only at the first collector, which finds no `gh`.
#[test]
fn adv_a_df_that_cannot_answer_does_not_stop_start_snapshot() {
    let fakes: [(&str, Option<&str>); 4] = [
        ("missing", None),
        ("failing", Some("#!/bin/sh\nexit 1\n")),
        (
            "bsd",
            Some("#!/bin/sh\necho 'df: invalid option -- B' >&2\nexit 64\n"),
        ),
        ("not-a-number", Some("#!/bin/sh\necho Avail\n")),
    ];
    let mut problems = Vec::new();
    for (case, df) in fakes {
        let root = case_dir(&format!("df-{case}"));
        // The programs the start-time check requires, as offline fakes (story:install-prerequisites).
        let bin = offline(&root);
        if let Some(script) = df {
            let fake = bin.join("df");
            fs::write(&fake, script).expect("write the fake df");
            fs::set_permissions(&fake, fs::Permissions::from_mode(0o755))
                .expect("make the fake df executable");
        }
        let output = conductor_with(&root, Some(&bin), &["snapshot", "start-snapshot"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let snapshots = if state(&root).join("tree").exists() {
            view(&root, "snapshots")
        } else {
            Vec::new()
        };
        let free = snapshots
            .first()
            .and_then(|snapshot| snapshot["disk_free_bytes"].as_i64());
        if output.status.code() != Some(1)
            || !stderr.contains(&format!(
                "snapshot {} failed: repositories: ",
                stdout(&output)
            ))
            || stderr.contains("`df")
            || stderr.contains("df:")
            || snapshots.len() != 1
            || !free.is_some_and(|free| free > 0)
        {
            problems.push(format!(
                "df {case}: snapshots {snapshots:?}; {}",
                describe(&output)
            ));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// `story:catalog-source`: `record-repository` takes `--in-catalog` or leaves it out. Left out,
/// the repository's catalog membership is unknown and the view reads it `null`; given, it reads
/// as given.
#[test]
fn record_repository_without_in_catalog_records_the_membership_absent() {
    let root = case_dir("record-without-catalog");
    left_collecting(&root, STALE);
    let (command, flags) = record_flags()
        .into_iter()
        .next()
        .expect("record-repository");
    assert_eq!(command, "record-repository");
    let at = flags
        .iter()
        .position(|flag| *flag == "--in-catalog")
        .expect("the flags give --in-catalog");
    let mut without = flags.clone();
    without.drain(at..at + 2);
    let mut given = without.clone();
    given[1] = "gamma";
    given.extend(["--in-catalog", "true"]);
    for flags in [&without, &given] {
        let args = [&["snapshot", command, "--snapshot-id", STALE], &flags[..]].concat();
        let output = conductor(&root, &args);
        assert!(output.status.success(), "{}", describe(&output));
    }
    let read: Vec<(Value, Value)> = view(&root, "repositories")
        .into_iter()
        .map(|row| (row["repository"].clone(), row["in_catalog"].clone()))
        .collect();
    assert_eq!(
        read,
        [
            (Value::from("beta"), Value::Null),
            (Value::from("gamma"), Value::from(true))
        ]
    );
}
