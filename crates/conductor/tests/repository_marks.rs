//! `story:repository-marks`: which repositories conductor processes. Without a mark, a
//! repository's activity is computed from the newest complete snapshot: Active when it had at least
//! one real commit in 7 days and is not archived. A `RepositoryMark`, set by the operator or by
//! conductor with a reason, overrides that computation, and the computation never writes one.
//!
//! Each case keeps its store under `state/` in its own directory in this test target's temporary
//! directory. It takes snapshots through the library ([`snapshot::take`], over fake collectors that
//! record fixture repositories), reads the effective activity through the library
//! ([`repository::activity`]), and runs the `repository` commands and view through the built
//! binary from the case's empty `work/`, with `--state-dir` naming that `state/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::bail;
use conductor_cli::collect::Collector;
use conductor_cli::repository;
use conductor_cli::snapshot::{self, Taken};
use conductor_cli::store;
use conductor_model::behaviour::{RepositoryObservationStorage, SnapshotStorage};
use conductor_model::direction::{
    Activity, DecidedBy, MarkedBy, MarkedRepository, RepositoryActivityRow,
};
use conductor_model::observation::{
    CommitSha, ObservationId, RecordRepository, RepositoryName, RepositoryObservationData,
    RepositoryObservationSnapshot, RepositoryObservationState, SnapshotData, SnapshotId,
    SnapshotSnapshot, SnapshotState, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

/// The reason conductor gives when it marks `x` inactive.
const SUPERSEDED: &str = "superseded by w (fixture)";

/// One case: its `state/` is the store, its `work/` the working directory of every run.
struct Case {
    root: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("repository_marks")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        fs::create_dir_all(root.join("work")).expect("create the case's directories");
        Self { root }
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    /// Takes one snapshot into the case's store through `collectors`.
    fn take(&self, collectors: &[Collector]) -> Taken {
        let store = store::open(&self.state()).expect("open the store");
        snapshot::take(store, collectors).expect("the driver ends the snapshot")
    }

    /// A complete snapshot in which each repository of `observed` was recorded.
    fn snapshot(&self, observed: &[(&str, i64, bool)]) -> Taken {
        let taken = self.take(&[recording(observed)]);
        assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
        taken
    }

    /// Every repository's effective activity, read through the library over the case's store.
    fn activity(&self) -> Vec<RepositoryActivityRow> {
        let store = store::open_existing(&self.state()).expect("open the store");
        let effective = repository::activity(&store);
        store.check().expect("every record of the store reads");
        effective
    }

    /// Runs the binary from `work/` with `--state-dir` naming `state/`, and asserts it left
    /// `work/` empty.
    fn run(&self, args: &[&str]) -> Output {
        let work = self.root.join("work");
        let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(&work)
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert_eq!(
            fs::read_dir(&work).expect("read work/").count(),
            0,
            "`{}` planted something in the working directory: {}",
            args.join(" "),
            describe(&output)
        );
        output
    }

    /// `repository mark-repository`.
    fn mark(&self, repository: &str, activity: &str, by: &str, reason: &str) -> Output {
        self.run(&[
            "repository",
            "mark-repository",
            "--repository",
            repository,
            "--activity",
            activity,
            "--marked-by",
            by,
            "--reason",
            reason,
        ])
    }

    /// `repository <command>` for `activate-repository` or `deactivate-repository`.
    fn steer(&self, command: &str, repository: &str, by: &str, reason: &str) -> Output {
        self.run(&[
            "repository",
            command,
            "--repository",
            repository,
            "--marked-by",
            by,
            "--reason",
            reason,
        ])
    }

    /// The rows of `repository repository-marks --format json`.
    fn marks(&self) -> Value {
        let output = self.run(&["repository", "repository-marks", "--format", "json"]);
        let text = succeeded(&output, "`repository repository-marks --format json`");
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("the view printed {text:?}: {error}"))
    }
}

/// A fake collector that records each `(repository, real_commits_7d, archived)` of `observed`.
fn recording(observed: &[(&str, i64, bool)]) -> Collector {
    let inputs: Vec<RecordRepository> = observed
        .iter()
        .map(|&(name, real, archived)| observation(name, real, archived))
        .collect();
    Collector::new("repositories", move |recorder| {
        for input in &inputs {
            recorder.repository(input.clone())?;
        }
        Ok(())
    })
}

/// A collector that does not answer.
fn silent() -> Collector {
    Collector::new("github", |_| bail!("the fixture source does not answer"))
}

/// One repository's state; the recorder files it under the snapshot it fills.
fn observation(name: &str, real_commits_7d: i64, archived: bool) -> RecordRepository {
    RecordRepository {
        snapshot_id: SnapshotId(Uuid("00000000-0000-0000-0000-000000000000".to_owned())),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Private,
        archived,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: real_commits_7d + 2,
        real_commits_7d,
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

/// Activity computed from the newest complete snapshot: no mark decided it.
fn computed(repository: &str, activity: Activity) -> RepositoryActivityRow {
    RepositoryActivityRow {
        repository: MarkedRepository(repository.to_owned()),
        activity,
        decided_by: DecidedBy::Snapshot,
        marked_by: None,
        reason: None,
    }
}

/// Activity a mark decided.
fn marked(
    repository: &str,
    activity: Activity,
    by: MarkedBy,
    reason: &str,
) -> RepositoryActivityRow {
    RepositoryActivityRow {
        repository: MarkedRepository(repository.to_owned()),
        activity,
        decided_by: DecidedBy::Mark,
        marked_by: Some(by),
        reason: Some(reason.to_owned()),
    }
}

/// A row of `repository activity --format json`; `marked_by` and `reason` are null for a
/// computed activity.
fn activity_row(
    repository: &str,
    activity: &str,
    decided_by: &str,
    marked_by: Option<&str>,
    reason: Option<&str>,
) -> Value {
    json!({
        "repository": repository,
        "activity": activity,
        "decided_by": decided_by,
        "marked_by": marked_by,
        "reason": reason,
    })
}

/// Every file under `dir` with its bytes, so a case can show a command wrote nothing.
fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read a directory of the store") {
            let path = entry.expect("read a store entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("read a file of the store");
                found.push((path, bytes));
            }
        }
    }
    found.sort();
    found
}

/// A `RepositoryMarks` row.
fn row(repository: &str, state: &str, by: &str, reason: &str) -> Value {
    json!({
        "repository": repository,
        "state": state,
        "marked_by": by,
        "reason": reason,
    })
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Standard output of a command that succeeded.
fn succeeded(output: &Output, what: &str) -> String {
    assert!(output.status.success(), "{what}: {}", describe(output));
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

/// A command that succeeded and printed nothing.
fn quiet(output: &Output, what: &str) {
    assert_eq!(succeeded(output, what), "", "{what} prints nothing");
}

/// A command refused with the declared `error`: exit 1, nothing on standard output, and one line
/// on standard error that names the error.
fn refused(output: &Output, what: &str, error: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && output.stdout.is_empty()
            && stderr.lines().count() == 1
            && stderr.contains(error),
        "{what} is refused with {error}: {}",
        describe(output)
    );
}

fn path(dir: &Path) -> &str {
    dir.to_str().expect("the case's path is UTF-8")
}

#[test]
fn a_mark_overrides_activity_computed_from_the_newest_complete_snapshot() {
    let case = Case::new("override");

    // 1. Snapshot 1: `x` has real commits, `y` has none, `z` has some but is archived.
    case.snapshot(&[("x", 3, false), ("y", 0, false), ("z", 4, true)]);
    assert_eq!(
        case.activity(),
        [
            computed("x", Activity::Active),
            computed("y", Activity::Inactive),
            computed("z", Activity::Inactive),
        ],
        "snapshot 1: every activity is computed"
    );
    assert_eq!(case.marks(), json!([]), "computing activity writes no mark");

    // 2. Conductor marks `x` inactive, with a reason.
    quiet(
        &case.mark("x", "Inactive", "Conductor", SUPERSEDED),
        "conductor marks x inactive",
    );
    assert_eq!(
        case.marks(),
        json!([row("x", "Inactive", "Conductor", SUPERSEDED)])
    );

    // 3. Snapshot 2: `x` still has real commits, and now `y` has too. `x` stays inactive by the
    //    mark; `y` follows the newest snapshot.
    case.snapshot(&[("x", 5, false), ("y", 2, false), ("z", 4, true)]);
    assert_eq!(
        case.activity(),
        [
            marked("x", Activity::Inactive, MarkedBy::Conductor, SUPERSEDED),
            computed("y", Activity::Active),
            computed("z", Activity::Inactive),
        ],
        "snapshot 2: the mark wins over x's commits"
    );
    assert_eq!(
        case.marks(),
        json!([row("x", "Inactive", "Conductor", SUPERSEDED)]),
        "the view shows the mark, who set it and why; the computation wrote nothing"
    );

    // A failed snapshot is not the newest complete one: `y`'s activity still reads from
    // snapshot 2.
    let failed = case.take(&[recording(&[("y", 0, false)]), silent()]);
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");
    assert_eq!(
        case.activity()[1],
        computed("y", Activity::Active),
        "a failed snapshot is never read as current"
    );

    // 4. Activating an already active repository is refused, and so is deactivating an inactive
    //    one; nothing moves.
    quiet(
        &case.mark("w", "Active", "Operator", "the operator names it"),
        "the operator marks w active",
    );
    refused(
        &case.steer("activate-repository", "w", "Operator", "again"),
        "activating active w",
        "RepositoryMarkConflict",
    );
    refused(
        &case.steer("deactivate-repository", "x", "Conductor", "again"),
        "deactivating inactive x",
        "RepositoryMarkConflict",
    );
    let unmoved = json!([
        row("x", "Inactive", "Conductor", SUPERSEDED),
        row("w", "Active", "Operator", "the operator names it"),
    ]);
    assert_eq!(case.marks(), unmoved, "a refused command moves nothing");

    // 5. Activating or deactivating a repository with no mark is refused.
    refused(
        &case.steer("activate-repository", "y", "Operator", "process y"),
        "activating unmarked y",
        "RepositoryMarkNotFound",
    );
    refused(
        &case.steer("deactivate-repository", "y", "Operator", "leave y"),
        "deactivating unmarked y",
        "RepositoryMarkNotFound",
    );
    assert_eq!(case.marks(), unmoved, "a refused command creates no mark");

    // The operator overrides conductor's mark: `x` is active again, by the operator's word.
    quiet(
        &case.steer(
            "activate-repository",
            "x",
            "Operator",
            "x is not superseded",
        ),
        "the operator activates x",
    );
    assert_eq!(
        case.marks(),
        json!([
            row("x", "Active", "Operator", "x is not superseded"),
            row("w", "Active", "Operator", "the operator names it"),
        ])
    );
    assert_eq!(
        case.activity(),
        [
            marked(
                "w",
                Activity::Active,
                MarkedBy::Operator,
                "the operator names it"
            ),
            marked(
                "x",
                Activity::Active,
                MarkedBy::Operator,
                "x is not superseded"
            ),
            computed("y", Activity::Active),
            computed("z", Activity::Inactive),
        ],
        "a marked repository no snapshot observed is still answered, by its mark"
    );
    quiet(
        &case.steer("deactivate-repository", "w", "Conductor", "w is superseded"),
        "conductor deactivates w",
    );
    assert_eq!(
        case.activity()[0],
        marked(
            "w",
            Activity::Inactive,
            MarkedBy::Conductor,
            "w is superseded"
        )
    );
}

#[test]
fn without_a_complete_snapshot_only_marks_are_answered() {
    let case = Case::new("no_complete_snapshot");
    let failed = case.take(&[recording(&[("x", 3, false)]), silent()]);
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");
    assert_eq!(case.activity(), [], "a failed snapshot computes nothing");

    quiet(
        &case.mark("y", "Inactive", "Operator", "spike only"),
        "the operator marks y inactive",
    );
    assert_eq!(
        case.activity(),
        [marked(
            "y",
            Activity::Inactive,
            MarkedBy::Operator,
            "spike only"
        )]
    );
}

#[test]
fn the_newest_complete_snapshot_is_the_latest_started() {
    let case = Case::new("newest");
    case.snapshot(&[("x", 0, false)]);
    case.snapshot(&[("x", 1, false)]);
    assert_eq!(
        case.activity(),
        [computed("x", Activity::Active)],
        "the second snapshot is the newest"
    );
}

/// Stores, through the store's own ports, a snapshot in `state` that started at `started_at` and
/// observed `x` with `real_commits_7d`, so a case can store snapshots out of their start order.
fn stored(case: &Case, id: &str, state: SnapshotState, started_at: &str, real_commits_7d: i64) {
    let mut store = store::open(&case.state()).expect("open the store");
    let snapshot_id = SnapshotId(Uuid(id.to_owned()));
    SnapshotStorage::put(
        &mut store,
        SnapshotSnapshot {
            state,
            data: SnapshotData {
                snapshot_id: snapshot_id.clone(),
                started_at: Timestamp(started_at.to_owned()),
                disk_free_bytes: 1,
                failure_reason: None,
            },
        },
    );
    let input = observation("x", real_commits_7d, false);
    RepositoryObservationStorage::put(
        &mut store,
        RepositoryObservationSnapshot {
            state: RepositoryObservationState::Recorded,
            data: RepositoryObservationData {
                observation_id: ObservationId(Uuid(format!("{id}-x"))),
                snapshot_id,
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
            },
        },
    );
    store.check().expect("the store keeps the fixture snapshot");
}

#[test]
fn the_newest_complete_snapshot_is_decided_by_its_start_not_by_store_order() {
    let case = Case::new("start_order");
    let x = |case: &Case| case.activity()[0].activity;

    stored(
        &case,
        "a",
        SnapshotState::Complete,
        "2026-10-07T12:00:00Z",
        1,
    );
    stored(
        &case,
        "b",
        SnapshotState::Complete,
        "2026-10-07T13:00:00+02:00",
        0,
    );
    assert_eq!(
        x(&case),
        Activity::Active,
        "b was stored later but started earlier, at 11:00Z: a is the newest"
    );

    stored(
        &case,
        "c",
        SnapshotState::Complete,
        "2026-10-07T14:00:00+02:00",
        0,
    );
    assert_eq!(
        x(&case),
        Activity::Inactive,
        "c started at the same instant as a and was stored later: c is the newest"
    );

    stored(&case, "d", SnapshotState::Complete, "not a time", 1);
    stored(
        &case,
        "e",
        SnapshotState::Collecting,
        "2026-10-07T13:00:00Z",
        1,
    );
    stored(&case, "f", SnapshotState::Failed, "2026-10-07T13:00:00Z", 1);
    assert_eq!(
        x(&case),
        Activity::Inactive,
        "a start that does not read, a collecting and a failed snapshot are never the newest"
    );
}

#[test]
fn a_command_needs_every_flag_and_a_view_creates_no_store() {
    let case = Case::new("flags");
    let output = case.run(&[
        "repository",
        "mark-repository",
        "--repository",
        "x",
        "--activity",
        "Active",
        "--marked-by",
        "Operator",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1) && stderr.contains("--reason"),
        "a mark without a reason is refused naming --reason: {}",
        describe(&output)
    );

    let output = case.run(&["repository", "repository-marks"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        !case.state().exists(),
        "the view created {}",
        path(&case.state())
    );
}

/// `mark-repository` marks a repository the first time only (wave 07, coordinator decision 2): a
/// second mark, with the same activity or the other one, by either actor, is refused with
/// `RepositoryAlreadyMarked`, naming the repository and the two commands that change a mark, and
/// not one byte of the store is written.
#[test]
fn marking_a_marked_repository_is_refused_and_writes_nothing() {
    let case = Case::new("mark_twice");
    quiet(
        &case.mark("x", "Inactive", "Conductor", SUPERSEDED),
        "conductor marks x inactive",
    );
    let first = json!([row("x", "Inactive", "Conductor", SUPERSEDED)]);
    assert_eq!(case.marks(), first);

    for (activity, by) in [("Active", "Operator"), ("Inactive", "Conductor")] {
        let store = files(&case.state());
        let output = case.mark("x", activity, by, "a second mark");
        refused(
            &output,
            &format!("marking x {activity} by {by} again"),
            "RepositoryAlreadyMarked",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("\"x\"")
                && stderr.contains("activate-repository")
                && stderr.contains("deactivate-repository"),
            "the refusal names the repository and the commands that change a mark: {}",
            describe(&output)
        );
        assert!(
            files(&case.state()) == store,
            "a refused mark writes nothing to the store"
        );
    }
    assert_eq!(case.marks(), first, "the first mark stands");
    assert_eq!(
        case.activity(),
        [marked(
            "x",
            Activity::Inactive,
            MarkedBy::Conductor,
            SUPERSEDED
        )]
    );
}

/// `repository activity` (wave 07, coordinator decision 1) lists every repository's effective
/// activity, the computed ones beside the marked ones, by repository name, with what decided it:
/// the rows [`repository::activity`] answers, in each view format.
#[test]
fn repository_activity_lists_every_repository_and_what_decided_it() {
    let case = Case::new("activity_view");
    case.snapshot(&[("x", 3, false), ("y", 0, false), ("z", 4, true)]);
    quiet(
        &case.mark("x", "Inactive", "Conductor", SUPERSEDED),
        "conductor marks x inactive",
    );
    quiet(
        &case.mark("w", "Active", "Operator", "the operator names it"),
        "the operator marks w active",
    );
    assert_eq!(
        case.activity(),
        [
            marked(
                "w",
                Activity::Active,
                MarkedBy::Operator,
                "the operator names it"
            ),
            marked("x", Activity::Inactive, MarkedBy::Conductor, SUPERSEDED),
            computed("y", Activity::Inactive),
            computed("z", Activity::Inactive),
        ]
    );

    let expected = [
        activity_row(
            "w",
            "Active",
            "Mark",
            Some("Operator"),
            Some("the operator names it"),
        ),
        activity_row("x", "Inactive", "Mark", Some("Conductor"), Some(SUPERSEDED)),
        activity_row("y", "Inactive", "Snapshot", None, None),
        activity_row("z", "Inactive", "Snapshot", None, None),
    ];

    let json_out = succeeded(
        &case.run(&["repository", "activity", "--format", "json"]),
        "`repository activity --format json`",
    );
    let printed: Value = serde_json::from_str(&json_out)
        .unwrap_or_else(|error| panic!("printed {json_out:?}: {error}"));
    assert_eq!(printed, Value::from(expected.to_vec()));

    let jsonl = succeeded(
        &case.run(&["repository", "activity", "--format", "jsonl"]),
        "`repository activity --format jsonl`",
    );
    assert_eq!(
        jsonl.lines().next(),
        Some(
            r#"{"repository":"w","activity":"Active","decided_by":"Mark","marked_by":"Operator","reason":"the operator names it"}"#
        ),
        "one object per row, its keys in the row type's order"
    );
    let lines: Vec<Value> = jsonl
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
        .collect();
    assert_eq!(lines, expected);

    let text = succeeded(
        &case.run(&["repository", "activity"]),
        "`repository activity`",
    );
    let table: Vec<Vec<&str>> = text
        .lines()
        .map(|line| {
            line.split("  ")
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect()
        })
        .collect();
    assert_eq!(
        table,
        [
            vec![
                "repository",
                "activity",
                "decided_by",
                "marked_by",
                "reason"
            ],
            vec!["w", "Active", "Mark", "Operator", "the operator names it"],
            vec!["x", "Inactive", "Mark", "Conductor", SUPERSEDED],
            vec!["y", "Inactive", "Snapshot", "null", "null"],
            vec!["z", "Inactive", "Snapshot", "null", "null"],
        ],
        "text is the default: {text:?}"
    );

    let markdown = succeeded(
        &case.run(&["repository", "activity", "--format", "markdown"]),
        "`repository activity --format markdown`",
    );
    assert_eq!(
        markdown.lines().collect::<Vec<_>>(),
        [
            "| repository | activity | decided_by | marked_by | reason |",
            "|---|---|---|---|---|",
            "| w | Active | Mark | Operator | the operator names it |",
            &format!("| x | Inactive | Mark | Conductor | {SUPERSEDED} |"),
            "| y | Inactive | Snapshot | null | null |",
            "| z | Inactive | Snapshot | null | null |",
        ]
    );
}

/// `repository activity` refuses what it cannot answer with exit 1, one line on standard error and
/// nothing on standard output: no store (and it creates none), and a format no view offers.
#[test]
fn repository_activity_refuses_a_missing_store_and_an_unknown_format() {
    let case = Case::new("activity_refusals");
    let output = case.run(&["repository", "activity", "--format", "json"]);
    assert!(
        output.status.code() == Some(1) && output.stdout.is_empty(),
        "no store: {}",
        describe(&output)
    );
    assert!(
        !case.state().exists(),
        "the view created {}",
        path(&case.state())
    );

    case.snapshot(&[("x", 1, false)]);
    let output = case.run(&["repository", "activity", "--format", "yaml"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && output.stdout.is_empty()
            && stderr.lines().count() == 1
            && stderr.contains("--format \"yaml\""),
        "an unknown format is named: {}",
        describe(&output)
    );
}
