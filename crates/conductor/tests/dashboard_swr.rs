//! The dashboard answers at once while the store is written: an answer whose store moved is served
//! stale, with its `ETag` and `X-Conductor-Age`, while one recompute per endpoint runs in the
//! background; a recompute starts at most once per endpoint per debounce; a request before an
//! endpoint's first answer is `503` with `Retry-After`; and the server's store handle reads without
//! waiting on the store's writer lock.
//!
//! The cases serve [`dashboard::serve_tuned`] in this process on a free port of `127.0.0.1` over a
//! fixture store under this test target's temporary directory, as `dashboard_fast.rs` does, with a
//! [`Probe`] that delays every computation by a set time and counts the computations started per
//! endpoint.

mod active;

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use conductor_cli::collect::Collector;
use conductor_cli::dashboard::{self, Probe, Sources, Tuning};
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotState, StartSnapshot, Visibility,
};
use conductor_model::primitives::Timestamp;

/// How long a stale answer may take.
const STALE: Duration = Duration::from_millis(100);

/// The delay of a slow computation.
const SLOW: Duration = Duration::from_secs(3);

/// Longer than the fingerprint's settling window (`SETTLED` in `src/dashboard/cache.rs`): a store
/// written this long ago reads with one fingerprint until it is written again.
const SETTLE: Duration = Duration::from_millis(2_500);

/// The bound on anything a case waits for.
const PATIENCE: Duration = Duration::from_secs(60);

const BOARD: &str = "/api/board";

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

struct Case {
    dir: PathBuf,
}

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
            .join("dashboard-swr")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
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

    /// Serves the case with `tuning`; the port.
    fn start(&self, tuning: Tuning) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || dashboard::serve_tuned(listener, sources, Some(state), tuning));
        port
    }

    /// One complete snapshot of `repositories` repositories `repo-00`, `repo-01`, ….
    fn complete(&self, started_at: &str, repositories: usize) {
        complete(&self.state(), started_at, repositories);
    }

    /// A store of two repositories, settled, served with `tuning`; the port.
    fn served(&self, tuning: Tuning) -> u16 {
        self.complete("2026-10-13T06:00:00Z", 2);
        std::thread::sleep(SETTLE);
        self.start(tuning)
    }

    /// A third repository written to the store.
    fn write(&self) {
        self.complete("2026-10-13T08:00:00Z", 3);
    }
}

/// One complete snapshot of `repositories` repositories written to the store under `state`, by a
/// writer of its own: every observation and the snapshot's end are appends to the store.
fn complete(state: &Path, started_at: &str, repositories: usize) {
    let store = store::open(state).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let taken = snapshot::take_from(store, start, &[fixture(repositories)])
        .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
}

fn fixture(repositories: usize) -> Collector {
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        for index in 0..repositories {
            record.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(format!("repo-{index:02}")),
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: CommitSha(format!("head-of-{index}")),
                main_committed_at: Timestamp("2026-10-12T09:00:00Z".to_owned()),
                main_commits_7d: 2,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: 1,
                oldest_open_issue_at: None,
                latest_release: None,
                latest_release_at: None,
                unreleased_commits: None,
                planning_store_version: None,
                in_catalog: None,
                gates_hooks_version: None,
                gates_version: None,
                wave_width: None,
                wave_ready: None,
                wave_reason: None,
            })?;
        }
        Ok(())
    })
}

/// A tuning with `probe`: no debounce, a cold request waits up to [`PATIENCE`].
fn tuning(probe: &Arc<Probe>) -> Tuning {
    Tuning {
        debounce: Duration::ZERO,
        cold_wait: PATIENCE,
        probe: Some(Arc::clone(probe)),
    }
}

// ---------------------------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------------------------

struct Answer {
    status: u16,
    head: String,
    body: String,
    took: Duration,
}

impl Answer {
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

    fn age(&self) -> Option<u64> {
        self.header("X-Conductor-Age")
            .map(|age| age.parse().unwrap_or_else(|_| panic!("age {age:?}")))
    }
}

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

/// Asks `path` until `done` holds of an answer, up to [`PATIENCE`]; that answer.
fn until(port: u16, path: &str, done: impl Fn(&Answer) -> bool) -> Answer {
    let since = Instant::now();
    loop {
        let answer = get(port, path, None);
        if done(&answer) {
            return answer;
        }
        assert!(
            since.elapsed() < PATIENCE,
            "{path}: still {} {:?} after {PATIENCE:?}",
            answer.status,
            answer.head
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn fresh_with_third(answer: &Answer) -> bool {
    answer.status == 200 && answer.age().is_none() && answer.body.contains("repo-02")
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// After a write, while a slow recompute runs, the kept answer is served in under 100 ms with its
/// `ETag` and its age; the new answer is served once the recompute ends, without an age.
#[test]
fn a_stale_answer_is_served_at_once_while_the_recompute_runs() {
    let case = Case::new("stale");
    let probe = Arc::new(Probe::default());
    let port = case.served(tuning(&probe));
    let first = get(port, BOARD, None);
    assert_eq!(first.status, 200, "{}", first.body);
    assert_eq!(first.age(), None, "a fresh answer carries no age");
    assert_eq!(probe.starts(BOARD), 1);

    probe.set_delay(SLOW);
    case.write();
    let held = get(port, BOARD, Some(&first.etag()));
    assert!(held.took < STALE, "the stale answer took {:?}", held.took);
    assert_eq!(held.status, 304, "the client holds the stale answer");
    assert_eq!(held.etag(), first.etag());
    assert!(held.age().is_some(), "a stale answer names its age");
    let stale = get(port, BOARD, None);
    assert!(stale.took < STALE, "the stale answer took {:?}", stale.took);
    assert_eq!(stale.status, 200);
    assert_eq!(stale.body, first.body, "the stale answer is the kept one");
    assert_eq!(stale.etag(), first.etag());
    assert!(stale.age().is_some(), "a stale answer names its age");
    assert_eq!(probe.starts(BOARD), 2, "one recompute started");

    let fresh = until(port, BOARD, fresh_with_third);
    assert_ne!(fresh.etag(), first.etag());
}

/// Many requests for a stale answer start one recompute, and no two recomputes of one endpoint
/// ever run side by side.
#[test]
fn only_one_recompute_runs_at_a_time() {
    let case = Case::new("single");
    let probe = Arc::new(Probe::default());
    let port = case.served(tuning(&probe));
    assert_eq!(get(port, BOARD, None).status, 200);
    let before = probe.starts(BOARD);

    probe.set_delay(SLOW);
    case.write();
    let asking: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(move || get(port, BOARD, None)))
        .collect();
    for asked in asking {
        let answer = asked.join().expect("a request");
        assert_eq!(answer.status, 200, "{}", answer.body);
        assert!(answer.age().is_some(), "a stale answer while recomputing");
        assert!(answer.took < SLOW, "a request waited {:?}", answer.took);
    }
    assert_eq!(
        probe.starts(BOARD),
        before + 1,
        "eight requests, one recompute"
    );
    until(port, BOARD, fresh_with_third);
    assert_eq!(
        probe.peak(BOARD),
        1,
        "two recomputes of one endpoint ran at once"
    );
}

/// A recompute starts at most once per debounce: a write within it leaves the stale answer served
/// and no recompute started; one starts once the debounce has passed since the last.
#[test]
fn the_debounce_holds_a_recompute_back() {
    const DEBOUNCE: Duration = Duration::from_secs(8);
    let case = Case::new("debounce");
    let probe = Arc::new(Probe::default());
    let port = case.served(Tuning {
        debounce: DEBOUNCE,
        ..tuning(&probe)
    });
    assert_eq!(get(port, BOARD, None).status, 200);
    let started = probe.started(BOARD);
    assert_eq!(started.len(), 1);

    case.write();
    let hold = started[0] + DEBOUNCE - Duration::from_secs(2);
    assert!(
        Instant::now() < hold,
        "the write took too long to see the debounce"
    );
    while Instant::now() < hold {
        let answer = get(port, BOARD, None);
        assert_eq!(answer.status, 200);
        assert!(
            answer.age().is_some(),
            "within the debounce the answer is stale"
        );
        assert_eq!(
            probe.starts(BOARD),
            1,
            "a recompute started within the debounce"
        );
        std::thread::sleep(Duration::from_millis(250));
    }

    until(port, BOARD, fresh_with_third);
    let started = probe.started(BOARD);
    assert_eq!(started.len(), 2, "{started:?}");
    assert!(
        started[1] - started[0] >= DEBOUNCE,
        "the recompute started {:?} after the last",
        started[1] - started[0]
    );
}

/// A request before an endpoint's first answer exists is answered `503` with `Retry-After` within
/// the cold wait, not held until the computation ends; the answer follows once it is computed.
#[test]
fn a_request_before_the_first_answer_is_503() {
    let case = Case::new("cold");
    let probe = Arc::new(Probe::default());
    probe.set_delay(SLOW);
    let port = case.served(Tuning {
        cold_wait: Duration::from_millis(300),
        ..tuning(&probe)
    });
    let cold = get(port, BOARD, None);
    assert_eq!(cold.status, 503, "{}", cold.body);
    assert!(cold.header("Retry-After").is_some(), "{:?}", cold.head);
    assert!(cold.took < SLOW, "the cold request waited {:?}", cold.took);
    let warm = until(port, BOARD, |answer| answer.status == 200);
    assert!(warm.body.contains("repo-01"), "{}", warm.body);
}

/// A writer appends to the store while the dashboard runs, has served its answers and computes
/// them again after each write: the trees' writer locks are free whenever the dashboard has
/// answered, and each snapshot's appends end within the bound, however often the dashboard reads.
#[test]
fn a_writer_appends_while_the_dashboard_serves() {
    const APPEND: Duration = Duration::from_secs(60);
    let case = Case::new("writer");
    let probe = Arc::new(Probe::default());
    let port = case.served(tuning(&probe));
    for round in 0..3_usize {
        for path in [BOARD, "/api/repositories", "/api/unanswered", "/api/health"] {
            let answer = get(port, path, None);
            assert_eq!(answer.status, 200, "{path}: {}", answer.body);
        }
        for tree in ["tree", "observations"] {
            let lock = File::options()
                .write(true)
                .open(case.state().join(tree).join(".lock"))
                .expect("open the writer lock");
            lock.try_lock().unwrap_or_else(|error| {
                panic!("round {round}: the dashboard holds {tree}/.lock: {error}")
            });
        }
        let (done, appended) = mpsc::channel();
        let state = case.state();
        std::thread::spawn(move || {
            complete(&state, &format!("2026-10-13T1{round}:00:00Z"), 3);
            done.send(()).expect("send");
        });
        // The dashboard reads beside the writer: each request finds the store moving.
        let asked = Instant::now();
        loop {
            match appended.try_recv() {
                Ok(()) => break,
                Err(mpsc::TryRecvError::Disconnected) => panic!("round {round}: the writer failed"),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            assert!(
                asked.elapsed() < APPEND,
                "round {round}: the append did not end in {APPEND:?} beside the dashboard"
            );
            assert_eq!(get(port, BOARD, None).status, 200);
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    until(port, BOARD, fresh_with_third);
}

/// The dashboard's store handle reads a store another writer committed to since, without waiting
/// on the trees' writer locks, which a writer holds for as long as it commits.
#[test]
fn a_reader_does_not_wait_on_the_writer_lock() {
    let case = Case::new("lock");
    case.complete("2026-10-13T06:00:00Z", 2);
    let state = case.state();
    let (read_tx, read_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let reading = state.clone();
    std::thread::spawn(move || {
        let reader = store::open_reader(&reading).expect("open the reader");
        let rows = |reader: &store::Store| {
            dashboard::overview::snapshot_endpoint("/api/repositories", Some(reader))
                .expect("an endpoint")
                .to_string()
        };
        read_tx.send(rows(&reader)).expect("send");
        go_rx.recv().expect("go");
        read_tx.send(rows(&reader)).expect("send");
    });
    let first = read_rx.recv_timeout(PATIENCE).expect("the first read");
    assert!(
        first.contains("repo-01") && !first.contains("repo-02"),
        "{first}"
    );

    case.complete("2026-10-13T08:00:00Z", 3);
    let held: Vec<File> = ["tree", "observations"]
        .iter()
        .map(|tree| {
            let lock = File::options()
                .create(true)
                .truncate(false)
                .write(true)
                .open(state.join(tree).join(".lock"))
                .expect("open the writer lock");
            lock.lock().expect("hold the writer lock");
            lock
        })
        .collect();
    go_tx.send(()).expect("go");
    let second = read_rx.recv_timeout(Duration::from_secs(20));
    drop(held);
    let second = second.expect("the read waited on the writer lock");
    assert!(second.contains("repo-02"), "{second}");
}
