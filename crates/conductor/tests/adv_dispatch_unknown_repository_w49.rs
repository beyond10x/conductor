//! Adversary cases for `story:dispatch-refuses-unknown-repository`, wave 49 U13, pass 1: which
//! snapshot decides the names `dispatch send-dispatch` knows, how exactly a name is held against
//! them, and what a refusal leaves behind in the store and the records.
//!
//! Each case runs the built binary against its own directory under this test target's temporary
//! directory in `target/`; the snapshots are taken through the library at given start times.

mod active;
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot;
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotId, SnapshotState, StartSnapshot,
    Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};

/// The send instant every dispatch of a case carries.
const SENT_AT: &str = "2026-10-07T09:00:00Z";

/// What the refusal starts with, before the name.
const REFUSAL: &str = "conductor: dispatch send-dispatch: --repository ";

/// One case's directory, emptied: `state/` holds the store, `home/` is the binary's home.
fn case_dir(case: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_dispatch_unknown_repository_w49")
        .join(case);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case's directory");
    }
    fs::create_dir_all(&root).expect("create the case's directory");
    root
}

/// Takes one snapshot started at `started_at` that observes `repositories`, into the store under
/// `state`; with `fails`, its collector then answers an error, so the snapshot ends `Failed`.
fn snapshot_of(state: &Path, started_at: &str, repositories: &[&str], fails: bool) {
    active::isolate();
    let inputs: Vec<RecordRepository> = repositories
        .iter()
        .map(|repository| RecordRepository {
            actor: conductor_cli::actor::daemon(),
            snapshot_id: SnapshotId(Uuid("00000000-0000-0000-0000-000000000000".to_owned())),
            repository: RepositoryName((*repository).to_owned()),
            visibility: Visibility::Private,
            archived: false,
            local_checkout: true,
            main_head: CommitSha("0a1b2c3".to_owned()),
            main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
            main_commits_7d: 3,
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
            gates_hooks_version: None,
            gates_version: None,
            wave_width: None,
            wave_ready: None,
            wave_reason: None,
        })
        .collect();
    let collector = Collector::new("repositories", move |recorder| {
        for input in &inputs {
            recorder.repository(input.clone())?;
        }
        if fails {
            anyhow::bail!("the forge did not answer");
        }
        Ok(())
    });
    let store = conductor_cli::store::open(state).expect("open the store");
    let start = StartSnapshot {
        actor: conductor_cli::actor::daemon(),
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let taken =
        snapshot::take_from(store, start, &[collector]).expect("the driver ends the snapshot");
    let expected = if fails {
        SnapshotState::Failed
    } else {
        SnapshotState::Complete
    };
    assert_eq!(taken.state, expected, "{taken:?}");
}

/// Runs `conductor --state-dir <root>/state <args>` with no config file.
fn conductor(root: &Path, args: &[&str]) -> Output {
    common::conductor(root.join("home"))
        .current_dir(root)
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

/// The arguments that send a dispatch to `repository` for the goal `G9`.
fn send(repository: &str) -> [&str; 10] {
    [
        "dispatch",
        "send-dispatch",
        "--repository",
        repository,
        "--goal-id",
        "G9",
        "--brief",
        "b",
        "--sent-at",
        SENT_AT,
    ]
}

/// The arguments that propose the goal `G9`.
const GOAL: [&str; 8] = [
    "goal",
    "propose-goal",
    "--goal-id",
    "G9",
    "--title",
    "Ship the pilot",
    "--exit-evidence",
    "G9 is listed as met in NORTHSTAR.md",
];

/// Standard output of a run that must exit 0.
fn ok(output: &Output, what: &str) -> String {
    assert!(
        output.status.success(),
        "{what} exited {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

/// `output` is the refusal of `repository` that names exactly `known`: exit 1, nothing on
/// standard output, one line on standard error.
fn refused(output: &Output, repository: &str, known: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = format!(
        "{REFUSAL}{repository}: neither the newest complete snapshot nor the workspace names it; \
         they name {known}; nothing was sent\n"
    );
    assert!(
        output.status.code() == Some(1) && output.stdout.is_empty() && stderr == expected,
        "--repository {repository:?} is refused naming {known:?}: exit {:?}, stdout {:?}, stderr \
         {stderr:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// Every file under `dir` with its bytes, by path.
fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read a directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.insert(path.clone(), fs::read(&path).expect("read a file"));
            }
        }
    }
    found
}

/// The acceptance says "the newest complete snapshot". A repository only an older complete
/// snapshot observed, and one only a newer snapshot that failed observed, are refused, and the
/// refusal names what the newest complete one holds and nothing else.
#[test]
fn only_the_newest_complete_snapshot_decides_which_repositories_are_known() {
    let root = case_dir("newest-complete");
    let state = root.join("state");
    // Stored out of start order on purpose: the newest is picked by start, not by position.
    snapshot_of(&state, "2026-10-05T00:00:00Z", &["alpha", "beta"], false);
    snapshot_of(&state, "2026-10-01T00:00:00Z", &["alpha", "retired"], false);
    snapshot_of(&state, "2026-10-06T00:00:00Z", &["alpha", "fresh"], true);
    ok(&conductor(&root, &GOAL), "propose-goal");

    refused(
        &conductor(&root, &send("retired")),
        "retired",
        "alpha, beta",
    );
    refused(&conductor(&root, &send("fresh")), "fresh", "alpha, beta");
    assert_eq!(
        ok(
            &conductor(&root, &["dispatch", "dispatches", "--format", "jsonl"]),
            "dispatches"
        ),
        "",
        "a refused dispatch was kept"
    );
    assert_eq!(
        ok(&conductor(&root, &send("beta")), "send to beta"),
        "DSP-20261007-01\n"
    );
}

/// A name is held against the known ones exactly: another case, a trailing slash, an owner in
/// front, a path, surrounding space, a prefix of a known name and a known name with more behind
/// it are each refused, with no config file (the snapshot alone decides). None of them takes an
/// id: the first dispatch sent afterwards is number 01.
#[test]
fn a_name_that_only_resembles_a_known_one_is_refused() {
    let root = case_dir("name-forms");
    let state = root.join("state");
    snapshot_of(
        &state,
        "2026-10-05T00:00:00Z",
        &["alpha", "alpha-tools"],
        false,
    );
    ok(&conductor(&root, &GOAL), "propose-goal");

    for name in [
        "Alpha",
        "ALPHA",
        "alpha/",
        "example-org/alpha",
        "./alpha",
        "/alpha",
        " alpha",
        "alpha ",
        "alph",
        "alpha-",
        "alpha-tool",
        "alpha-tools-two",
        "tools",
    ] {
        refused(&conductor(&root, &send(name)), name, "alpha, alpha-tools");
    }
    assert_eq!(
        ok(
            &conductor(&root, &["dispatch", "dispatches", "--format", "jsonl"]),
            "dispatches"
        ),
        "",
        "a refused dispatch was kept"
    );
    assert_eq!(
        ok(
            &conductor(&root, &send("alpha-tools")),
            "send to alpha-tools"
        ),
        "DSP-20261007-01\n"
    );
    assert_eq!(
        ok(&conductor(&root, &send("alpha")), "send to alpha"),
        "DSP-20261007-02\n"
    );
}

/// With a config file: a refusal changes no file of the records (no line in the dispatch log),
/// and a workspace file that does not read ends the command with exit 1 naming the file, with
/// nothing sent and the records unchanged, also for a name the snapshot holds.
#[test]
fn a_refusal_leaves_the_records_as_they_were() {
    let root = case_dir("records-untouched");
    let state = root.join("state");
    let records = root.join("records");
    let workspace = records.join(".engineering/workspace.yaml");
    fs::create_dir_all(records.join(".engineering")).expect("create the records directory");
    fs::write(
        &workspace,
        "version: aep.workspace/1\nmembers:\n  - name: alpha\n    source: ../example-org/alpha\n",
    )
    .expect("write the workspace file");
    let config = root.join("conductor.yaml");
    fs::write(
        &config,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   session_prefix: ex\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {checkouts}, trees: {trees}}}\n\
             \x20   records: {records}\n\
             \x20   state: {state}\n",
            checkouts = root.join("home/example-org").display(),
            trees = root.join("trees").display(),
            records = records.display(),
            state = state.display(),
        ),
    )
    .expect("write the config file");
    snapshot_of(&state, "2026-10-05T00:00:00Z", &["beta"], false);
    let run = |args: &[&str]| {
        common::conductor(root.join("home"))
            .current_dir(&root)
            .env("CONDUCTOR_CONFIG", &config)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    };
    ok(&run(&GOAL), "propose-goal");
    let before = files(&records);

    refused(&run(&send("gamma")), "gamma", "alpha, beta");
    assert_eq!(files(&records), before, "a refusal wrote to the records");

    fs::write(
        &workspace,
        "version: aep.workspace/1\nmembers:\n  - name: alpha\n",
    )
    .expect("break the workspace file");
    let before = files(&records);
    let output = run(&send("beta"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && output.stdout.is_empty()
            && stderr.lines().count() == 1
            && stderr.starts_with("conductor: dispatch send-dispatch: read the workspace: ")
            && stderr.contains(&workspace.display().to_string()),
        "a workspace file that does not read ends the command naming it: exit {:?}, stdout {:?}, \
         stderr {stderr:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
    );
    assert_eq!(
        files(&records),
        before,
        "the failed command wrote to the records"
    );
    assert_eq!(
        ok(
            &run(&["dispatch", "dispatches", "--format", "jsonl"]),
            "dispatches"
        ),
        "",
        "a dispatch was kept though the command failed"
    );
}
