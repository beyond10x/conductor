//! `story:status-board`: `conductor snapshot publish-board` writes the board, `STATUS.md`, from
//! one complete snapshot and records `PublishBoard`; `snapshot boards` lists what it recorded.
//!
//! Each case keeps its store under `state/` in its own directory in this test target's temporary
//! directory. It takes snapshots through the library ([`snapshot::take_from`], at a start time the
//! case gives) with one fake collector that records a fixture of `tests/fixtures/status/`, and
//! publishes them through the built binary, run from the case's empty `work/` with `--state-dir`
//! naming that `state/` and `HOME` naming the case's `home/`.
//!
//! The fixtures: `base.json` holds `alpha` (a checkout, one open pull request with no failing
//! check, a green `main`), `beta` (no checkout) and the archived `gamma`; `red.json` is `base.json`
//! with that pull request's check turned red, and nothing else changed. `reviews.json` holds four
//! open pull requests of `alpha` (a draft, one with a failing check, and two waiting for review,
//! opened on different days) and one of `beta` waiting; `reviews-three.json` three of `alpha`
//! waiting (`story:review-limit`). `checks.json` is `red.json` with four pull requests of `alpha`:
//! one with three failing checks named, one with one, one failing with no name recorded (an
//! observation from before the names), and a green one (`story:pr-check-names`).

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use anyhow::bail;
use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::observation::{
    CommitSha, Harness, Mergeability, RecordBlocker, RecordPullRequest, RecordRepository,
    RecordSession, RecordSpecification, RecordWorkflowRun, RepositoryName, RunConclusion,
    SessionState, SnapshotState, SpecificationPresence, StartSnapshot, ValidationResult,
    Visibility,
};
use conductor_model::primitives::Timestamp;
use serde_json::Value;

/// The board's section headings, in the order the story gives them.
const SECTIONS: [&str; 8] = [
    "# Status",
    "## Waiting for review",
    "## Deviations",
    "## Overview",
    "## Specifications",
    "## Open blockers",
    "## Sessions",
    "## Delta",
];

/// When the first snapshot of a case starts.
const EARLIER: &str = "2026-10-12T08:00:00Z";

/// When the second snapshot of a case starts.
const LATER: &str = "2026-10-12T10:00:00Z";

/// A fresh directory for one case: its `state/` is the case's store, its `home/` the binary's
/// home, and its empty `work/` the working directory of every run of the binary.
fn case_dir(case: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("status")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    for sub in ["work", "home"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    dir
}

/// The fixture `name` of `tests/fixtures/status/`.
fn fixture(name: &str) -> Value {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/status")
        .join(format!("{name}.json"));
    let text = fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn items<'a>(fixture: &'a Value, key: &str) -> &'a [Value] {
    fixture[key]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn text(item: &Value, key: &str) -> String {
    item[key]
        .as_str()
        .unwrap_or_else(|| panic!("the fixture item {item} has no text {key:?}"))
        .to_owned()
}

fn optional_text(item: &Value, key: &str) -> Option<String> {
    item[key].as_str().map(str::to_owned)
}

fn number(item: &Value, key: &str) -> i64 {
    item[key]
        .as_i64()
        .unwrap_or_else(|| panic!("the fixture item {item} has no number {key:?}"))
}

fn flag(item: &Value, key: &str) -> bool {
    item[key]
        .as_bool()
        .unwrap_or_else(|| panic!("the fixture item {item} has no boolean {key:?}"))
}

fn conclusion(name: &str) -> RunConclusion {
    match name {
        "Success" => RunConclusion::Success,
        "Failure" => RunConclusion::Failure,
        "Pending" => RunConclusion::Pending,
        other => panic!("the fixtures spell no conclusion {other:?}"),
    }
}

fn presence(name: &str) -> SpecificationPresence {
    match name {
        "Present" => SpecificationPresence::Present,
        "OptedOut" => SpecificationPresence::OptedOut,
        "Missing" => SpecificationPresence::Missing,
        other => panic!("the fixtures spell no presence {other:?}"),
    }
}

fn validation(name: &str) -> ValidationResult {
    match name {
        "Valid" => ValidationResult::Valid,
        "Refused" => ValidationResult::Refused,
        "NotRun" => ValidationResult::NotRun,
        other => panic!("the fixtures spell no validation {other:?}"),
    }
}

fn activity(name: &str) -> SessionState {
    match name {
        "Busy" => SessionState::Busy,
        "Idle" => SessionState::Idle,
        other => panic!("the fixtures spell no activity {other:?}"),
    }
}

/// A fake collector that records everything the fixture `name` holds.
fn from_fixture(name: &str) -> Collector {
    let fixture = fixture(name);
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        for item in items(&fixture, "repositories") {
            let repository = text(item, "repository");
            record.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(repository.clone()),
                visibility: Visibility::Public,
                archived: flag(item, "archived"),
                local_checkout: flag(item, "local_checkout"),
                main_head: CommitSha(format!("head-of-{repository}")),
                main_committed_at: Timestamp("2026-10-11T12:00:00Z".to_owned()),
                main_commits_7d: 0,
                real_commits_7d: 0,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: number(item, "open_issues"),
                oldest_open_issue_at: None,
                latest_release: optional_text(item, "latest_release"),
                latest_release_at: optional_text(item, "latest_release_at").map(Timestamp),
                unreleased_commits: item["unreleased_commits"].as_i64(),
                planning_store_version: None,
                in_catalog: None,
                gates_hooks_version: None,
                gates_version: None,
                wave_width: None,
                wave_ready: None,
                wave_reason: None,
            })?;
        }
        for item in items(&fixture, "pull_requests") {
            record.pull_request(RecordPullRequest {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(text(item, "repository")),
                number: number(item, "number"),
                title: text(item, "title"),
                draft: item["draft"].as_bool().unwrap_or(false),
                mergeability: Mergeability::Mergeable,
                failing_checks: number(item, "failing_checks"),
                failing_check_names: item["failing_check_names"]
                    .as_array()
                    .map(|names| {
                        names
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
                opened_at: Timestamp(text(item, "opened_at")),
                updated_at: Timestamp(text(item, "opened_at")),
                url: optional_text(item, "url"),
            })?;
        }
        for item in items(&fixture, "workflow_runs") {
            record.workflow_run(RecordWorkflowRun {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(text(item, "repository")),
                workflow: text(item, "workflow"),
                conclusion: conclusion(&text(item, "conclusion")),
                run_at: Timestamp("2026-10-11T12:30:00Z".to_owned()),
            })?;
        }
        for item in items(&fixture, "specifications") {
            record.specification(RecordSpecification {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(text(item, "repository")),
                presence: presence(&text(item, "presence")),
                path: None,
                format: None,
                required_ess: None,
                validation: validation(&text(item, "validation")),
                validation_refusals: 0,
                scenarios: None,
                synthesis_refusals: None,
                conformance_status: None,
            })?;
        }
        for item in items(&fixture, "blockers") {
            record.blocker(RecordBlocker {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(text(item, "repository")),
                reference: text(item, "reference"),
                blocker_kind: text(item, "blocker_kind"),
                title: text(item, "title"),
            })?;
        }
        for item in items(&fixture, "sessions") {
            record.session(RecordSession {
                snapshot_id: snapshot.clone(),
                harness: Harness::Claude,
                session_ref: text(item, "session_ref"),
                name: optional_text(item, "name"),
                cwd: "~/example-org/alpha".to_owned(),
                repository: optional_text(item, "repository").map(RepositoryName),
                role: None,
                activity: activity(&text(item, "activity")),
                envelope_provider: None,
                envelope_cgroup: None,
                envelope_memory_max: None,
                envelope_cpu_weight: None,
                envelope_tasks_max: None,
                envelope_writable: None,
                envelope_unboxed: None,
            })?;
        }
        Ok(())
    })
}

/// A fake collector whose source does not answer.
fn fails() -> Collector {
    Collector::new("fails", |_: &mut Recorder<'_>| {
        bail!("the source did not answer")
    })
}

/// Takes one snapshot into the store under `root`, started at `started_at`, through `collectors`.
fn take(root: &Path, started_at: &str, collectors: &[Collector]) -> Taken {
    let store = store::open(&root.join("state")).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    snapshot::take_from(store, start, collectors).expect("the driver ends the snapshot")
}

/// Takes one complete snapshot of the fixture `name`, started at `started_at`; its id.
fn complete(root: &Path, started_at: &str, name: &str) -> String {
    let taken = take(root, started_at, &[from_fixture(name)]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    taken.snapshot_id.0.0
}

/// Runs the binary from `root`'s `work/` with `HOME` naming `root`'s `home/` and `--state-dir`
/// naming `root`'s `state/`, and asserts it left `work/` empty.
fn conductor(root: &Path, args: &[&str]) -> Output {
    conductor_with(root, None, args)
}

/// [`conductor`], with `CONDUCTOR_CONFIG` naming `config` when it is given.
fn conductor_with(root: &Path, config: Option<&Path>, args: &[&str]) -> Output {
    let work = root.join("work");
    let mut command = common::conductor(root.join("home"));
    command
        .current_dir(&work)
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .stdin(Stdio::null());
    if let Some(config) = config {
        command.env("CONDUCTOR_CONFIG", config);
    }
    let output = command.output().expect("the conductor binary runs");
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

fn path(file: &Path) -> &str {
    file.to_str().expect("the case's path is UTF-8")
}

/// Publishes the snapshot `snapshot` to `root`'s `out/STATUS.md`, asserts it succeeded, and
/// answers the board's text.
fn publish(root: &Path, snapshot: &str) -> String {
    let board = root.join("out/STATUS.md");
    let output = conductor(
        root,
        &[
            "snapshot",
            "publish-board",
            "--snapshot-id",
            snapshot,
            "--path",
            path(&board),
        ],
    );
    assert!(output.status.success(), "{}", describe(&output));
    fs::read_to_string(&board)
        .unwrap_or_else(|error| panic!("read {}: {error}; {}", board.display(), describe(&output)))
}

/// The rows of `conductor snapshot boards --format json` over `root`'s store.
fn boards(root: &Path) -> Vec<Value> {
    let output = conductor(root, &["snapshot", "boards", "--format", "json"]);
    assert!(output.status.success(), "{}", describe(&output));
    match serde_json::from_slice(&output.stdout) {
        Ok(Value::Array(rows)) => rows,
        other => panic!("`snapshot boards` printed {other:?}: {}", describe(&output)),
    }
}

/// The lines of `board` that are headings.
fn headings(board: &str) -> Vec<&str> {
    board.lines().filter(|line| line.starts_with('#')).collect()
}

/// The lines under `heading` up to the next heading.
fn section(board: &str, heading: &str) -> String {
    let mut lines = board.lines().skip_while(|line| *line != heading);
    assert!(
        lines.next().is_some(),
        "the board has no heading {heading:?}:\n{board}"
    );
    lines
        .take_while(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The rows of the Markdown table in `section`, its header first, each cell trimmed; none when
/// the section holds no table.
fn table(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .filter(|line| line.starts_with('|'))
        .map(|line| {
            line.trim()
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect::<Vec<String>>()
        })
        .filter(|cells| {
            !cells
                .iter()
                .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-'))
        })
        .collect()
}

/// The column named `name` of a table's `header`.
fn column(header: &[String], name: &str) -> usize {
    header
        .iter()
        .position(|cell| cell == name)
        .unwrap_or_else(|| panic!("no column {name:?} in {header:?}"))
}

/// Acceptance 1: over a complete snapshot, `publish-board` writes a file whose section
/// headings appear in the story's order, and whose header names the command, the snapshot id and
/// its start time; `snapshot boards` lists the board it recorded.
#[test]
fn the_board_has_its_sections_in_order_and_names_its_snapshot() {
    let root = case_dir("sections");
    let snapshot = complete(&root, EARLIER, "base");

    let board = publish(&root, &snapshot);
    assert_eq!(headings(&board), SECTIONS, "{board}");
    let header = section(&board, "# Status");
    for named in ["snapshot publish-board", snapshot.as_str(), EARLIER] {
        assert!(
            header.contains(named),
            "the header names no {named:?}:\n{board}"
        );
    }

    let rows = boards(&root);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["snapshot_id"], Value::from(snapshot.as_str()));
    assert_eq!(rows[0]["previous_snapshot_id"], Value::Null, "{rows:?}");
    assert_eq!(rows[0]["deviation_rows"], Value::from(1), "{rows:?}");
    assert_eq!(
        rows[0]["path"],
        Value::from(path(&root.join("out/STATUS.md"))),
        "{rows:?}"
    );
}

/// Acceptance 2: a failed snapshot is refused with `SnapshotNotComplete`, and no file is written
/// and no board recorded.
#[test]
fn a_failed_snapshot_is_refused_and_no_file_is_written() {
    let root = case_dir("failed");
    let taken = take(&root, EARLIER, &[from_fixture("base"), fails()]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let board = root.join("out/STATUS.md");

    let output = conductor(
        &root,
        &[
            "snapshot",
            "publish-board",
            "--snapshot-id",
            taken.snapshot_id.0.0.as_str(),
            "--path",
            path(&board),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("SnapshotNotComplete"),
        "{}",
        describe(&output)
    );
    assert_eq!(stderr.lines().count(), 1, "{}", describe(&output));
    assert!(
        !board.exists(),
        "a refused publish wrote {}",
        board.display()
    );
    assert!(
        !root.join("out").exists(),
        "a refused publish created the board's directory"
    );
    assert_eq!(boards(&root), Vec::<Value>::new());
}

/// Acceptance 3, first half: two snapshots that differ only in one pull request's check turning
/// red; the delta against the earlier lists exactly that repository, with `+failing PR` and the
/// failing-PR count `+1`, and every other count unchanged.
#[test]
fn one_check_turning_red_is_the_whole_delta() {
    let root = case_dir("red");
    let earlier = complete(&root, EARLIER, "base");
    let later = complete(&root, LATER, "red");

    let board = publish(&root, &later);
    let delta = section(&board, "## Delta");
    assert!(
        delta.contains(&earlier),
        "the delta names no earlier snapshot:\n{board}"
    );
    let rows = table(&delta);
    let (header, body) = rows
        .split_first()
        .unwrap_or_else(|| panic!("the delta holds no table:\n{board}"));
    assert_eq!(body.len(), 1, "{board}");
    let row = &body[0];
    assert_eq!(row[column(header, "repository")], "alpha", "{board}");
    assert_eq!(row[column(header, "added")], "+failing PR", "{board}");
    assert_eq!(row[column(header, "removed")], "", "{board}");
    assert_eq!(row[column(header, "failing PRs")], "+1", "{board}");
    for unchanged in [
        "open PRs",
        "non-green workflows",
        "open blockers",
        "specifications present",
    ] {
        assert_eq!(row[column(header, unchanged)], "0", "{unchanged}: {board}");
    }

    let rows = boards(&root);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        rows[0]["previous_snapshot_id"],
        Value::from(earlier.as_str())
    );
    assert_eq!(rows[0]["deviation_rows"], Value::from(2), "{rows:?}");
}

/// Acceptance 3, second half: two identical snapshots; the delta reads `no change`.
#[test]
fn identical_snapshots_read_no_change() {
    let root = case_dir("identical");
    complete(&root, EARLIER, "base");
    let later = complete(&root, LATER, "base");

    let board = publish(&root, &later);
    let delta = section(&board, "## Delta");
    assert!(delta.contains("no change"), "{board}");
    assert!(table(&delta).is_empty(), "{board}");
}

/// Acceptance 4: a repository with `local_checkout` false reads `no checkout` in its deviation
/// row; a repository with no deviation has no row, and an archived one none either.
#[test]
fn a_repository_without_a_checkout_reads_no_checkout() {
    let root = case_dir("no-checkout");
    let snapshot = complete(&root, EARLIER, "base");

    let board = publish(&root, &snapshot);
    let rows = table(&section(&board, "## Deviations"));
    let (header, body) = rows
        .split_first()
        .unwrap_or_else(|| panic!("the deviations hold no table:\n{board}"));
    let repository = column(header, "repository");
    let names: Vec<&str> = body.iter().map(|row| row[repository].as_str()).collect();
    assert_eq!(names, ["beta"], "{board}");
    assert!(
        body[0][column(header, "deviations")]
            .split(", ")
            .any(|deviation| deviation == "no checkout"),
        "{board}"
    );

    // The overview lists every repository that is not archived, in name order.
    let rows = table(&section(&board, "## Overview"));
    let (header, body) = rows.split_first().expect("an overview table");
    let repository = column(header, "repository");
    let names: Vec<&str> = body.iter().map(|row| row[repository].as_str()).collect();
    assert_eq!(names, ["alpha", "beta"], "{board}");
}

/// Writes a config file of one instance whose records are `root`'s `records/`, and answers its
/// path.
fn records_config(root: &Path) -> PathBuf {
    records_config_with(root, "")
}

/// [`records_config`], with `extra` lines appended to the instance.
fn records_config_with(root: &Path, extra: &str) -> PathBuf {
    let records = root.join("records");
    fs::create_dir_all(&records).expect("create the records directory");
    let home = root.join("home");
    let file = root.join("conductor.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             {extra}",
            root = home.join("example-org"),
            trees = home.join("trees"),
            records = records,
            state = root.join("state"),
            cache = root.join("cache"),
        ),
    )
    .expect("write the config file");
    file
}

// ---------------------------------------------------------------------------------------------
// story:review-limit
// ---------------------------------------------------------------------------------------------

/// The board of the snapshot `snapshot` under `root`, published with the config file `config`.
fn publish_under(root: &Path, config: Option<&Path>, snapshot: &str) -> String {
    let board = root.join("out/STATUS.md");
    let output = conductor_with(
        root,
        config,
        &[
            "snapshot",
            "publish-board",
            "--snapshot-id",
            snapshot,
            "--path",
            path(&board),
        ],
    );
    assert!(output.status.success(), "{}", describe(&output));
    fs::read_to_string(&board)
        .unwrap_or_else(|error| panic!("read {}: {error}; {}", board.display(), describe(&output)))
}

/// The lines of `board`'s **Waiting for review** section that are not blank.
fn waiting(board: &str) -> Vec<String> {
    section(board, "## Waiting for review")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// Of four open pull requests of `alpha`, the draft and the one with a failing check do not wait
/// for review; the two that do are listed oldest first, `beta`'s younger one after them, each
/// `<repo> #<n> <title> — <age>`. With the limit 2 `alpha` is at it and its limit line
/// follows; with the limit 3 there is none, and `beta`, one under either, never has one.
#[test]
fn the_board_opens_with_what_waits_for_review_oldest_first_and_the_repositories_at_the_limit() {
    let root = case_dir("reviews");
    let snapshot = complete(&root, LATER, "reviews");

    let config = records_config_with(&root, "\x20   controllers: {max_open_reviews: 2}\n");
    let board = publish_under(&root, Some(&config), &snapshot);
    assert_eq!(headings(&board), SECTIONS, "{board}");
    assert_eq!(
        waiting(&board),
        [
            "- alpha #4 Read the catalog — 7d",
            "- alpha #3 Name the exporter — 3d",
            "- beta #9 Tidy the docs — 5h",
            "- alpha: 2 wait for review (limit 2): no new dispatch",
        ],
        "{board}"
    );

    let config = records_config_with(&root, "\x20   controllers: {max_open_reviews: 3}\n");
    let board = publish_under(&root, Some(&config), &snapshot);
    let lines = waiting(&board);
    assert_eq!(lines.len(), 3, "{board}");
    assert!(
        !lines.iter().any(|line| line.contains("no new dispatch")),
        "{board}"
    );
}

/// Without `max_open_reviews` the limit is 3, so three waiting pull requests of `alpha` print the
/// limit line; with `max_open_reviews: 0` there is no limit and no limit line.
#[test]
fn the_limit_is_3_without_the_key_and_none_at_0() {
    let root = case_dir("reviews-default");
    let snapshot = complete(&root, LATER, "reviews-three");

    let board = publish_under(&root, None, &snapshot);
    let lines = waiting(&board);
    assert_eq!(lines.len(), 4, "{board}");
    assert_eq!(
        lines[3], "- alpha: 3 wait for review (limit 3): no new dispatch",
        "{board}"
    );

    let config = records_config_with(&root, "\x20   controllers: {max_open_reviews: 0}\n");
    let board = publish_under(&root, Some(&config), &snapshot);
    let lines = waiting(&board);
    assert_eq!(lines.len(), 3, "{board}");
    assert!(
        !lines.iter().any(|line| line.contains("no new dispatch")),
        "{board}"
    );
}

/// No pull request waits: the section says so.
#[test]
fn a_board_with_nothing_waiting_for_review_says_none() {
    let root = case_dir("reviews-none");
    let snapshot = complete(&root, EARLIER, "red");
    let board = publish_under(&root, None, &snapshot);
    assert_eq!(waiting(&board), ["None."], "{board}");
}

/// Without `--snapshot-id` the board is the newest complete snapshot's, a failed one after it
/// left out; without `--path` it is `STATUS.md` in the instance's records directory.
#[test]
fn without_flags_the_newest_complete_snapshot_goes_to_the_records_status_file() {
    let root = case_dir("defaults");
    let snapshot = complete(&root, EARLIER, "base");
    let failed = take(&root, LATER, &[fails()]);
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");
    let config = records_config(&root);
    let records = root.join("records");

    let output = conductor_with(&root, Some(&config), &["snapshot", "publish-board"]);
    assert!(output.status.success(), "{}", describe(&output));
    let file = records.join("STATUS.md");
    let board = fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}; {}", file.display(), describe(&output)));
    assert!(section(&board, "# Status").contains(&snapshot), "{board}");
    assert!(
        !board.contains(&failed.snapshot_id.0.0),
        "the board names the failed snapshot:\n{board}"
    );
}

// ---------------------------------------------------------------------------------------------
// `conductor status` (story:status-command)
// ---------------------------------------------------------------------------------------------

/// The parts `conductor status` prints, in order: the header names the newest complete snapshot
/// and its age, then its deviations, the live sessions, the open dispatches, the registered waits
/// and the pending requests; and last the usage line (`story:usage-probe`).
const STATUS_PARTS: [&str; 7] = [
    "# Status",
    "## Deviations",
    "## Sessions",
    "## Open dispatches",
    "## Waits",
    "## Pending requests",
    "## Usage",
];

/// How long `conductor status` may take on a fixture store.
const STATUS_WITHIN: std::time::Duration = std::time::Duration::from_secs(2);

/// Runs `args` and answers what it printed, asserting it succeeded.
fn printed(root: &Path, args: &[&str]) -> String {
    let output = conductor(root, args);
    assert!(
        output.status.success(),
        "`{}`: {}",
        args.join(" "),
        describe(&output)
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

/// How many snapshots `root`'s store holds, read through `snapshot snapshots`.
fn snapshot_count(root: &Path) -> usize {
    let text = printed(root, &["snapshot", "snapshots", "--format", "json"]);
    match serde_json::from_str(&text) {
        Ok(Value::Array(rows)) => rows.len(),
        other => panic!("`snapshot snapshots` printed {other:?}"),
    }
}

/// Proposes the goal `G1`, which [`dispatch_to`] sends against.
fn propose_goal(root: &Path) {
    printed(
        root,
        &[
            "goal",
            "propose-goal",
            "--goal-id",
            "G1",
            "--title",
            "Ship the pilot",
            "--exit-evidence",
            "G1 is listed as met in NORTHSTAR.md",
        ],
    );
}

/// Sends one dispatch for `repository` against the goal `G1`; its id.
fn dispatch_to(root: &Path, repository: &str) -> String {
    printed(
        root,
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            repository,
            "--goal-id",
            "G1",
            "--brief",
            &format!("Ship the board of {repository}"),
            "--sent-at",
            LATER,
            "--priority",
            "1",
        ],
    )
    .trim()
    .to_owned()
}

/// Runs `conductor status` with `args` over `root`'s store; what it printed, and asserts it
/// succeeded within [`STATUS_WITHIN`].
fn status_of(root: &Path, args: &[&str]) -> String {
    let mut argv = vec!["status"];
    argv.extend_from_slice(args);
    let started = std::time::Instant::now();
    let output = conductor(root, &argv);
    let took = started.elapsed();
    assert!(output.status.success(), "{}", describe(&output));
    assert!(
        took < STATUS_WITHIN,
        "`conductor status` took {took:?}, more than {STATUS_WITHIN:?}"
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

/// Acceptance 2: `conductor status` prints its six parts from the store: the newest complete
/// snapshot's id and age, its deviations, the sessions it observed, the open dispatches (a done
/// one is not), the registered waits and the pending requests (a decision request and a resource
/// request); within 2 s, and without starting a snapshot.
#[test]
fn status_prints_its_six_parts_from_the_store_and_starts_no_snapshot() {
    let root = case_dir("status-parts");
    let snapshot = complete(&root, EARLIER, "red");
    propose_goal(&root);
    let open = dispatch_to(&root, "alpha");
    let done = dispatch_to(&root, "beta");
    printed(
        &root,
        &["dispatch", "report-started", "--dispatch-id", &done],
    );
    printed(
        &root,
        &[
            "dispatch",
            "report-done",
            "--dispatch-id",
            &done,
            "--evidence",
            "origin/main 0123abc",
        ],
    );
    printed(
        &root,
        &[
            "decision",
            "raise-decision-request",
            "--request-id",
            "REQ-1",
            "--repository",
            "alpha",
            "--question",
            "which token scope?",
            "--options",
            "A: read; B: write",
            "--recommendation",
            "A",
            "--raised-at",
            LATER,
        ],
    );
    let resource = printed(
        &root,
        &[
            "resource",
            "request-resource",
            "--repository",
            "beta",
            "--resource",
            "Disk",
            "--amount",
            "4",
            "--requested-at",
            LATER,
        ],
    )
    .trim()
    .to_owned();
    let snapshots = snapshot_count(&root);

    let status = status_of(&root, &["--full"]);

    assert_eq!(headings(&status), STATUS_PARTS, "{status}");
    let header = section(&status, "# Status");
    for named in [snapshot.as_str(), EARLIER, " ago"] {
        assert!(
            header.contains(named),
            "the header names no {named:?}:\n{status}"
        );
    }

    let deviations = table(&section(&status, "## Deviations"));
    let rows: Vec<(String, String)> = deviations
        .iter()
        .skip(1)
        .map(|row| (row[0].clone(), row[1].clone()))
        .collect();
    assert_eq!(
        rows,
        [
            ("alpha".to_owned(), "failing PR".to_owned()),
            ("beta".to_owned(), "no checkout".to_owned()),
        ],
        "{status}"
    );

    let sessions = section(&status, "## Sessions");
    assert!(
        sessions.contains("alpha") && sessions.contains("Busy"),
        "the sessions part lists no session `alpha`:\n{status}"
    );

    let dispatches = section(&status, "## Open dispatches");
    assert!(
        dispatches.contains(&open),
        "the open dispatch {open} is not listed:\n{status}"
    );
    assert!(
        !dispatches.contains(&done),
        "the done dispatch {done} is listed:\n{status}"
    );

    assert_eq!(section(&status, "## Waits").trim(), "None.", "{status}");

    let requests = section(&status, "## Pending requests");
    for named in ["REQ-1", "which token scope?", resource.as_str()] {
        assert!(
            requests.contains(named),
            "the pending requests name no {named:?}:\n{status}"
        );
    }

    assert_eq!(
        snapshot_count(&root),
        snapshots,
        "`conductor status` started a snapshot"
    );
}

/// Acceptance 2: with no complete snapshot, `conductor status` says so and still prints every
/// other part.
#[test]
fn status_without_a_complete_snapshot_says_so_and_prints_the_rest() {
    let root = case_dir("status-no-snapshot");
    let failed = take(&root, EARLIER, &[fails()]);
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");
    propose_goal(&root);
    let open = dispatch_to(&root, "alpha");

    let status = status_of(&root, &["--full"]);

    assert_eq!(headings(&status), STATUS_PARTS, "{status}");
    assert!(
        section(&status, "# Status").contains("No snapshot is complete"),
        "the header does not say no snapshot is complete:\n{status}"
    );
    assert!(
        !status.contains(&failed.snapshot_id.0.0),
        "the status names the failed snapshot:\n{status}"
    );
    assert!(
        section(&status, "## Open dispatches").contains(&open),
        "the open dispatch {open} is not listed:\n{status}"
    );
    assert_eq!(
        snapshot_count(&root),
        1,
        "`conductor status` started a snapshot"
    );
}

/// `conductor status --format json` prints the same parts as one object.
#[test]
fn status_prints_its_parts_as_json() {
    let root = case_dir("status-json");
    let snapshot = complete(&root, EARLIER, "base");

    let text = status_of(&root, &["--format", "json"]);
    let status: Value = serde_json::from_str(&text).expect("the status is JSON");

    assert_eq!(
        status["snapshot_id"],
        Value::from(snapshot.as_str()),
        "{status}"
    );
    assert_eq!(
        status["snapshot_started_at"],
        Value::from(EARLIER),
        "{status}"
    );
    assert!(
        status["snapshot_age_seconds"].as_i64().is_some(),
        "{status}"
    );
    let deviations = status["deviations"].as_array().expect("deviations");
    assert_eq!(deviations.len(), 1, "{status}");
    assert_eq!(deviations[0]["repository"], "beta", "{status}");
    assert_eq!(deviations[0]["deviations"][0], "no checkout", "{status}");
    assert_eq!(
        status["sessions"].as_array().map(Vec::len),
        Some(1),
        "{status}"
    );
    for part in ["dispatches", "waits", "requests"] {
        assert_eq!(status[part], Value::Array(Vec::new()), "{part}: {status}");
    }
}

/// Raises the decision request `id` of `alpha` at `raised_at`.
fn raise_at(root: &Path, id: &str, raised_at: &str) {
    printed(
        root,
        &[
            "decision",
            "raise-decision-request",
            "--request-id",
            id,
            "--repository",
            "alpha",
            "--question",
            "which token scope?",
            "--options",
            "A: read; B: write",
            "--recommendation",
            "A",
            "--raised-at",
            raised_at,
        ],
    );
}

/// `story:decisions-notify`, acceptance 2: `conductor status` counts the requests escalated as
/// class O or H and not answered, and names the oldest with its age, in text and in JSON; an open
/// request and a withdrawn escalation are not counted.
#[test]
fn status_counts_the_unanswered_operator_items_and_the_oldest_one_s_age() {
    let root = case_dir("status-unanswered");
    complete(&root, EARLIER, "base");
    raise_at(&root, "REQ-OLD", EARLIER);
    raise_at(&root, "REQ-NEW", LATER);
    raise_at(&root, "REQ-OPEN", EARLIER);
    raise_at(&root, "REQ-GONE", "2026-10-11T08:00:00Z");
    for (id, class) in [("REQ-NEW", "O"), ("REQ-OLD", "H"), ("REQ-GONE", "O")] {
        let mut args = vec![
            "decision",
            "escalate-request",
            "--request-id",
            id,
            "--escalation-class",
            class,
            "--reason",
            "the operator's call",
        ];
        if class == "H" {
            args.extend(["--no-dry-run", "the step needs the operator's login"]);
        } else {
            // What the operator chooses from (story:decisions-page).
            args.extend([
                "--kind",
                "yes-no",
                "--lettered-options",
                r#"[{"letter":"Y","label":"yes","cost":"an hour"},{"letter":"N","label":"no","cost":"nothing"}]"#,
                "--recommended-letters",
                "Y",
                "--default-letters",
                "N",
                "--deadline",
                "2026-10-20T12:00:00Z",
            ]);
        }
        printed(&root, &args);
    }
    printed(
        &root,
        &[
            "decision",
            "withdraw-request",
            "--request-id",
            "REQ-GONE",
            "--reason",
            "no longer needed",
        ],
    );
    let raised =
        time::OffsetDateTime::parse(EARLIER, &time::format_description::well_known::Rfc3339)
            .expect("a fixed instant");

    let before = (time::OffsetDateTime::now_utc() - raised).whole_seconds();
    let text = status_of(&root, &["--full"]);
    let json: Value =
        serde_json::from_str(&status_of(&root, &["--format", "json"])).expect("the status is JSON");
    let after = (time::OffsetDateTime::now_utc() - raised).whole_seconds();

    assert!(
        section(&text, "# Status")
            .contains("Unanswered O/H items: 2; the oldest, REQ-OLD, was raised "),
        "the header gives no count of unanswered O/H items:\n{text}"
    );
    assert_eq!(json["unanswered_operator_items"], 2, "{json}");
    assert_eq!(json["oldest_unanswered_request_id"], "REQ-OLD", "{json}");
    let age = json["oldest_unanswered_age_seconds"]
        .as_i64()
        .unwrap_or_else(|| panic!("no age of the oldest item: {json}"));
    assert!(
        (before..=after).contains(&age),
        "the oldest item's age {age} is not between {before} and {after}"
    );
}

/// `story:decisions-notify`: with no unanswered O/H item the count is 0 and no oldest is named.
#[test]
fn status_without_an_unanswered_operator_item_counts_none() {
    let root = case_dir("status-unanswered-none");
    complete(&root, EARLIER, "base");
    raise_at(&root, "REQ-OPEN", EARLIER);

    let text = status_of(&root, &["--full"]);
    let json: Value =
        serde_json::from_str(&status_of(&root, &["--format", "json"])).expect("the status is JSON");

    assert!(
        section(&text, "# Status").contains("Unanswered O/H items: 0."),
        "{text}"
    );
    assert_eq!(json["unanswered_operator_items"], 0, "{json}");
    assert_eq!(json["oldest_unanswered_request_id"], Value::Null, "{json}");
    assert_eq!(json["oldest_unanswered_age_seconds"], Value::Null, "{json}");
}

/// `story:conductor-context-budget`: without `--full`, `conductor status` prints only what asks
/// for attention, one line each and no heading: the snapshot, each repository's deviations with
/// its failing checks, each blocked dispatch, each pending request; a dispatch that is only sent
/// and the sessions are left out, and one closing line counts them and names `--full`. The
/// unanswered operator items are named when there are any.
#[test]
fn status_prints_the_attention_lines_by_default_and_full_prints_every_part() {
    let root = case_dir("status-attention");
    let snapshot = complete(&root, EARLIER, "checks");
    propose_goal(&root);
    let sent = dispatch_to(&root, "alpha");
    let blocked = dispatch_to(&root, "beta");
    printed(
        &root,
        &["dispatch", "report-started", "--dispatch-id", &blocked],
    );
    printed(
        &root,
        &[
            "dispatch",
            "report-blocked",
            "--dispatch-id",
            &blocked,
            "--reason",
            "waits on a token",
        ],
    );
    raise_at(&root, "REQ-1", LATER);

    let brief = status_of(&root, &[]);
    let full = status_of(&root, &["--full"]);

    assert!(
        brief.lines().all(|line| !line.starts_with('#')),
        "the attention lines carry no heading:\n{brief}"
    );
    let has = |start: &str| brief.lines().any(|line| line.starts_with(start));
    assert!(
        has(&format!("snapshot {snapshot}")),
        "a line names the snapshot:\n{brief}"
    );
    assert!(
        brief
            .lines()
            .any(|line| line.starts_with("deviation alpha: failing PR")
                && line.contains(&CHECKS_ALPHA.join("; "))),
        "alpha's deviations with its failing checks, on one line:\n{brief}"
    );
    assert!(
        has("deviation beta: no checkout"),
        "beta's deviation:\n{brief}"
    );
    assert!(
        has(&format!("blocked {blocked} beta")),
        "the blocked dispatch:\n{brief}"
    );
    assert!(
        brief
            .lines()
            .any(|line| line.starts_with("request decision REQ-1 alpha")
                && line.contains("which token scope?")),
        "the pending request:\n{brief}"
    );
    assert!(
        !brief.contains(&sent),
        "a dispatch that is only sent asks for no attention:\n{brief}"
    );
    assert!(
        !brief.contains("Busy"),
        "the sessions are left out:\n{brief}"
    );
    let last = brief.lines().last().unwrap_or_default();
    assert!(
        last.contains("2 open dispatches") && last.contains("--full"),
        "the closing line counts what it left out and names --full: {last:?}"
    );
    assert!(
        !has("unanswered"),
        "no unanswered operator item, no line for it:\n{brief}"
    );

    assert_eq!(headings(&full), STATUS_PARTS, "{full}");
    assert!(
        section(&full, "## Open dispatches").contains(&sent),
        "--full lists the sent dispatch:\n{full}"
    );
    assert!(
        brief.len() * 2 < full.len(),
        "the attention lines are under half of the full status: {} of {} bytes",
        brief.len(),
        full.len()
    );
}

/// Rule 7: `conductor status` is a view; it opens only an existing store and creates none.
#[test]
fn status_creates_no_store() {
    let root = case_dir("status-no-store");
    let output = conductor(&root, &["status"]);
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(
        !root.join("state").exists(),
        "`conductor status` created a store: {}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:pr-check-names
// ---------------------------------------------------------------------------------------------

/// What `checks.json` reads in the `failing checks` column: one entry per failing pull request of
/// `alpha`, by number, every failing check named; `#8` alone, whose observation names none.
const CHECKS_ALPHA: [&str; 3] = ["#7 Security, test (shard 3), docs", "#8", "#9 lint"];

/// The `failing checks` cell of `repository`'s row in the Deviations table of `text`.
fn failing_checks_cell(text: &str, repository: &str) -> String {
    let rows = table(&section(text, "## Deviations"));
    let (header, body) = rows
        .split_first()
        .unwrap_or_else(|| panic!("the deviations hold no table:\n{text}"));
    let at = column(header, "failing checks");
    body.iter()
        .find(|row| row[0] == repository)
        .map(|row| row[at].clone())
        .unwrap_or_else(|| panic!("no deviation row for {repository}:\n{text}"))
}

/// `conductor status` names every failing check of each pull request with one, in its text and
/// in its JSON; a repository without a failing pull request names none.
#[test]
fn status_names_every_failing_check_of_a_pull_request() {
    let root = case_dir("status-check-names");
    complete(&root, EARLIER, "checks");

    let status = status_of(&root, &["--full"]);
    assert_eq!(
        failing_checks_cell(&status, "alpha"),
        CHECKS_ALPHA.join("; "),
        "{status}"
    );
    assert_eq!(failing_checks_cell(&status, "beta"), "", "{status}");

    let json: Value =
        serde_json::from_str(&status_of(&root, &["--format", "json"])).expect("the status is JSON");
    let deviations = json["deviations"].as_array().expect("deviations");
    let alpha = deviations
        .iter()
        .find(|row| row["repository"] == "alpha")
        .unwrap_or_else(|| panic!("no alpha: {json}"));
    assert_eq!(
        alpha["failing_checks"],
        serde_json::json!(CHECKS_ALPHA),
        "{json}"
    );
    assert_eq!(alpha["failing_prs"], 3, "{json}");
}

/// The board names every failing check of each pull request with one in its Deviations table.
#[test]
fn the_board_names_every_failing_check_of_a_pull_request() {
    let root = case_dir("board-check-names");
    let snapshot = complete(&root, EARLIER, "checks");

    let board = publish(&root, &snapshot);
    assert_eq!(
        failing_checks_cell(&board, "alpha"),
        CHECKS_ALPHA.join("; "),
        "{board}"
    );
    assert_eq!(failing_checks_cell(&board, "beta"), "", "{board}");
}
