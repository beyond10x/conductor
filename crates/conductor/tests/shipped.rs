//! `story:release-digest`: what shipped since an instant, read from the snapshot's GitHub
//! observations.
//!
//! The `github` collector records each repository's releases and merged pull requests of the 14
//! days before its clock. Every `gh` command line is `cat` of a fixture under
//! `tests/fixtures/shipped/`, so no case reaches the network or a real checkout. Each case takes
//! its snapshots through the library ([`snapshot::take_from`], at a start the case chooses) into
//! its own directory in this test target's temporary directory, and reads them back through the
//! built `conductor`, run from the case's empty `work/` with `--state-dir` naming its `state/`.

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;

use anyhow::bail;
use conductor_cli::collect::{Collector, github};
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::{MergedPullRequestsQuery, ReleasesQuery};
use conductor_model::observation::{
    RecordMergedPullRequest, RecordRelease, RepositoryName, SnapshotState, StartSnapshot,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The collector's clock, and the start of the snapshot it records into.
const NOW: &str = "2026-10-07T12:00:00Z";

/// Where the 14-day window before [`NOW`] opens.
const WINDOW_START: &str = "2026-09-23T12:00:00Z";

/// The columns of `conductor repository shipped`, in order.
const COLUMNS: [&str; 7] = [
    "repository",
    "release",
    "published_at",
    "number",
    "title",
    "merged_at",
    "url",
];

// ---------------------------------------------------------------------------------------------
// The case's directory, the sources and the binary
// ---------------------------------------------------------------------------------------------

/// A fresh directory for one case: its `state/` is the case's store, and its empty `work/` the
/// working directory of every run of the binary.
fn case_dir(case: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("shipped")
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

/// `cat` of the fixture `name`, whose `{repository}` and `{since}` the collector fills.
fn cat(name: &str) -> Vec<String> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/shipped")
        .join(name);
    vec![
        "cat".to_owned(),
        fixture
            .to_str()
            .expect("the fixture's path is UTF-8")
            .to_owned(),
    ]
}

fn now() -> OffsetDateTime {
    OffsetDateTime::parse(NOW, &Rfc3339).expect("NOW is RFC 3339")
}

/// The organization `organization.json` lists, with no open pull request and no `main` run.
fn sources() -> github::Sources {
    github::Sources {
        organization: "example-org".to_owned(),
        repository_list: cat("organization.json"),
        pull_requests: cat("empty.json"),
        workflow_runs: cat("empty.json"),
        width: 8,
        bound: Duration::from_secs(60),
    }
}

/// Each repository's releases in `<repository>.releases.json`, and its pull requests merged since
/// the window's first day in `<repository>.merged.<day>.json`, at the clock [`NOW`], with the live
/// limits. No fixture list fills its first limit.
fn shipments() -> github::Shipments {
    github::Shipments {
        releases: cat("{repository}.releases.json"),
        merged_pull_requests: cat("{repository}.merged.{since}.json"),
        ..github::Shipments::live(now())
    }
}

/// The `github` collector over `sources` and `shipments`.
fn github(sources: github::Sources, shipments: github::Shipments) -> Collector {
    Collector::new("github", move |record| {
        github::collect_all_from(&sources, &shipments, record)
    })
}

/// Takes one snapshot started at `started_at` into the store under `root` through `collectors`.
fn take_at(root: &Path, started_at: &str, collectors: &[Collector]) -> Taken {
    let store = store::open(&root.join("state")).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1,
    };
    snapshot::take_from(store, start, collectors).expect("the driver ends the snapshot")
}

/// The snapshot of the fixtures, started at [`NOW`], which must complete.
fn take_fixtures(root: &Path) -> Taken {
    let taken = take_at(root, NOW, &[github(sources(), shipments())]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    taken
}

/// Runs the binary from `root`'s `work/` with `--state-dir` naming `root`'s `state/`, and asserts
/// it left `work/` empty.
fn conductor(root: &Path, args: &[&str]) -> Output {
    let work = root.join("work");
    let state = root.join("state");
    let output = common::conductor(root.join("home"))
        .current_dir(&work)
        .args(["--state-dir", path(&state)])
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

/// `conductor repository shipped --since <since>`, with `--format <format>` when one is given.
fn shipped(root: &Path, since: &str, format: Option<&str>) -> Output {
    let mut args = vec!["repository", "shipped", "--since", since];
    if let Some(format) = format {
        args.extend(["--format", format]);
    }
    conductor(root, &args)
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
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

/// The rows a `--format json` run printed, after asserting it exited 0.
fn json_rows(output: &Output) -> Vec<Value> {
    assert!(output.status.success(), "{}", describe(output));
    match serde_json::from_str(&stdout(output)) {
        Ok(Value::Array(rows)) => rows,
        other => panic!("printed no JSON array: {other:?}; {}", describe(output)),
    }
}

/// Each line of a text table, as its cells: the line split where two or more spaces separate
/// two values.
fn cells(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(|line| {
            line.split("  ")
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

/// A release row of the digest.
fn release(repository: &str, tag: &str, published_at: &str) -> Value {
    json!({
        "repository": repository,
        "release": tag,
        "published_at": published_at,
        "number": null,
        "title": null,
        "merged_at": null,
        "url": format!("https://github.com/example-org/{repository}/releases/tag/{tag}"),
    })
}

/// A pull-request row of the digest, under the release `under` (`tag`, `published_at`), or
/// "merged, not released" when `under` is `None`.
fn pull(
    repository: &str,
    under: Option<(&str, &str)>,
    number: i64,
    title: &str,
    merged_at: &str,
) -> Value {
    let (release, published_at) = match under {
        Some((tag, published_at)) => (json!(tag), json!(published_at)),
        None => (json!("merged, not released"), Value::Null),
    };
    json!({
        "repository": repository,
        "release": release,
        "published_at": published_at,
        "number": number,
        "title": title,
        "merged_at": merged_at,
        "url": format!("https://github.com/example-org/{repository}/pull/{number}"),
    })
}

/// The release v0.2.0 of `alpha`, as the fixtures publish it (`+02:00`, kept in UTC).
const V020: (&str, &str) = ("v0.2.0", "2026-10-05T10:00:00Z");

/// A row as the text table shows it: each value as it is, `null` for none.
fn shown(row: &Value) -> Vec<String> {
    COLUMNS
        .iter()
        .map(|column| match &row[*column] {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect()
}

fn header() -> Vec<String> {
    COLUMNS.iter().map(|column| (*column).to_owned()).collect()
}

// ---------------------------------------------------------------------------------------------
// The collector
// ---------------------------------------------------------------------------------------------

/// The collector records, per non-archived repository, the releases published in the 14 days
/// before its clock (drafts and prereleases aside), each with its link, and the pull requests
/// merged in them, each with GitHub's link; every instant in UTC. `gamma` is archived, so no
/// command names it (it has no fixture, and `cat` of one would fail the snapshot).
#[test]
fn the_collector_records_the_windows_releases_and_merged_pull_requests() {
    let root = case_dir("collector");
    let taken = take_fixtures(&root);

    let generated = Generated::new(store::open_existing(&root.join("state")).expect("open"));
    let releases: Vec<(String, String, String, String)> = generated
        .releases()
        .expect("the releases view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .map(|row| (row.repository.0, row.tag, row.published_at.0, row.url))
        .collect();
    assert_eq!(
        releases,
        vec![(
            "alpha".to_owned(),
            "v0.2.0".to_owned(),
            "2026-10-05T10:00:00Z".to_owned(),
            "https://github.com/example-org/alpha/releases/tag/v0.2.0".to_owned(),
        )],
        "the prerelease and the release before the window are not recorded"
    );

    let merged: Vec<(String, i64, String, String, String)> = generated
        .merged_pull_requests()
        .expect("the merged pull requests view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .map(|row| {
            (
                row.repository.0,
                row.number,
                row.title,
                row.merged_at.0,
                row.url,
            )
        })
        .collect();
    let expected = [
        ("alpha", 10, "Start the parser", "2026-09-28T10:00:00Z"),
        ("alpha", 12, "Add the parser", "2026-10-04T09:00:00Z"),
        ("alpha", 14, "Fix the clock", "2026-10-06T08:00:00Z"),
        ("beta", 3, "Document the API", "2026-10-01T13:30:00Z"),
    ]
    .map(|(repository, number, title, merged_at)| {
        (
            repository.to_owned(),
            number,
            title.to_owned(),
            merged_at.to_owned(),
            format!("https://github.com/example-org/{repository}/pull/{number}"),
        )
    });
    assert_eq!(
        merged, expected,
        "repositories in name order, pull requests by number; #9 merged before the window opened"
    );
    generated.ports.check().expect("the store read every row");
}

/// What the live collector runs, as the story's decision states it.
#[test]
fn the_live_shipments_are_the_gh_commands_the_story_names() {
    let live = github::Shipments::live(now());
    assert_eq!(
        live.releases,
        [
            "gh",
            "release",
            "list",
            "-R",
            "{organization}/{repository}",
            "--exclude-drafts",
            "--limit",
            "{limit}",
            "--json",
            "tagName,publishedAt,isPrerelease",
        ]
    );
    assert_eq!(
        live.release_limits,
        github::Limits {
            first: 20,
            larger: 100
        },
        "20 releases, and 100 when 20 fill the window"
    );
    assert_eq!(
        live.merged_pull_requests,
        [
            "gh",
            "pr",
            "list",
            "-R",
            "{organization}/{repository}",
            "--state",
            "merged",
            "--search",
            "merged:>={since}",
            "--limit",
            "{limit}",
            "--json",
            "number,title,mergedAt,url",
        ]
    );
    assert_eq!(
        live.merged_limits,
        github::Limits {
            first: 200,
            larger: 1000
        },
        "200 merged pull requests, and 1000 when 200 fill the window"
    );
    assert_eq!(live.now, now());
}

// ---------------------------------------------------------------------------------------------
// A shipped list that fills (wave 07, coordinator decision 3)
// ---------------------------------------------------------------------------------------------

/// Where a case's generated lists are kept: `lists/` in its directory.
fn lists(root: &Path) -> PathBuf {
    let dir = root.join("lists");
    fs::create_dir_all(&dir).expect("create the case's lists");
    dir
}

/// The organization holding `alpha` only, with no open pull request and no `main` run.
fn alpha_only(root: &Path) -> github::Sources {
    let organization = lists(root).join("organization.json");
    fs::write(&organization, r#"[{"name": "alpha", "isArchived": false}]"#)
        .expect("write the organization");
    github::Sources {
        repository_list: vec!["cat".to_owned(), path(&organization).to_owned()],
        ..sources()
    }
}

/// The live shipments, at the clock [`NOW`], each `gh` command line replaced by `cat` of
/// `lists/<repository>.releases.<limit>.json` and `lists/<repository>.merged.<limit>.json` in the
/// case's directory: a list the collector asks for at a limit with no file fails the snapshot.
fn limited(root: &Path) -> github::Shipments {
    let dir = lists(root);
    let file = |name: &str| vec!["cat".to_owned(), path(&dir.join(name)).to_owned()];
    github::Shipments {
        releases: file("{repository}.releases.{limit}.json"),
        merged_pull_requests: file("{repository}.merged.{limit}.json"),
        ..github::Shipments::live(now())
    }
}

/// `count` instants a minute apart, the newest `newest_minutes_ago` minutes before [`NOW`], each
/// in RFC 3339.
fn instants(count: usize, newest_minutes_ago: i64) -> Vec<String> {
    (0..count)
        .map(|index| {
            let back = newest_minutes_ago + i64::try_from(index).expect("a small count");
            (now() - time::Duration::minutes(back))
                .format(&Rfc3339)
                .expect("format an instant")
        })
        .collect()
}

/// Writes `alpha`'s releases at `limit`, one published at each of `published`, newest first.
fn write_releases(root: &Path, limit: usize, published: &[String]) {
    let releases: Vec<Value> = published
        .iter()
        .enumerate()
        .map(|(index, at)| {
            json!({"tagName": format!("v0.0.{index}"), "publishedAt": at, "isPrerelease": false})
        })
        .collect();
    fs::write(
        lists(root).join(format!("alpha.releases.{limit}.json")),
        Value::from(releases).to_string(),
    )
    .expect("write the releases");
}

/// Writes `alpha`'s merged pull requests at `limit`, one merged at each of `merged`.
fn write_merged(root: &Path, limit: usize, merged: &[String]) {
    let pulls: Vec<Value> = merged
        .iter()
        .enumerate()
        .map(|(index, at)| {
            let number = index + 1;
            json!({
                "number": number,
                "title": format!("Change {number}"),
                "mergedAt": at,
                "url": format!("https://github.com/example-org/alpha/pull/{number}"),
            })
        })
        .collect();
    fs::write(
        lists(root).join(format!("alpha.merged.{limit}.json")),
        Value::from(pulls).to_string(),
    )
    .expect("write the merged pull requests");
}

/// How many releases and merged pull requests the snapshot `taken` recorded.
fn recorded(root: &Path, taken: &Taken) -> (usize, usize) {
    let generated = Generated::new(store::open_existing(&root.join("state")).expect("open"));
    let releases = generated
        .releases()
        .expect("the releases view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .count();
    let merged = generated
        .merged_pull_requests()
        .expect("the merged pull requests view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .count();
    generated.ports.check().expect("the store read every row");
    (releases, merged)
}

/// 20 releases, all inside the window, fill the first list: the collector asks again with
/// `--limit 100` and records the larger answer's releases in the window, 25 of its 30. The
/// merged pull requests do not fill 200, so they are not asked for again (there is no 1000 file).
#[test]
fn a_full_release_list_inside_the_window_is_asked_for_again_larger() {
    let root = case_dir("fill-releases");
    write_releases(&root, 20, &instants(20, 60));
    let mut larger = instants(25, 60);
    larger.extend(instants(5, 20 * 24 * 60));
    write_releases(&root, 100, &larger);
    write_merged(&root, 200, &instants(3, 60));

    let taken = take_at(&root, NOW, &[github(alpha_only(&root), limited(&root))]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(recorded(&root, &taken), (25, 3));
}

/// 20 releases, of which the oldest was published before the window opened, hold the whole
/// window: they are not asked for again (there is no 100 file), and the 19 inside it are recorded.
#[test]
fn a_full_release_list_that_reaches_past_the_window_is_not_asked_for_again() {
    let root = case_dir("full-past-window");
    let mut published = instants(19, 60);
    published.insert(7, "2026-09-20T00:00:00Z".to_owned());
    write_releases(&root, 20, &published);
    write_merged(&root, 200, &instants(3, 60));

    let taken = take_at(&root, NOW, &[github(alpha_only(&root), limited(&root))]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(recorded(&root, &taken), (19, 3));
}

/// When the larger release list fills the window too, the snapshot fails naming the repository,
/// the list and the limit, and nothing of the repository is recorded: no silent cut.
#[test]
fn a_larger_release_list_that_fills_too_fails_naming_the_repository_list_and_limit() {
    let root = case_dir("fill-releases-twice");
    write_releases(&root, 20, &instants(20, 60));
    write_releases(&root, 100, &instants(100, 60));
    write_merged(&root, 200, &instants(3, 60));

    let taken = take_at(&root, NOW, &[github(alpha_only(&root), limited(&root))]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with("github: repository alpha: ")
            && reason.contains("releases")
            && reason.contains("--limit 100"),
        "{reason:?}"
    );
    assert_eq!(recorded(&root, &taken), (0, 0));
}

/// The merged pull requests the same way: 200 inside the window are asked for again with
/// `--limit 1000`, whose 250 are recorded; when 1000 fill the window too, the snapshot fails
/// naming the repository, the list and the limit, and records nothing.
#[test]
fn a_full_merged_list_is_asked_for_again_larger_and_fails_when_that_fills_too() {
    let root = case_dir("fill-merged");
    write_releases(&root, 20, &instants(2, 60));
    write_merged(&root, 200, &instants(200, 60));
    write_merged(&root, 1000, &instants(250, 60));
    let taken = take_at(&root, NOW, &[github(alpha_only(&root), limited(&root))]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(recorded(&root, &taken), (2, 250));

    let root = case_dir("fill-merged-twice");
    write_releases(&root, 20, &instants(2, 60));
    write_merged(&root, 200, &instants(200, 1));
    write_merged(&root, 1000, &instants(1000, 1));
    let taken = take_at(&root, NOW, &[github(alpha_only(&root), limited(&root))]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with("github: repository alpha: ")
            && reason.contains("merged pull requests")
            && reason.contains("--limit 1000"),
        "{reason:?}"
    );
    assert_eq!(recorded(&root, &taken), (0, 0));
}

/// A releases source that does not answer fails the snapshot naming the repository and the
/// command, and nothing of that repository's GitHub state is recorded.
#[test]
fn a_releases_source_that_fails_fails_the_snapshot_naming_it() {
    let root = case_dir("failing");
    let mut shipments = shipments();
    shipments.releases = vec!["false".to_owned()];
    let taken = take_at(&root, NOW, &[github(sources(), shipments)]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.unwrap_or_default();
    assert!(
        reason.starts_with("github: repository ") && reason.contains("`false` exited 1"),
        "{reason:?}"
    );
    let generated = Generated::new(store::open_existing(&root.join("state")).expect("open"));
    assert_eq!(generated.releases().expect("releases").len(), 0);
    assert_eq!(generated.merged_pull_requests().expect("merged").len(), 0);
}

// ---------------------------------------------------------------------------------------------
// `conductor repository shipped`
// ---------------------------------------------------------------------------------------------

/// Acceptance: over two repositories' observations (alpha: one release and two merged pull
/// requests since the instant, one of them after the release; beta: one merged pull request), the
/// digest lists alpha's release with its link, the earlier pull request under it, and the later
/// one as "merged, not released", then beta's pull request the same way. `delta`, with nothing
/// since the instant, is left out, and so is #10, merged before it.
#[test]
fn shipped_lists_each_release_with_what_merged_before_it_and_the_rest_as_not_released() {
    let root = case_dir("acceptance");
    take_fixtures(&root);
    let since = "2026-10-01T00:00:00Z";

    let expected = vec![
        release("alpha", V020.0, V020.1),
        pull(
            "alpha",
            Some(V020),
            12,
            "Add the parser",
            "2026-10-04T09:00:00Z",
        ),
        pull("alpha", None, 14, "Fix the clock", "2026-10-06T08:00:00Z"),
        pull("beta", None, 3, "Document the API", "2026-10-01T13:30:00Z"),
    ];
    let output = shipped(&root, since, Some("json"));
    assert_eq!(json_rows(&output), expected, "{}", describe(&output));
    assert_eq!(
        stderr(&output),
        "",
        "nothing on stderr: {}",
        describe(&output)
    );

    let output = shipped(&root, since, None);
    assert!(output.status.success(), "{}", describe(&output));
    let mut table = vec![header()];
    table.extend(expected.iter().map(shown));
    assert_eq!(cells(&stdout(&output)), table, "{}", describe(&output));

    let output = shipped(&root, since, Some("jsonl"));
    assert!(output.status.success(), "{}", describe(&output));
    let lines: Vec<Value> = stdout(&output)
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
        .collect();
    assert_eq!(lines, expected);

    let output = shipped(&root, since, Some("markdown"));
    assert!(output.status.success(), "{}", describe(&output));
    let markdown = stdout(&output);
    assert_eq!(
        markdown.lines().next(),
        Some("| repository | release | published_at | number | title | merged_at | url |"),
        "{markdown}"
    );
    assert_eq!(markdown.lines().count(), 2 + expected.len(), "{markdown}");
}

/// An instant older than the window prints one first line saying where the window starts, ahead
/// of the rows, which then include everything the window holds. In `json` and `jsonl` the line is
/// on standard error, so standard output stays one JSON document. An instant at the window's
/// start prints no such line.
#[test]
fn an_instant_older_than_the_window_is_named_on_the_first_line() {
    let root = case_dir("window");
    take_fixtures(&root);
    let since = "2026-09-01T00:00:00Z";
    let note = format!("the window starts at {WINDOW_START}");

    let expected = vec![
        release("alpha", V020.0, V020.1),
        pull(
            "alpha",
            Some(V020),
            10,
            "Start the parser",
            "2026-09-28T10:00:00Z",
        ),
        pull(
            "alpha",
            Some(V020),
            12,
            "Add the parser",
            "2026-10-04T09:00:00Z",
        ),
        pull("alpha", None, 14, "Fix the clock", "2026-10-06T08:00:00Z"),
        pull("beta", None, 3, "Document the API", "2026-10-01T13:30:00Z"),
    ];

    let output = shipped(&root, since, None);
    assert!(output.status.success(), "{}", describe(&output));
    let text = stdout(&output);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some(note.as_str()), "{text}");
    let rest: String = lines.map(|line| format!("{line}\n")).collect();
    let mut table = vec![header()];
    table.extend(expected.iter().map(shown));
    assert_eq!(cells(&rest), table, "{text}");

    let output = shipped(&root, since, Some("json"));
    assert_eq!(json_rows(&output), expected, "{}", describe(&output));
    assert_eq!(
        stderr(&output),
        format!("{note}\n"),
        "{}",
        describe(&output)
    );

    let output = shipped(&root, WINDOW_START, None);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(
        cells(&stdout(&output)).first(),
        Some(&header()),
        "an instant at the window's start prints no window line: {}",
        describe(&output)
    );
}

/// A release published before the instant is not listed, and still closes what merged before it:
/// only what merged after the newest release is "merged, not released". A repository with nothing
/// since the instant (beta, here) is left out.
#[test]
fn a_release_before_the_instant_is_left_out_and_still_closes_what_merged_before_it() {
    let root = case_dir("after-release");
    take_fixtures(&root);
    let output = shipped(&root, "2026-10-05T12:00:00Z", Some("json"));
    assert_eq!(
        json_rows(&output),
        vec![pull(
            "alpha",
            None,
            14,
            "Fix the clock",
            "2026-10-06T08:00:00Z"
        )],
        "{}",
        describe(&output)
    );

    let output = shipped(&root, "2026-10-07T00:00:00Z", None);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(
        cells(&stdout(&output)),
        vec![header()],
        "nothing shipped since the instant: the header alone"
    );
}

/// A fake collector that records one release and one merged pull request of `repository`, then
/// answers `fail` as its error when one is given.
fn records(
    repository: &'static str,
    tag: &'static str,
    at: &'static str,
    number: i64,
    fail: Option<&'static str>,
) -> Collector {
    Collector::new("records", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        record.release(RecordRelease {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(repository.to_owned()),
            tag: tag.to_owned(),
            published_at: Timestamp(at.to_owned()),
            url: format!("https://github.com/example-org/{repository}/releases/tag/{tag}"),
        })?;
        record.merged_pull_request(RecordMergedPullRequest {
            snapshot_id: snapshot,
            repository: RepositoryName(repository.to_owned()),
            number,
            title: format!("Pull request {number}"),
            merged_at: Timestamp(at.to_owned()),
            url: format!("https://github.com/example-org/{repository}/pull/{number}"),
        })?;
        match fail {
            Some(reason) => bail!("{reason}"),
            None => Ok(()),
        }
    })
}

/// The digest reads the complete snapshot that started last, whatever order the snapshots were
/// taken in, and never a failed one.
#[test]
fn shipped_reads_only_the_newest_complete_snapshot() {
    let root = case_dir("newest");
    take_fixtures(&root);
    let older = take_at(
        &root,
        "2026-10-06T12:00:00Z",
        &[records("alpha", "v0.1.9", "2026-10-06T09:00:00Z", 77, None)],
    );
    assert_eq!(older.state, SnapshotState::Complete, "{older:?}");
    let newer = take_at(
        &root,
        "2026-10-07T13:00:00Z",
        &[records(
            "alpha",
            "v9.9.9",
            "2026-10-07T12:30:00Z",
            99,
            Some("the source did not answer"),
        )],
    );
    assert_eq!(newer.state, SnapshotState::Failed, "{newer:?}");

    let output = shipped(&root, "2026-10-01T00:00:00Z", Some("json"));
    assert_eq!(
        json_rows(&output),
        vec![
            release("alpha", V020.0, V020.1),
            pull(
                "alpha",
                Some(V020),
                12,
                "Add the parser",
                "2026-10-04T09:00:00Z"
            ),
            pull("alpha", None, 14, "Fix the clock", "2026-10-06T08:00:00Z"),
            pull("beta", None, 3, "Document the API", "2026-10-01T13:30:00Z"),
        ],
        "{}",
        describe(&output)
    );
}

/// Each refusal exits 1 with one line on standard error and nothing on standard output: no store
/// (and none is created), no complete snapshot, no `--since`, an instant that is not RFC 3339, and
/// a format no view offers.
#[test]
fn shipped_refuses_what_it_cannot_answer() {
    let root = case_dir("refusals");
    let refused = |output: &Output, says: &str, what: &str| {
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what}: {}",
            describe(output)
        );
        let stderr = stderr(output);
        assert_eq!(stderr.lines().count(), 1, "{what}: {}", describe(output));
        assert!(stderr.contains(says), "{what}: {}", describe(output));
        assert!(output.stdout.is_empty(), "{what}: {}", describe(output));
    };

    refused(
        &shipped(&root, NOW, None),
        "no store at",
        "no store under the state directory",
    );
    assert!(
        !root.join("state").exists(),
        "a read created the state directory"
    );

    take_at(
        &root,
        NOW,
        &[records("alpha", "v1.0.0", NOW, 1, Some("down"))],
    );
    refused(
        &shipped(&root, "2026-10-01T00:00:00Z", None),
        "no complete snapshot",
        "a failed snapshot alone",
    );

    take_fixtures(&root);
    refused(
        &conductor(&root, &["repository", "shipped"]),
        "--since is missing",
        "no --since",
    );
    refused(
        &shipped(&root, "yesterday", None),
        "--since \"yesterday\" is not an RFC 3339 instant",
        "--since yesterday",
    );
    refused(
        &shipped(&root, "2026-10-01T00:00:00Z", Some("yaml")),
        "--format \"yaml\" is not one of text, json, jsonl, markdown",
        "--format yaml",
    );
}

// ---------------------------------------------------------------------------------------------
// The record commands and views of the built binary
// ---------------------------------------------------------------------------------------------

/// The rows of `conductor snapshot <view> --format json`, each without its assigned
/// `observation_id`.
fn view_rows(root: &Path, view: &str) -> Vec<Value> {
    let output = conductor(root, &["snapshot", view, "--format", "json"]);
    json_rows(&output)
        .into_iter()
        .map(|mut row| {
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
        })
        .collect()
}

/// A release and a merged pull request recorded through the [`Recorder`] read back, field for
/// field, through `snapshot releases` and `snapshot merged-pull-requests`; the recorder names the
/// snapshot's start. `snapshot record-release` and `snapshot record-merged-pull-request` refuse
/// a snapshot that is not collecting, an id no snapshot carries, a pull request number that is not
/// positive and a missing flag, and record nothing.
#[test]
fn the_record_commands_and_views_answer_through_the_binary() {
    let root = case_dir("binary");
    let started = Collector::new("started", |record: &mut Recorder<'_>| {
        assert_eq!(record.started_at()?, Timestamp(NOW.to_owned()));
        Ok(())
    });
    let taken = take_at(
        &root,
        NOW,
        &[
            started,
            records("alpha", "v0.4.0", "2026-10-06T09:00:00Z", 21, None),
        ],
    );
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = taken.snapshot_id.0.0.as_str();
    let releases = vec![json!({
        "snapshot_id": snapshot,
        "repository": "alpha",
        "tag": "v0.4.0",
        "published_at": "2026-10-06T09:00:00Z",
        "url": "https://github.com/example-org/alpha/releases/tag/v0.4.0",
    })];
    let merged = vec![json!({
        "snapshot_id": snapshot,
        "repository": "alpha",
        "number": 21,
        "title": "Pull request 21",
        "merged_at": "2026-10-06T09:00:00Z",
        "url": "https://github.com/example-org/alpha/pull/21",
    })];
    assert_eq!(view_rows(&root, "releases"), releases);
    assert_eq!(view_rows(&root, "merged-pull-requests"), merged);

    let refused = |output: &Output, error: &str, what: &str| {
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what}: {}",
            describe(output)
        );
        let stderr = stderr(output);
        assert_eq!(stderr.lines().count(), 1, "{what}: {}", describe(output));
        assert!(stderr.contains(error), "{what}: {}", describe(output));
        assert!(output.stdout.is_empty(), "{what}: {}", describe(output));
    };
    let release = |snapshot: &str, tag: Option<&str>| {
        let mut args = vec![
            "snapshot",
            "record-release",
            "--snapshot-id",
            snapshot,
            "--repository",
            "beta",
            "--published-at",
            "2026-10-06T09:00:00Z",
            "--url",
            "https://github.com/example-org/beta/releases/tag/v1",
        ];
        if let Some(tag) = tag {
            args.extend(["--tag", tag]);
        }
        conductor(&root, &args)
    };
    let merged_pull = |snapshot: &str, number: &str| {
        conductor(
            &root,
            &[
                "snapshot",
                "record-merged-pull-request",
                "--snapshot-id",
                snapshot,
                "--repository",
                "beta",
                "--number",
                number,
                "--title",
                "Late",
                "--merged-at",
                "2026-10-06T09:00:00Z",
                "--url",
                "https://github.com/example-org/beta/pull/1",
            ],
        )
    };
    let unknown = "00000000-0000-7000-8000-000000000000";
    refused(
        &release(snapshot, Some("v1")),
        "SnapshotNotCollecting",
        "record-release into the complete snapshot",
    );
    refused(
        &release(unknown, Some("v1")),
        "SnapshotNotFound",
        "record-release into no snapshot",
    );
    refused(
        &release(snapshot, None),
        "--tag is missing",
        "record-release without --tag",
    );
    refused(
        &merged_pull(snapshot, "1"),
        "SnapshotNotCollecting",
        "record-merged-pull-request into the complete snapshot",
    );
    refused(
        &merged_pull(unknown, "1"),
        "SnapshotNotFound",
        "record-merged-pull-request into no snapshot",
    );
    refused(
        &merged_pull(snapshot, "0"),
        "InvalidPullRequestNumber",
        "record-merged-pull-request numbered 0",
    );

    assert_eq!(view_rows(&root, "releases"), releases);
    assert_eq!(view_rows(&root, "merged-pull-requests"), merged);
}
