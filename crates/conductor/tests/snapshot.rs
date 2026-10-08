//! `story:snapshot-driver`: one sense run through the driver, with fake collectors in place of the
//! `story:collect-*` ones, and the observation commands and views of the built `conductor`.
//!
//! Each case keeps its store under `state/` in its own directory in this test target's temporary
//! directory. It takes snapshots through the library ([`snapshot::take`], over a set of fake
//! collectors) and reads them back through the binary, run from the case's empty `work/` with
//! `--state-dir` naming that `state/`; every run leaves `work/` empty.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::bail;
use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::SpecificationsQuery;
use conductor_model::observation::{
    CommitSha, Harness, Mergeability, RecordBlocker, RecordPullRequest, RecordRepository,
    RecordSession, RecordSpecification, RecordWorkflowRun, RepositoryName, RunConclusion,
    SessionState, SnapshotId, SnapshotState, SpecificationPresence, ValidationResult, Visibility,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};

/// A fresh directory for one case: its `state/` is the case's store, and its empty `work/` the
/// working directory of every run of the binary.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("snapshot")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("work")).expect("create the case's directories");
    dir
}

fn path(dir: &Path) -> &str {
    dir.to_str().expect("the case's path is UTF-8")
}

/// Takes one snapshot into the store under `root` through `collectors`.
fn take(root: &Path, collectors: &[Collector]) -> Taken {
    let store = store::open(&root.join("state")).expect("open the store");
    snapshot::take(store, collectors).expect("the driver ends the snapshot")
}

/// Runs the binary from `cwd` with `args` alone.
fn run(cwd: &Path, args: &[&str]) -> Output {
    run_with(cwd, None, args)
}

/// Runs the binary from `cwd` with `args` alone, and with `PATH` replaced when `path_env` is
/// given.
fn run_with(cwd: &Path, path_env: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
    command.current_dir(cwd).args(args).stdin(Stdio::null());
    if let Some(path) = path_env {
        command.env("PATH", path);
    }
    command.output().expect("the conductor binary runs")
}

/// A `PATH` for `start-snapshot` that holds only the `df` this process finds: the free disk is
/// measured, and the registered collectors reach no live source. With no `gh` the `repositories`
/// collector fails at its first command, so nothing asks GitHub and nothing runs `git fetch` in a
/// real checkout.
fn offline(root: &Path) -> PathBuf {
    let bin = root.join("offline");
    fs::create_dir_all(&bin).expect("create the offline PATH");
    let df = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("df"))
                .find(|df| df.is_file())
        })
        .expect("df is on PATH");
    std::os::unix::fs::symlink(df, bin.join("df")).expect("link df");
    bin
}

/// Runs the binary from `root`'s `work/` with `--state-dir` naming `root`'s `state/`, and asserts
/// it left `work/` empty.
fn conductor(root: &Path, args: &[&str]) -> Output {
    conductor_with(root, None, args)
}

/// [`conductor`], with `PATH` replaced when `path_env` is given.
fn conductor_with(root: &Path, path_env: Option<&Path>, args: &[&str]) -> Output {
    let work = root.join("work");
    let state = root.join("state");
    let output = run_with(
        &work,
        path_env,
        &[&["--state-dir", path(&state)], args].concat(),
    );
    assert_eq!(
        fs::read_dir(&work).expect("read work/").count(),
        0,
        "`{}` planted something in the working directory: {}",
        args.join(" "),
        describe(&output)
    );
    output
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The rows of `conductor snapshot <view> --format json` over `root`'s store.
fn rows(root: &Path, view: &str) -> Vec<Value> {
    let output = conductor(root, &["snapshot", view, "--format", "json"]);
    assert!(
        output.status.success(),
        "`snapshot {view}`: {}",
        describe(&output)
    );
    let text = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    match serde_json::from_str(&text) {
        Ok(Value::Array(rows)) => rows,
        other => panic!("`snapshot {view}` printed {text:?}: {other:?}"),
    }
}

/// `row` without its `observation_id`, which the store assigns; asserts it is there.
fn assigned(mut row: Value) -> Value {
    let id = row
        .as_object_mut()
        .and_then(|row| row.remove("observation_id"));
    assert!(
        id.as_ref()
            .and_then(Value::as_str)
            .is_some_and(|id| id.len() == 36),
        "the row carries an assigned observation id: {id:?}"
    );
    row
}

fn id(snapshot: &SnapshotId) -> &str {
    &snapshot.0.0
}

fn repository(snapshot: &SnapshotId, name: &str) -> RecordRepository {
    RecordRepository {
        snapshot_id: snapshot.clone(),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Private,
        archived: false,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: 7,
        real_commits_7d: 3,
        behind_main: 2,
        dirty_files: 4,
        worktrees: 5,
        open_issues: 6,
        oldest_open_issue_at: None,
        latest_release: Some("0.1.0".to_owned()),
        latest_release_at: None,
        unreleased_commits: None,
        planning_store_version: Some("aep.project/5".to_owned()),
        in_catalog: Some(true),
    }
}

/// The `Repositories` row `repository(snapshot, name)` reads as, without its observation id.
fn repository_row(snapshot: &SnapshotId, name: &str) -> Value {
    json!({
        "snapshot_id": id(snapshot),
        "repository": name,
        "visibility": "Private",
        "archived": false,
        "local_checkout": true,
        "main_head": "0a1b2c3",
        "main_committed_at": "2026-10-01T00:00:00Z",
        "main_commits_7d": 7,
        "real_commits_7d": 3,
        "behind_main": 2,
        "dirty_files": 4,
        "worktrees": 5,
        "open_issues": 6,
        "oldest_open_issue_at": null,
        "latest_release": "0.1.0",
        "latest_release_at": null,
        "unreleased_commits": null,
        "planning_store_version": "aep.project/5",
        "in_catalog": true,
    })
}

/// A fake collector that records the repository `name`.
fn records(name: &'static str) -> Collector {
    Collector::new("records", move |record: &mut Recorder<'_>| {
        let input = repository(record.snapshot(), name);
        record.repository(input)?;
        Ok(())
    })
}

/// A fake collector whose source does not answer.
fn fails() -> Collector {
    Collector::new("fails", |_: &mut Recorder<'_>| {
        bail!("the source did not answer")
    })
}

/// Acceptance 1: with one collector recording a repository and one failing, the snapshot ends
/// `Failed`, naming the failing collector, and the view lists the first collector's observation.
#[test]
fn a_source_that_does_not_answer_fails_the_snapshot_and_keeps_what_was_recorded() {
    let root = case_dir("failed");
    let taken = take(&root, &[records("alpha"), fails()]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    assert_eq!(
        taken.reason.as_deref(),
        Some("fails: the source did not answer"),
        "the reason names the first collector that did not answer"
    );

    let snapshots = rows(&root, "snapshots");
    assert_eq!(snapshots.len(), 1, "{snapshots:?}");
    assert_eq!(snapshots[0]["snapshot_id"], id(&taken.snapshot_id));
    assert_eq!(snapshots[0]["state"], "Failed");

    let repositories: Vec<Value> = rows(&root, "repositories")
        .into_iter()
        .map(assigned)
        .collect();
    assert_eq!(
        repositories,
        vec![repository_row(&taken.snapshot_id, "alpha")]
    );
}

/// Acceptance 2: a `RecordRepository` against the failed snapshot is refused with
/// `SnapshotNotCollecting`, and nothing is recorded. Completing or failing it again is refused the
/// same way, and an id no snapshot carries with `SnapshotNotFound`.
#[test]
fn a_failed_snapshot_takes_no_more_observations() {
    let root = case_dir("closed");
    let taken = take(&root, &[records("alpha"), fails()]);
    let snapshot = id(&taken.snapshot_id).to_owned();

    let record = |snapshot: &str| {
        conductor(
            &root,
            &[
                "snapshot",
                "record-repository",
                "--snapshot-id",
                snapshot,
                "--repository",
                "beta",
                "--visibility",
                "Public",
                "--archived",
                "false",
                "--local-checkout",
                "true",
                "--main-head",
                "0a1b2c3",
                "--main-committed-at",
                "2026-10-01T00:00:00Z",
                "--main-commits-7d",
                "1",
                "--real-commits-7d",
                "1",
                "--behind-main",
                "0",
                "--dirty-files",
                "0",
                "--worktrees",
                "1",
                "--open-issues",
                "0",
                "--in-catalog",
                "true",
            ],
        )
    };
    let refused = |output: &Output, error: &str, what: &str| {
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what}: {}",
            describe(output)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.lines().count(), 1, "{what}: {}", describe(output));
        assert!(stderr.contains(error), "{what}: {}", describe(output));
        assert!(output.stdout.is_empty(), "{what}: {}", describe(output));
    };

    refused(
        &record(&snapshot),
        "SnapshotNotCollecting",
        "record-repository into the failed snapshot",
    );
    refused(
        &conductor(
            &root,
            &["snapshot", "complete-snapshot", "--snapshot-id", &snapshot],
        ),
        "SnapshotNotCollecting",
        "complete-snapshot of the failed snapshot",
    );
    refused(
        &conductor(
            &root,
            &[
                "snapshot",
                "fail-snapshot",
                "--snapshot-id",
                &snapshot,
                "--reason",
                "again",
            ],
        ),
        "SnapshotNotCollecting",
        "fail-snapshot of the failed snapshot",
    );
    let unknown = "00000000-0000-7000-8000-000000000000";
    refused(
        &record(unknown),
        "SnapshotNotFound",
        "record-repository into no snapshot",
    );

    let repositories: Vec<Value> = rows(&root, "repositories")
        .into_iter()
        .map(assigned)
        .collect();
    assert_eq!(
        repositories,
        vec![repository_row(&taken.snapshot_id, "alpha")],
        "nothing was recorded by the refused commands"
    );
    assert_eq!(rows(&root, "snapshots")[0]["state"], "Failed");
}

/// Acceptance 3: with both collectors answering, the snapshot ends `Complete`, its free disk is
/// measured and greater than 0, and the view lists both observations in the order recorded.
#[test]
fn every_source_answering_completes_the_snapshot_with_the_free_disk_measured() {
    let root = case_dir("complete");
    let taken = take(&root, &[records("alpha"), records("beta")]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(taken.reason, None);

    let snapshots = rows(&root, "snapshots");
    assert_eq!(snapshots.len(), 1, "{snapshots:?}");
    assert_eq!(snapshots[0]["snapshot_id"], id(&taken.snapshot_id));
    assert_eq!(snapshots[0]["state"], "Complete");
    assert!(
        snapshots[0]["disk_free_bytes"]
            .as_i64()
            .is_some_and(|bytes| bytes > 0),
        "{snapshots:?}"
    );
    assert!(
        snapshots[0]["started_at"]
            .as_str()
            .is_some_and(|at| at.starts_with("20") && at.contains('T')),
        "{snapshots:?}"
    );

    let repositories: Vec<Value> = rows(&root, "repositories")
        .into_iter()
        .map(assigned)
        .collect();
    assert_eq!(
        repositories,
        vec![
            repository_row(&taken.snapshot_id, "alpha"),
            repository_row(&taken.snapshot_id, "beta"),
        ]
    );
}

/// A refused observation is the collector's error: the snapshot fails naming the refusal, and the
/// collector after it does not run.
#[test]
fn a_refused_observation_fails_the_snapshot_with_the_refusal() {
    let root = case_dir("refused");
    let zero = Collector::new("pulls", |record: &mut Recorder<'_>| {
        let input = RecordPullRequest {
            snapshot_id: record.snapshot().clone(),
            repository: RepositoryName("alpha".to_owned()),
            number: 0,
            title: "zero".to_owned(),
            draft: false,
            mergeability: Mergeability::Unknown,
            failing_checks: 0,
            opened_at: Timestamp("2026-09-01T00:00:00Z".to_owned()),
            updated_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        };
        record.pull_request(input)?;
        Ok(())
    });
    let taken = take(&root, &[zero, records("never")]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.unwrap_or_default();
    assert!(
        reason.starts_with("pulls: ") && reason.contains("InvalidPullRequestNumber"),
        "{reason:?}"
    );
    assert_eq!(rows(&root, "pull-requests"), Vec::<Value>::new());
    assert_eq!(rows(&root, "repositories"), Vec::<Value>::new());
}

/// Every observation kind a collector records reads back, field for field, through its view.
#[test]
fn every_observation_kind_reads_back_through_its_view() {
    let root = case_dir("kinds");
    let everything = Collector::new("everything", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let repository = RepositoryName("alpha".to_owned());
        record.pull_request(RecordPullRequest {
            snapshot_id: snapshot.clone(),
            repository: repository.clone(),
            number: 12,
            title: "Add the driver".to_owned(),
            draft: true,
            mergeability: Mergeability::Conflicting,
            failing_checks: 3,
            opened_at: Timestamp("2026-09-01T00:00:00Z".to_owned()),
            updated_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        })?;
        record.workflow_run(RecordWorkflowRun {
            snapshot_id: snapshot.clone(),
            repository: repository.clone(),
            workflow: "check".to_owned(),
            conclusion: RunConclusion::Cancelled,
            run_at: Timestamp("2026-10-02T00:00:00Z".to_owned()),
        })?;
        record.session(RecordSession {
            snapshot_id: snapshot.clone(),
            harness: Harness::Codex,
            session_ref: "session-1".to_owned(),
            name: None,
            cwd: "~/example-org/alpha".to_owned(),
            repository: Some(repository.clone()),
            role: Some("conductor".to_owned()),
            activity: SessionState::Background,
        })?;
        record.blocker(RecordBlocker {
            snapshot_id: snapshot.clone(),
            repository: repository.clone(),
            reference: "blocker:decide".to_owned(),
            blocker_kind: "decision".to_owned(),
            title: "Pick one".to_owned(),
        })?;
        record.specification(RecordSpecification {
            snapshot_id: snapshot,
            repository,
            presence: SpecificationPresence::OptedOut,
            path: None,
            format: Some("ess/22".to_owned()),
            required_ess: None,
            validation: ValidationResult::NotRun,
            validation_refusals: 0,
            scenarios: Some(185),
            synthesis_refusals: None,
            conformance_status: Some("validated".to_owned()),
        })?;
        Ok(())
    });
    let taken = take(&root, &[everything]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = id(&taken.snapshot_id);

    let read = |view: &str| -> Vec<Value> { rows(&root, view).into_iter().map(assigned).collect() };
    assert_eq!(
        read("pull-requests"),
        vec![json!({
            "snapshot_id": snapshot,
            "repository": "alpha",
            "number": 12,
            "title": "Add the driver",
            "draft": true,
            "mergeability": "Conflicting",
            "failing_checks": 3,
            "opened_at": "2026-09-01T00:00:00Z",
            "updated_at": "2026-10-01T00:00:00Z",
        })]
    );
    assert_eq!(
        read("workflow-runs"),
        vec![json!({
            "snapshot_id": snapshot,
            "repository": "alpha",
            "workflow": "check",
            "conclusion": "Cancelled",
            "run_at": "2026-10-02T00:00:00Z",
        })]
    );
    assert_eq!(
        read("sessions"),
        vec![json!({
            "snapshot_id": snapshot,
            "harness": "Codex",
            "session_ref": "session-1",
            "name": null,
            "cwd": "~/example-org/alpha",
            "repository": "alpha",
            "role": "conductor",
            "activity": "Background",
        })]
    );
    assert_eq!(
        read("blockers"),
        vec![json!({
            "snapshot_id": snapshot,
            "repository": "alpha",
            "reference": "blocker:decide",
            "blocker_kind": "decision",
            "title": "Pick one",
        })]
    );

    // The `specifications` view is `story:collect-specifications`'s; its rows are read here
    // through the generated query over the store.
    let store = store::open_existing(&root.join("state")).expect("open the store");
    let generated = Generated::new(store);
    let specifications = generated
        .specifications()
        .expect("the generated query answers");
    generated.ports.check().expect("the store read every row");
    assert_eq!(specifications.len(), 1, "{specifications:?}");
    let held = &specifications[0];
    assert_eq!(held.snapshot_id, taken.snapshot_id);
    assert_eq!(held.repository, RepositoryName("alpha".to_owned()));
    assert_eq!(held.presence, SpecificationPresence::OptedOut);
    assert_eq!(held.path, None);
    assert_eq!(held.format.as_deref(), Some("ess/22"));
    assert_eq!(held.required_ess, None);
    assert_eq!(held.validation, ValidationResult::NotRun);
    assert_eq!(held.validation_refusals, 0);
    assert_eq!(held.scenarios, Some(185));
    assert_eq!(held.synthesis_refusals, None);
    assert_eq!(held.conformance_status.as_deref(), Some("validated"));
}

/// `snapshot start-snapshot` is the driver over the registered collectors. It prints the
/// snapshot's id, which the view lists with the free disk measured; it exits 0 when the snapshot
/// completed and 1, with one stderr line naming the collector, when it failed.
#[test]
fn start_snapshot_runs_the_registered_collectors() {
    let root = case_dir("start");
    let output = conductor_with(
        &root,
        Some(&offline(&root)),
        &["snapshot", "start-snapshot"],
    );
    let printed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let snapshots = rows(&root, "snapshots");
    assert_eq!(snapshots.len(), 1, "{snapshots:?}; {}", describe(&output));
    assert_eq!(snapshots[0]["snapshot_id"], printed.as_str());
    assert!(
        snapshots[0]["disk_free_bytes"]
            .as_i64()
            .is_some_and(|bytes| bytes > 0),
        "{snapshots:?}"
    );
    match snapshots[0]["state"].as_str() {
        Some("Complete") => assert!(output.status.success(), "{}", describe(&output)),
        Some("Failed") => {
            assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(stderr.lines().count(), 1, "{}", describe(&output));
            assert!(stderr.contains(&printed), "{}", describe(&output));
        }
        other => panic!("the snapshot ended {other:?}; {}", describe(&output)),
    }
}

/// A negative free-disk count is refused with `InvalidCount`, and no snapshot is started.
#[test]
fn start_snapshot_refuses_a_negative_free_disk() {
    let root = case_dir("negative-disk");
    let output = conductor(
        &root,
        &["snapshot", "start-snapshot", "--disk-free-bytes", "-1"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("InvalidCount"),
        "{}",
        describe(&output)
    );
    assert_eq!(rows(&root, "snapshots"), Vec::<Value>::new());
}

/// `--state-dir` names the store: a command creates it under that directory and plants nothing
/// under the working directory, and the group's views and commands read it there, whether the
/// flag comes before or after the subcommand.
#[test]
fn the_state_dir_flag_names_the_store() {
    let root = case_dir("state-dir");
    let work = root.join("work");
    let elsewhere = root.join("elsewhere");
    let flag = path(&elsewhere);
    let planted = |what: &str, output: &Output| {
        assert_eq!(
            fs::read_dir(&work).expect("read work/").count(),
            0,
            "{what} planted something in the working directory: {}",
            describe(output)
        );
    };

    let started = run_with(
        &work,
        Some(&offline(&root)),
        &["snapshot", "start-snapshot", "--state-dir", flag],
    );
    let printed = String::from_utf8_lossy(&started.stdout).trim().to_owned();
    assert!(
        elsewhere.join("tree").is_dir(),
        "start-snapshot created no store under --state-dir: {}",
        describe(&started)
    );
    planted("start-snapshot", &started);

    let listed = run(
        &work,
        &[
            "--state-dir",
            flag,
            "snapshot",
            "snapshots",
            "--format",
            "json",
        ],
    );
    assert!(listed.status.success(), "{}", describe(&listed));
    let rows: Value = serde_json::from_slice(&listed.stdout).expect("the view prints JSON");
    assert_eq!(rows[0]["snapshot_id"], printed.as_str(), "{rows}");
    planted("snapshots", &listed);

    // The snapshot start-snapshot took ended complete or failed; either refuses another end.
    let completed = run(
        &work,
        &[
            "--state-dir",
            flag,
            "snapshot",
            "complete-snapshot",
            "--snapshot-id",
            &printed,
        ],
    );
    assert!(
        String::from_utf8_lossy(&completed.stderr).contains("SnapshotNotCollecting"),
        "complete-snapshot read the store under --state-dir: {}",
        describe(&completed)
    );
    planted("complete-snapshot", &completed);
}
