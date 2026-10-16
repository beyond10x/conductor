//! `story:dashboard-fast`: every dashboard endpoint answers warm in under 200 ms. The server
//! computes an endpoint's answer once per store write, keeps it in memory with an `ETag`, answers
//! a repeat request from memory, and `304 Not Modified` to one whose `If-None-Match` names the
//! answer it holds; a new write to the store refreshes the answer.
//!
//! The cases serve [`dashboard::serve_over`] in this process on a free port of `127.0.0.1` over a
//! fixture store under this test target's temporary directory, as `dashboard_overview.rs` does.
//! Nothing is read from the environment: the home, the records, the watch and the store are the
//! case's, and the session list is `cat` of a file of the case.
//!
//! The last case is a measurement, not a check, and is ignored by default: it seeds a store of an
//! instance's size (kept between runs) and prints, per endpoint, the first answer and the best and
//! worst of the warm ones. Run it in a release build:
//!
//! ```console
//! cargo test --release -p conductor-cli --test dashboard_fast -- --ignored --nocapture
//! ```

mod active;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use conductor_cli::collect::Collector;
use conductor_cli::dashboard::{self, Sources};
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_model::observation::{
    CommitSha, Mergeability, RecordBlocker, RecordMergedPullRequest, RecordPullRequest,
    RecordRelease, RecordRepository, RecordSpecification, RecordWorkflowRun, RepositoryName,
    RunConclusion, SnapshotState, SpecificationPresence, StartSnapshot, ValidationResult,
    Visibility,
};
use conductor_model::primitives::Timestamp;
use serde_json::Value;

/// The endpoints whose answer is the store's alone, so that a repeat request with nothing
/// written in between is `304`.
const STORE_ENDPOINTS: [&str; 12] = [
    "/api/board",
    "/api/repositories",
    "/api/pull-requests",
    "/api/workflow-runs",
    "/api/releases",
    "/api/reviews",
    "/api/events?since=2026-10-13T00%3A00%3A00Z",
    "/api/series",
    "/api/series?series=prs_merged&days=30",
    "/api/kpis",
    "/api/blockers",
    "/api/specifications",
];

/// The endpoints that read a live source beside the store (the session list, the free disk, the
/// clock): each still answers with an `ETag` and from memory where it reads the store.
const LIVE_ENDPOINTS: [&str; 4] = [
    "/data.json",
    "/api/health",
    "/api/waiting-on",
    "/api/sessions?repository=repo-00",
];

/// How long a repeat request may take.
const REPEAT: Duration = Duration::from_millis(50);

/// How long a warm request may take on a store of an instance's size.
const WARM: Duration = Duration::from_millis(200);

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/` with the session list and the records, `state/` with the store.
struct Case {
    dir: PathBuf,
    /// Whether the directory is removed when the case ends; the measurement keeps its store.
    keep: bool,
}

/// A case that passed leaves nothing behind, but the measurement's store; a failed one keeps its
/// directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !self.keep && !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let case = Self::at(name, false);
        if case.dir.exists() {
            fs::remove_dir_all(&case.dir).expect("clear the case's directory");
        }
        case.skeleton();
        case
    }

    fn at(name: &str, keep: bool) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-fast")
            .join(name);
        Self { dir, keep }
    }

    fn skeleton(&self) {
        fs::create_dir_all(self.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(self.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(self.home().join("watch")).expect("create watch/");
        fs::write(self.home().join("agents.json"), "[]").expect("write the session list");
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

    /// The sources over the case's home and store; `cat` stands in for every command.
    fn sources(&self) -> Sources {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
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

    /// One complete snapshot started at `started_at` of `repositories` repositories, each with
    /// `per` pull requests, workflow runs, releases and merges, a blocker and a specification.
    fn complete(&self, started_at: &str, repositories: usize, per: usize) {
        let store = store::open(&self.state()).expect("open the store");
        let start = StartSnapshot {
            started_at: Timestamp(started_at.to_owned()),
            disk_free_bytes: 1_000_000,
            budget: None,
        };
        let taken = snapshot::take_from(store, start, &[fixture(repositories, per)])
            .expect("the driver ends the snapshot");
        assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    }
}

/// A collector of `repositories` repositories `repo-00`, `repo-01`, …, each with `per` open pull
/// requests, `main` workflow runs, releases and merges, one blocker and one specification.
fn fixture(repositories: usize, per: usize) -> Collector {
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let at = |day: usize| Timestamp(format!("2026-10-{:02}T09:00:00Z", 1 + day % 12));
        for index in 0..repositories {
            let repository = RepositoryName(format!("repo-{index:02}"));
            record.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                repository: repository.clone(),
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: CommitSha(format!("head-of-{index}")),
                main_committed_at: at(index),
                main_commits_7d: 2,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: 1,
                oldest_open_issue_at: None,
                latest_release: Some("v1.0.0".to_owned()),
                latest_release_at: Some(at(index)),
                unreleased_commits: Some(0),
                planning_store_version: None,
                in_catalog: None,
                gates_hooks_version: None,
                gates_version: None,
                wave_width: None,
                wave_ready: None,
                wave_reason: None,
            })?;
            for number in 0..per {
                let number = i64::try_from(number + 1).expect("a small number");
                record.pull_request(RecordPullRequest {
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    number,
                    title: format!("Change {number}"),
                    draft: false,
                    mergeability: Mergeability::Mergeable,
                    failing_checks: number % 2,
                    failing_check_names: Vec::new(),
                    opened_at: at(index),
                    updated_at: at(index),
                    url: Some(format!(
                        "https://github.com/example-org/repo-{index:02}/pull/{number}"
                    )),
                })?;
                record.workflow_run(RecordWorkflowRun {
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    workflow: format!("workflow-{number}"),
                    conclusion: RunConclusion::Success,
                    run_at: at(index),
                })?;
                record.release(RecordRelease {
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    tag: format!("v1.0.{number}"),
                    published_at: at(index),
                    url: format!(
                        "https://github.com/example-org/repo-{index:02}/releases/tag/v1.0.{number}"
                    ),
                })?;
                record.merged_pull_request(RecordMergedPullRequest {
                    snapshot_id: snapshot.clone(),
                    repository: repository.clone(),
                    number: 1000 + number,
                    title: format!("Merged {number}"),
                    merged_at: at(index),
                    url: format!(
                        "https://github.com/example-org/repo-{index:02}/pull/{}",
                        1000 + number
                    ),
                })?;
            }
            record.blocker(RecordBlocker {
                snapshot_id: snapshot.clone(),
                repository: repository.clone(),
                reference: format!("blocker:{index}"),
                blocker_kind: "decision".to_owned(),
                title: "Which token".to_owned(),
            })?;
            record.specification(RecordSpecification {
                snapshot_id: snapshot.clone(),
                repository,
                presence: SpecificationPresence::Present,
                path: Some("spec".to_owned()),
                format: Some("ess/22".to_owned()),
                required_ess: None,
                validation: ValidationResult::Valid,
                validation_refusals: 0,
                scenarios: Some(10),
                synthesis_refusals: None,
                conformance_status: None,
            })?;
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------------------------

/// One answer: its status, its head and its body, and how long it took.
struct Answer {
    status: u16,
    head: String,
    body: String,
    took: Duration,
}

impl Answer {
    /// The value of the header `name`, when the answer carries it.
    fn header(&self, name: &str) -> Option<&str> {
        self.head
            .split("\r\n")
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    }

    fn etag(&self) -> String {
        self.header("ETag")
            .unwrap_or_else(|| panic!("no ETag in {:?}", self.head))
            .to_owned()
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|error| panic!("{error}: {}", self.body))
    }
}

/// `GET path`, with `If-None-Match: <etag>` when one is given, read to the end.
fn get(port: u16, path: &str, etag: Option<&str>) -> Answer {
    let asked = Instant::now();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(120)))
        .expect("set a read timeout");
    let condition = etag.map_or_else(String::new, |etag| format!("If-None-Match: {etag}\r\n"));
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{condition}Connection: close\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let took = asked.elapsed();
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"));
    Answer {
        status,
        head: head.to_owned(),
        body: body.to_owned(),
        took,
    }
}

/// `GET path` with `If-None-Match: <etag>` until the answer is no longer the stale one the client
/// holds: after a write the kept answer is served stale, `304` with `X-Conductor-Age`, while its
/// recompute runs, which starts at most once per debounce (`cache::DEBOUNCE`); up to a minute.
fn revalidated(port: u16, path: &str, etag: &str) -> Answer {
    let since = Instant::now();
    loop {
        let answer = get(port, path, Some(etag));
        if answer.status != 304 {
            return answer;
        }
        assert!(
            answer.header("X-Conductor-Age").is_some(),
            "{path}: a 304 after the write is not marked stale: {:?}",
            answer.head
        );
        assert!(
            since.elapsed() < Duration::from_secs(60),
            "{path} still answers the old snapshot after a minute"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The median of `times`.
fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// Each endpoint answers with an `ETag`; asked again with that `ETag` and nothing written in
/// between, an endpoint of the store answers `304` with no body, and every endpoint answers a
/// repeat request in under 50 ms.
#[test]
fn a_repeat_request_is_answered_from_memory_and_304_when_unchanged() {
    let case = Case::new("etag");
    case.complete("2026-10-13T06:00:00Z", 3, 2);
    let port = case.start();
    for path in STORE_ENDPOINTS {
        let first = get(port, path, None);
        assert_eq!(first.status, 200, "{path}: {}", first.body);
        let etag = first.etag();
        let mut repeats = Vec::new();
        for _ in 0..5 {
            let again = get(port, path, Some(&etag));
            assert_eq!(again.status, 304, "{path} changed with nothing written");
            assert_eq!(again.body, "", "{path}: a 304 carries no body");
            assert_eq!(again.etag(), etag, "{path}");
            repeats.push(again.took);
        }
        let repeat = median(repeats);
        assert!(repeat < REPEAT, "{path}: a repeat request took {repeat:?}");
        let unconditional = get(port, path, None);
        assert_eq!(unconditional.status, 200, "{path}");
        assert_eq!(unconditional.etag(), etag, "{path}");
        assert_eq!(unconditional.body, first.body, "{path}");
    }
    for path in LIVE_ENDPOINTS {
        let first = get(port, path, None);
        assert_eq!(first.status, 200, "{path}: {}", first.body);
        let etag = first.etag();
        let repeats: Vec<Duration> = (0..5).map(|_| get(port, path, None).took).collect();
        let repeat = median(repeats);
        assert!(repeat < REPEAT, "{path}: a repeat request took {repeat:?}");
        let again = get(port, path, Some(&etag));
        assert!(
            matches!(again.status, 200 | 304),
            "{path}: {}",
            again.status
        );
    }
}

/// A new snapshot written to the store refreshes every snapshot endpoint's answer: the `ETag` the
/// client holds is answered `200` with the new rows and a new `ETag`.
#[test]
fn a_new_write_refreshes_the_cached_answer() {
    let case = Case::new("refresh");
    case.complete("2026-10-13T06:00:00Z", 2, 1);
    let port = case.start();
    let paths = ["/api/board", "/api/repositories", "/api/blockers"];
    let held: Vec<(String, Answer)> = paths
        .iter()
        .map(|path| ((*path).to_owned(), get(port, path, None)))
        .collect();
    for (path, answer) in &held {
        assert_eq!(answer.status, 200, "{path}");
        assert!(!answer.body.contains("repo-02"), "{path}: {}", answer.body);
        assert_eq!(get(port, path, Some(&answer.etag())).status, 304, "{path}");
    }

    case.complete("2026-10-13T08:00:00Z", 3, 1);
    for (path, before) in &held {
        let after = revalidated(port, path, &before.etag());
        assert_eq!(after.status, 200, "{path} still answers the old snapshot");
        assert_ne!(after.etag(), before.etag(), "{path}");
        let rows = after.json()["rows"].to_string();
        assert!(rows.contains("repo-02"), "{path}: {rows}");
        assert_eq!(get(port, path, Some(&after.etag())).status, 304, "{path}");
    }
}

/// The measurement: a store of an instance's size, 12 kept snapshots of 40 repositories each with
/// 6 pull requests, workflow runs, releases and merges; every endpoint's first answer, and the best
/// and worst of 10 warm ones, which must be under 200 ms.
#[test]
#[ignore = "a measurement: run it in a release build with --ignored --nocapture"]
fn every_endpoint_answers_warm_in_under_200_ms_on_a_store_of_an_instance_s_size() {
    const SNAPSHOTS: usize = 12;
    let case = Case::at("measure", true);
    let seeded = case.dir.join("seeded");
    if !seeded.exists() {
        if case.dir.exists() {
            fs::remove_dir_all(&case.dir).expect("clear a partial seed");
        }
        case.skeleton();
        let seeding = Instant::now();
        for snapshot in 0..SNAPSHOTS {
            case.complete(&format!("2026-10-13T{snapshot:02}:00:00Z"), 40, 6);
        }
        fs::write(&seeded, "").expect("mark the store seeded");
        println!("seeded {SNAPSHOTS} snapshots in {:?}", seeding.elapsed());
    }
    let port = case.start();
    let mut slow = Vec::new();
    println!(
        "{:<45} {:>10} {:>10} {:>10}",
        "endpoint", "first", "best", "worst"
    );
    for path in STORE_ENDPOINTS.iter().chain(&LIVE_ENDPOINTS) {
        let first = get(port, path, None);
        assert_eq!(first.status, 200, "{path}: {}", first.body);
        let warm: Vec<Duration> = (0..10).map(|_| get(port, path, None).took).collect();
        let (best, worst) = (
            *warm.iter().min().expect("ten times"),
            *warm.iter().max().expect("ten times"),
        );
        println!(
            "{path:<45} {:>10.1?} {best:>10.1?} {worst:>10.1?}",
            first.took
        );
        if worst >= WARM {
            slow.push(format!("{path}: {worst:?}"));
        }
    }
    assert!(slow.is_empty(), "warm answers over {WARM:?}: {slow:?}");
}
