//! `story:dashboard-overview`: what the overview reads, served by [`dashboard::serve_over`] in
//! this process on a free port of `127.0.0.1` over a fixture store and fixture records: the
//! newest complete snapshot's board, repositories, pull requests, workflow runs and releases; the
//! day's events; and the session's last report in `/data.json`. The last case holds the keys of the
//! overview's fixtures (`crates/conductor/dashboard/src/views/__tests__/fixtures/`) to what the
//! endpoints answer, so the app's tests cannot drift from the served shape.
//!
//! Every case keeps its files under its own directory in this test target's temporary directory:
//! `home/` (the session list, the records under `home/example-org/conductor`) and `state/` (the
//! store). Snapshots are taken through the library ([`snapshot::take_from`]) at start times the
//! case gives, with one fake collector. Nothing is read from the environment; "today" is pinned
//! with `since`, in UTC, so no case depends on the runner's clock or time zone.

mod active;
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use conductor_cli::collect::Collector;
use conductor_cli::dashboard::{self, Sources, overview};
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_model::observation::{
    CommitSha, Mergeability, RecordBlocker, RecordMergedPullRequest, RecordPullRequest,
    RecordRelease, RecordRepository, RecordWorkflowRun, RepositoryName, RunConclusion,
    SnapshotState, StartSnapshot, Visibility,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// When the case's first snapshot starts.
const EARLIER: &str = "2026-10-13T06:00:00Z";

/// When a later snapshot of a case starts.
const LATER: &str = "2026-10-13T08:00:00Z";

/// The `since` the event cases ask with: 00:00 UTC of the fixture's day.
const SINCE: &str = "2026-10-13T00:00:00Z";

/// One second after [`SINCE`], and one second before it.
const AFTER: &str = "2026-10-13T00:00:01Z";
const BEFORE: &str = "2026-10-12T23:59:59Z";

/// The six endpoints the overview reads.
const ENDPOINTS: [&str; 6] = [
    "/api/board",
    "/api/repositories",
    "/api/pull-requests",
    "/api/workflow-runs",
    "/api/releases",
    "/api/events",
];

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/` with the session list and the records, `state/` with the store.
struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-overview")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.dir.join("work")).expect("create work/");
        case.write("home/agents.json", "[]");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.dir.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(path, text).expect("write a case file");
    }

    /// The sources over the case's home: the session list `home/agents.json`, the records under
    /// `home/example-org/conductor`, the checkouts root `home/example-org`, and `cat /dev/null`
    /// for the two `worktree` dry-runs.
    fn sources(&self) -> Sources {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join(".local/state/worktree/trees/example-org");
        sources.targets = home.join(".cache/target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources
    }

    /// Serves the case's sources over its store in this process; the port.
    fn start(&self) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || dashboard::serve_over(listener, sources, Some(state)));
        port
    }

    /// Takes one snapshot started at `started_at` through `collector` into the case's store.
    fn take(&self, started_at: &str, collector: Collector) -> (String, SnapshotState) {
        let store = store::open(&self.state()).expect("open the store");
        let start = StartSnapshot {
            started_at: Timestamp(started_at.to_owned()),
            disk_free_bytes: 1_000_000,
        };
        let taken =
            snapshot::take_from(store, start, &[collector]).expect("the driver ends the snapshot");
        (taken.snapshot_id.0.0, taken.state)
    }

    /// One complete snapshot of [`fixture`] started at `started_at`; its id.
    fn complete(&self, started_at: &str) -> String {
        let (id, state) = self.take(started_at, fixture());
        assert_eq!(state, SnapshotState::Complete);
        id
    }
}

/// The fixture snapshot: `alpha` (a checkout; a red `main` workflow and a green one; one open pull
/// request with a failing check and its `url`; one open blocker; a release and a merge just after
/// [`SINCE`] and one of each just before it), `beta` (no checkout, nothing open) and the archived
/// `gamma`.
fn fixture() -> Collector {
    Collector::new("fixture", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let name = |name: &str| RepositoryName(name.to_owned());
        for (repository, archived, local_checkout) in [
            ("alpha", false, true),
            ("beta", false, false),
            ("gamma", true, false),
        ] {
            record.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                visibility: Visibility::Public,
                archived,
                local_checkout,
                main_head: CommitSha(format!("head-of-{repository}")),
                main_committed_at: Timestamp("2026-10-12T12:00:00Z".to_owned()),
                main_commits_7d: 2,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: 1,
                oldest_open_issue_at: None,
                latest_release: Some("v1.1.0".to_owned()),
                latest_release_at: Some(Timestamp(AFTER.to_owned())),
                unreleased_commits: Some(0),
                planning_store_version: None,
                in_catalog: None,
            })?;
        }
        record.pull_request(RecordPullRequest {
            snapshot_id: snapshot.clone(),
            repository: name("alpha"),
            number: 7,
            title: "Link the board".to_owned(),
            draft: false,
            mergeability: Mergeability::Mergeable,
            failing_checks: 1,
            opened_at: Timestamp("2026-10-12T09:00:00Z".to_owned()),
            updated_at: Timestamp("2026-10-12T10:00:00Z".to_owned()),
            url: Some("https://github.com/example-org/alpha/pull/7".to_owned()),
        })?;
        for (workflow, conclusion) in [
            ("CI", RunConclusion::Failure),
            ("Docs", RunConclusion::Success),
        ] {
            record.workflow_run(RecordWorkflowRun {
                snapshot_id: snapshot.clone(),
                repository: name("alpha"),
                workflow: workflow.to_owned(),
                conclusion,
                run_at: Timestamp("2026-10-12T11:00:00Z".to_owned()),
            })?;
        }
        record.blocker(RecordBlocker {
            snapshot_id: snapshot.clone(),
            repository: name("alpha"),
            reference: "blocker:token".to_owned(),
            blocker_kind: "decision".to_owned(),
            title: "Which token".to_owned(),
        })?;
        for (tag, at) in [("v1.1.0", AFTER), ("v1.0.9", BEFORE)] {
            record.release(RecordRelease {
                snapshot_id: snapshot.clone(),
                repository: name("alpha"),
                tag: tag.to_owned(),
                published_at: Timestamp(at.to_owned()),
                url: format!("https://github.com/example-org/alpha/releases/tag/{tag}"),
            })?;
        }
        for (number, at) in [(5, AFTER), (4, BEFORE)] {
            record.merged_pull_request(RecordMergedPullRequest {
                snapshot_id: snapshot.clone(),
                repository: name("alpha"),
                number,
                title: format!("Merge {number}"),
                merged_at: Timestamp(at.to_owned()),
                url: format!("https://github.com/example-org/alpha/pull/{number}"),
            })?;
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------------------------

/// One request, read to the end: its status and body.
fn request(port: u16, method: &str, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Content-Length: 0\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"));
    (status, body.to_owned())
}

/// `GET path` answered `200` with JSON; that JSON.
fn get(port: u16, path: &str) -> Value {
    let (status, body) = request(port, "GET", path);
    assert_eq!(status, 200, "GET {path}: {body}");
    serde_json::from_str(&body).unwrap_or_else(|error| panic!("GET {path}: {error}: {body}"))
}

/// `/api/events` since [`SINCE`].
fn events(port: u16) -> Vec<Value> {
    let answer = get(port, "/api/events?since=2026-10-13T00%3A00%3A00Z");
    answer["rows"].as_array().expect("rows").clone()
}

fn rows(answer: &Value) -> &Vec<Value> {
    answer["rows"].as_array().expect("a list of rows")
}

/// The rows of a view's `answer` without the identities the store assigns, each of which must
/// carry an `observation_id` and the snapshot's id `snapshot`.
fn lean(answer: &Value, snapshot: &str) -> Vec<Value> {
    rows(answer)
        .iter()
        .map(|row| {
            let mut row = row.clone();
            let fields = row.as_object_mut().expect("a row is an object");
            assert!(
                fields
                    .remove("observation_id")
                    .is_some_and(|id| id.is_string()),
                "{answer:#}"
            );
            assert_eq!(
                fields.remove("snapshot_id"),
                Some(json!(snapshot)),
                "{answer:#}"
            );
            row
        })
        .collect()
}

fn column<'a>(answer: &'a Value, key: &str) -> Vec<&'a Value> {
    rows(answer).iter().map(|row| &row[key]).collect()
}

// ---------------------------------------------------------------------------------------------
// The snapshot endpoints
// ---------------------------------------------------------------------------------------------

/// Each snapshot endpoint answers `{snapshot: {id, started_at}, rows}` of the newest complete
/// snapshot, with the fixture's rows.
#[test]
fn each_snapshot_endpoint_answers_the_fixture_snapshot_s_rows() {
    let case = Case::new("rows");
    let id = case.complete(EARLIER);
    let port = case.start();
    let header = json!({"id": id, "started_at": EARLIER});

    let board = get(port, "/api/board");
    assert_eq!(board["snapshot"], header, "{board:#}");
    assert_eq!(
        rows(&board),
        &vec![
            json!({"repository": "alpha", "deviations": ["red main", "failing PR"],
                   "open_prs": 1, "failing_prs": 1, "non_green_workflows": 1,
                   "open_blockers": 1, "specifications_present": 0}),
            json!({"repository": "beta", "deviations": ["no checkout"],
                   "open_prs": 0, "failing_prs": 0, "non_green_workflows": 0,
                   "open_blockers": 0, "specifications_present": 0}),
        ],
        "one row per repository that is not archived: {board:#}"
    );

    let repositories = get(port, "/api/repositories");
    assert_eq!(repositories["snapshot"], header);
    assert_eq!(
        column(&repositories, "repository"),
        [&json!("alpha"), &json!("beta"), &json!("gamma")]
    );
    let repositories = lean(&repositories, &id);
    assert_eq!(repositories[2]["archived"], json!(true));
    assert_eq!(repositories[0]["latest_release"], json!("v1.1.0"));

    let pulls = get(port, "/api/pull-requests");
    assert_eq!(pulls["snapshot"], header);
    assert_eq!(
        lean(&pulls, &id),
        vec![json!({
            "repository": "alpha", "number": 7, "title": "Link the board", "draft": false,
            "mergeability": "Mergeable", "failing_checks": 1,
            "opened_at": "2026-10-12T09:00:00Z", "updated_at": "2026-10-12T10:00:00Z",
            "url": "https://github.com/example-org/alpha/pull/7",
        })]
    );

    let runs = get(port, "/api/workflow-runs");
    assert_eq!(runs["snapshot"], header);
    assert_eq!(
        lean(&runs, &id),
        vec![
            json!({"repository": "alpha", "workflow": "CI", "conclusion": "Failure",
                   "run_at": "2026-10-12T11:00:00Z"}),
            json!({"repository": "alpha", "workflow": "Docs", "conclusion": "Success",
                   "run_at": "2026-10-12T11:00:00Z"}),
        ]
    );

    let releases = get(port, "/api/releases");
    assert_eq!(releases["snapshot"], header);
    assert_eq!(
        lean(&releases, &id),
        vec![
            json!({"repository": "alpha", "tag": "v1.0.9", "published_at": BEFORE,
                   "url": "https://github.com/example-org/alpha/releases/tag/v1.0.9"}),
            json!({"repository": "alpha", "tag": "v1.1.0", "published_at": AFTER,
                   "url": "https://github.com/example-org/alpha/releases/tag/v1.1.0"}),
        ]
    );
}

/// A collector that records the repository `delta`, asks every snapshot endpoint while its
/// snapshot is still collecting, keeps the answers in `seen`, and then fails.
fn asks_while_collecting(port: u16, seen: Arc<Mutex<Vec<(String, Value)>>>) -> Collector {
    Collector::new("asks", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        record.repository(RecordRepository {
            snapshot_id: snapshot,
            repository: RepositoryName("delta".to_owned()),
            visibility: Visibility::Public,
            archived: false,
            local_checkout: true,
            main_head: CommitSha("head-of-delta".to_owned()),
            main_committed_at: Timestamp("2026-10-12T12:00:00Z".to_owned()),
            main_commits_7d: 0,
            real_commits_7d: 0,
            behind_main: 0,
            dirty_files: 0,
            worktrees: 0,
            open_issues: 0,
            oldest_open_issue_at: None,
            latest_release: None,
            latest_release_at: None,
            unreleased_commits: None,
            planning_store_version: None,
            in_catalog: None,
        })?;
        let mut seen = seen.lock().expect("the answers");
        for path in &ENDPOINTS[..5] {
            seen.push(((*path).to_owned(), get(port, path)));
        }
        anyhow::bail!("fail after the endpoints were asked")
    })
}

/// With a newer snapshot still collecting, and once it failed, every endpoint answers the older
/// complete one.
#[test]
fn a_newer_snapshot_collecting_or_failed_is_not_read() {
    let case = Case::new("newer");
    let id = case.complete(EARLIER);
    let port = case.start();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (_, state) = case.take(LATER, asks_while_collecting(port, Arc::clone(&seen)));
    assert_eq!(state, SnapshotState::Failed);

    let collecting = seen.lock().expect("the answers").clone();
    assert_eq!(
        collecting.len(),
        5,
        "every endpoint was asked while collecting"
    );
    let failed: Vec<(String, Value)> = ENDPOINTS[..5]
        .iter()
        .map(|path| ((*path).to_owned(), get(port, path)))
        .collect();
    for (path, answer) in collecting.iter().chain(&failed) {
        assert_eq!(answer["snapshot"]["id"], json!(id), "{path}: {answer:#}");
        assert_eq!(answer["snapshot"]["started_at"], json!(EARLIER), "{path}");
        assert!(
            !answer.to_string().contains("delta"),
            "{path} reads the newer snapshot: {answer:#}"
        );
        assert!(!rows(answer).is_empty(), "{path}: {answer:#}");
    }
}

/// Without a store, and over a store with no snapshot, every snapshot endpoint answers no
/// snapshot and no rows, and `/api/events` no rows.
#[test]
fn an_empty_store_answers_no_snapshot_and_no_rows() {
    let case = Case::new("empty");
    for opened in [false, true] {
        if opened {
            drop(store::open(&case.state()).expect("create an empty store"));
        }
        let port = case.start();
        for path in &ENDPOINTS[..5] {
            assert_eq!(
                get(port, path),
                json!({"snapshot": null, "rows": []}),
                "{path}, store created: {opened}"
            );
        }
        assert_eq!(
            get(port, "/api/events?since=2026-10-13T00:00:00Z"),
            json!({"rows": []})
        );
    }
}

/// A POST to each endpoint is 405 naming GET.
#[test]
fn a_post_to_each_endpoint_is_405() {
    let case = Case::new("post");
    case.complete(EARLIER);
    let port = case.start();
    for path in ENDPOINTS {
        let (status, body) = request(port, "POST", path);
        assert_eq!(status, 405, "POST {path}: {body}");
    }
}

/// A `since` that is not RFC 3339 is answered 400.
#[test]
fn a_since_that_does_not_read_is_400() {
    let case = Case::new("since");
    let port = case.start();
    let (status, body) = request(port, "GET", "/api/events?since=yesterday");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("since=yesterday"), "{body}");
}

/// "Today" is 00:00 UTC of the current day in UTC, whatever offset the clock reads in.
#[test]
fn today_starts_at_00_00_utc() {
    let at = |text: &str| OffsetDateTime::parse(text, &Rfc3339).expect("RFC 3339");
    let start = overview::start_of_day(at("2026-10-13T01:30:00+02:00"));
    assert_eq!(start, at("2026-10-12T00:00:00Z"));
    assert_eq!(start.offset(), UtcOffset::UTC);
    assert_eq!(
        overview::start_of_day(at("2026-10-13T23:59:59Z")),
        at(SINCE)
    );
}

// ---------------------------------------------------------------------------------------------
// One computation of the figures
// ---------------------------------------------------------------------------------------------

/// The `| repository | deviations |` rows of the board's `## Deviations` section, by repository.
fn deviations_of(board: &str) -> BTreeMap<String, Vec<String>> {
    let section = board
        .split("## Deviations\n")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("a Deviations section");
    section
        .lines()
        .filter(|line| {
            line.starts_with("| ")
                && !line.starts_with("| repository")
                && !line.starts_with("| ---")
        })
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
            (
                cells[0].to_owned(),
                cells[1].split(", ").map(str::to_owned).collect(),
            )
        })
        .collect()
}

/// For one fixture snapshot, `/api/board`'s deviations per repository are those of the
/// `STATUS.md` that `publish-board` writes.
#[test]
fn the_board_endpoint_s_deviations_are_those_status_md_holds() {
    let case = Case::new("board");
    let id = case.complete(EARLIER);
    let path = case.dir.join("out/STATUS.md");
    let output = common::conductor(case.home())
        .current_dir(case.dir.join("work"))
        .arg("--state-dir")
        .arg(case.state())
        .args([
            "snapshot",
            "publish-board",
            "--snapshot-id",
            id.as_str(),
            "--path",
        ])
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let written = deviations_of(&fs::read_to_string(&path).expect("read STATUS.md"));
    assert!(!written.is_empty(), "the fixture has deviations");

    let port = case.start();
    let served: BTreeMap<String, Vec<String>> = rows(&get(port, "/api/board"))
        .iter()
        .filter(|row| !row["deviations"].as_array().expect("a list").is_empty())
        .map(|row| {
            (
                row["repository"].as_str().expect("a name").to_owned(),
                row["deviations"]
                    .as_array()
                    .expect("a list")
                    .iter()
                    .map(|deviation| deviation.as_str().expect("text").to_owned())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(served, written);
}

// ---------------------------------------------------------------------------------------------
// The session's last report
// ---------------------------------------------------------------------------------------------

/// "Last event" is the session's newest report: a dispatch whose state line, written last, is of
/// days ago, and whose report is of minutes ago, reads the report in the session's row.
#[test]
fn the_last_event_is_the_session_s_newest_report_not_its_dispatch_s_state() {
    let case = Case::new("last-report");
    let now = OffsetDateTime::now_utc();
    let ago = |seconds: i64| {
        (now - time::Duration::seconds(seconds))
            .format(&Rfc3339)
            .expect("format an instant")
    };
    let millis = |at: OffsetDateTime| {
        i64::try_from(at.unix_timestamp_nanos() / 1_000_000).expect("milliseconds fit")
    };
    let cwd = case.home().join("example-org/alpha");
    case.write(
        "home/agents.json",
        &json!([{"name": "alpha", "kind": "background", "state": "working", "status": "busy",
                 "startedAt": millis(now - time::Duration::hours(1)),
                 "cwd": cwd.display().to_string(), "sessionId": "s1", "pid": 1001}])
        .to_string(),
    );
    let lines = [
        json!({"id": "DSP-1", "to": "alpha", "repository": "alpha", "goal": "G1",
               "brief": "Link the board", "at": ago(4 * 86_400)}),
        json!({"id": "DSP-1", "event": "progress", "reported_by": "alpha", "at": ago(600)}),
        json!({"id": "DSP-1", "event": "started", "reported_by": "alpha",
               "at": ago(3 * 86_400)}),
    ];
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    case.write("home/example-org/conductor/dispatches/2026-10.jsonl", &text);

    let port = case.start();
    let data = get(port, "/data.json");
    let alpha = data["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|row| row["name"] == "alpha")
        .unwrap_or_else(|| panic!("no alpha: {data:#}"));
    assert_eq!(alpha["dispatch"]["event"], "progress", "{alpha:#}");
    assert_eq!(alpha["dispatch"]["event_ago"], "10m", "{alpha:#}");
}

// ---------------------------------------------------------------------------------------------
// The day's events
// ---------------------------------------------------------------------------------------------

/// The fixture records: for each kind from the records, one line just after [`SINCE`] and one just
/// before it, besides the snapshot's release and merge.
fn event_records(case: &Case) {
    let dispatches = [
        json!({"id": "DSP-A", "to": "alpha", "goal": "goal after", "brief": "b", "at": AFTER}),
        json!({"id": "DSP-B", "to": "beta", "goal": "goal before", "brief": "b", "at": BEFORE}),
        json!({"id": "DSP-B", "event": "blocked", "reported_by": "beta", "at": BEFORE}),
        json!({"id": "DSP-A", "event": "started", "reported_by": "alpha", "at": AFTER}),
    ];
    let decisions = [
        json!({"id": "DEC-A", "class": "C", "question": "Which first?", "choice": "A",
               "repository": "alpha", "decided_at": AFTER}),
        json!({"id": "DEC-B", "class": "C", "question": "Which then?", "choice": "B",
               "repository": "beta", "decided_at": BEFORE}),
    ];
    let lines =
        |lines: &[Value]| -> String { lines.iter().map(|line| format!("{line}\n")).collect() };
    case.write(
        "home/example-org/conductor/dispatches/2026-10.jsonl",
        &lines(&dispatches[..]),
    );
    case.write(
        "home/example-org/conductor/decisions/2026-10.jsonl",
        &lines(&decisions[..]),
    );
}

/// The rows of `/api/events?since=SINCE` of `kind`, over the fixture records and snapshot.
fn events_of(name: &str, kind: &str) -> Vec<Value> {
    let case = Case::new(name);
    event_records(&case);
    case.complete(EARLIER);
    let port = case.start();
    let rows = events(port);
    let instants: Vec<OffsetDateTime> = rows
        .iter()
        .map(|row| {
            OffsetDateTime::parse(row["at"].as_str().expect("an at"), &Rfc3339).expect("RFC 3339")
        })
        .collect();
    assert!(
        instants.windows(2).all(|pair| pair[0] >= pair[1]),
        "newest first: {rows:#?}"
    );
    rows.into_iter().filter(|row| row["kind"] == kind).collect()
}

#[test]
fn a_dispatch_after_since_is_an_event_and_one_before_is_not() {
    assert_eq!(
        events_of("events-dispatch", "dispatch"),
        [
            json!({"at": AFTER, "repository": "alpha", "kind": "dispatch", "text": "goal after",
                "link": null})
        ]
    );
}

#[test]
fn a_report_after_since_is_an_event_and_one_before_is_not() {
    assert_eq!(
        events_of("events-report", "report"),
        [
            json!({"at": AFTER, "repository": "alpha", "kind": "report", "text": "started",
                "link": null})
        ]
    );
}

#[test]
fn a_decision_after_since_is_an_event_and_one_before_is_not() {
    assert_eq!(
        events_of("events-decision", "decision"),
        [
            json!({"at": AFTER, "repository": "alpha", "kind": "decision", "text": "C: A",
                "link": null})
        ]
    );
}

#[test]
fn a_release_after_since_is_an_event_and_one_before_is_not() {
    assert_eq!(
        events_of("events-release", "release"),
        [
            json!({"at": AFTER, "repository": "alpha", "kind": "release", "text": "v1.1.0",
                "link": "https://github.com/example-org/alpha/releases/tag/v1.1.0"})
        ]
    );
}

#[test]
fn a_merge_after_since_is_an_event_and_one_before_is_not() {
    assert_eq!(
        events_of("events-merge", "merge"),
        [
            json!({"at": AFTER, "repository": "alpha", "kind": "merge", "text": "#5 Merge 5",
                "link": "https://github.com/example-org/alpha/pull/5"})
        ]
    );
}

/// An open request is an event at the instant it was raised, with its class and question.
#[test]
fn an_open_request_is_an_event_when_it_was_raised() {
    let case = Case::new("events-open");
    case.write(
        "home/example-org/conductor/decisions/2026-10.jsonl",
        &format!(
            "{}\n",
            json!({"id": "DEC-O", "class": "O", "question": "Release now?", "repository": null,
                   "raised_at": AFTER})
        ),
    );
    let port = case.start();
    assert_eq!(
        events(port),
        [
            json!({"at": AFTER, "repository": null, "kind": "decision", "text": "O: Release now?",
                "link": null})
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// The app's fixtures
// ---------------------------------------------------------------------------------------------

/// The overview's fixture `name`, under `crates/conductor/dashboard/src/views/__tests__/fixtures/`.
fn app_fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("dashboard/src/views/__tests__/fixtures")
        .join(name);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

/// The keys of every row of an answer, together.
fn row_keys(answer: &Value) -> BTreeSet<String> {
    answer["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(keys)
        .collect()
}

/// Each fixture of the overview has the keys its endpoint answers: the answer's, the snapshot
/// header's and its rows'.
#[test]
fn the_overview_fixtures_have_the_keys_the_endpoints_answer() {
    let case = Case::new("fixture-keys");
    event_records(&case);
    case.complete(EARLIER);
    let port = case.start();
    for (path, file) in [
        ("/api/board", "overview-board.json"),
        ("/api/repositories", "overview-repositories.json"),
        ("/api/pull-requests", "overview-pull-requests.json"),
        ("/api/workflow-runs", "overview-workflow-runs.json"),
        ("/api/releases", "overview-releases.json"),
        (
            "/api/events?since=2026-10-13T00:00:00Z",
            "overview-events.json",
        ),
    ] {
        let served = get(port, path);
        let fixture = app_fixture(file);
        assert!(!row_keys(&served).is_empty(), "{path} answers rows");
        assert_eq!(keys(&fixture), keys(&served), "{file}: the answer's keys");
        assert_eq!(
            keys(&fixture["snapshot"]),
            keys(&served["snapshot"]),
            "{file}: the snapshot's keys"
        );
        assert_eq!(
            row_keys(&fixture),
            row_keys(&served),
            "{file}: the rows' keys"
        );
    }
    let kinds: BTreeSet<String> = app_fixture("overview-events.json")["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["kind"].as_str().expect("a kind").to_owned())
        .collect();
    assert_eq!(
        kinds,
        overview::KINDS
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect::<BTreeSet<String>>(),
        "the events fixture holds one event of each kind"
    );
}
