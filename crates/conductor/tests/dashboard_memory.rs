//! The dashboard's memory stays bounded however long it runs (`story:dashboard-memory-bound`):
//! with a thousand writes to the store between requests, the health view's `memory` part shows one
//! store handle, whose reopens drop the old replay before they build the new one, and at most
//! [`PER_ENDPOINT`] answers kept for any one endpoint, however many query strings were asked.
//!
//! The case serves [`dashboard::serve_tuned`] in this process on a free port of `127.0.0.1` over a
//! fixture store under this test target's temporary directory, as `dashboard_swr.rs` does.

mod active;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use conductor_cli::collect::Collector;
use conductor_cli::dashboard::{self, Sources, Tuning};
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_model::behaviour::RepositoryMarkStorage;
use conductor_model::direction::{
    MarkedBy, MarkedRepository, RepositoryMarkData, RepositoryMarkSnapshot, RepositoryMarkState,
};
use conductor_model::observation::{
    CommitSha, RecordRepository, RepositoryName, SnapshotState, StartSnapshot, Visibility,
};
use conductor_model::primitives::Timestamp;
use serde_json::Value;

/// The most answers the server keeps for one endpoint (`PER_ENDPOINT` in
/// `src/dashboard/cache.rs`).
const PER_ENDPOINT: u64 = 16;

/// The endpoints whose answers are kept by request target (`STORED_PATHS` in
/// `src/dashboard.rs`), and the three answers kept under one key each.
const ENDPOINTS: u64 = 12 + 3;

/// The trees one store handle may hold replayed: `tree/`, `observations/` and `series/`.
const TREES: u64 = 3;

/// The writes to the store between two rounds of requests.
const WRITES: usize = 1_000;

/// The rounds of writes, each followed by its requests.
const ROUNDS: usize = 2;

/// The distinct query strings of one endpoint asked in each round: more than [`PER_ENDPOINT`].
const QUERIES: usize = 40;

/// Longer than the fingerprint's settling window (`SETTLED` in `src/dashboard/cache.rs`).
const SETTLE: Duration = Duration::from_millis(2_500);

/// The bound on anything the case waits for.
const PATIENCE: Duration = Duration::from_secs(120);

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
            .join("dashboard-memory")
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

    /// A store of one complete snapshot, settled, served without a debounce; the port.
    fn served(&self) -> u16 {
        complete(&self.state());
        std::thread::sleep(SETTLE);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        let tuning = Tuning {
            debounce: Duration::ZERO,
            cold_wait: PATIENCE,
            probe: None,
        };
        std::thread::spawn(move || dashboard::serve_tuned(listener, sources, Some(state), tuning));
        port
    }

    /// [`WRITES`] writes to the store by a writer of its own, each one commit: a repository mark
    /// each, named after `round`.
    fn writes(&self, round: usize) {
        let mut store = store::open(&self.state()).expect("open the store");
        for index in 0..WRITES {
            RepositoryMarkStorage::put(
                &mut store,
                RepositoryMarkSnapshot {
                    state: RepositoryMarkState::Inactive,
                    data: RepositoryMarkData {
                        repository: MarkedRepository(format!("repo-{round}-{index:04}")),
                        marked_by: MarkedBy::Operator,
                        session: None,
                        reason: "a write between requests".to_owned(),
                    },
                },
            );
        }
        store.check().expect("every write is kept");
    }
}

/// One complete snapshot of two repositories written to the store under `state`.
fn complete(state: &Path) {
    let store = store::open(state).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp("2026-10-13T06:00:00Z".to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    let fixture = Collector::new("fixture", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        for index in 0..2 {
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
    });
    let taken =
        snapshot::take_from(store, start, &[fixture]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
}

/// The status and body of `GET path`.
fn get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(PATIENCE + Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
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

/// The health view's `memory` part.
fn memory(port: u16) -> Value {
    let (status, body) = get(port, "/api/health");
    assert_eq!(status, 200, "{body}");
    let health: Value = serde_json::from_str(&body).expect("the health view is JSON");
    health["memory"].clone()
}

/// `field` of the `memory` part, a count.
fn count(memory: &Value, field: &str) -> u64 {
    memory[field]
        .as_u64()
        .unwrap_or_else(|| panic!("no count {field} in the memory part {memory}"))
}

/// With [`WRITES`] writes between rounds of requests for many query strings of one endpoint, the
/// health view shows one store handle that never held two replays of one tree at once, and at
/// most [`PER_ENDPOINT`] answers kept for any one endpoint.
#[test]
fn memo_entries_and_store_handles_stay_bounded_across_writes() {
    let case = Case::new("bounded");
    let port = case.served();
    for round in 0..ROUNDS {
        case.writes(round);
        for query in 0..QUERIES {
            let (status, body) = get(port, &format!("/api/board?view={round}-{query}"));
            assert_eq!(status, 200, "round {round}, query {query}: {body}");
        }
        let memory = memory(port);
        eprintln!("round {round}: memory {memory}");
        assert_eq!(count(&memory, "store_handles"), 1, "{memory}");
        assert!(count(&memory, "store_replays") <= TREES, "{memory}");
        assert_eq!(
            count(&memory, "store_replays_peak"),
            1,
            "a reopen held the old replay while it built the new one: {memory}"
        );
        assert!(count(&memory, "store_bytes") > 0, "{memory}");
        assert!(
            count(&memory, "memo_entries_peak_endpoint") <= PER_ENDPOINT,
            "one endpoint keeps more answers than its bound: {memory}"
        );
        assert!(
            count(&memory, "memo_entries") <= PER_ENDPOINT * ENDPOINTS,
            "{memory}"
        );
        assert!(count(&memory, "memo_bytes") > 0, "{memory}");
        if cfg!(target_os = "linux") {
            assert!(count(&memory, "rss_bytes") > 0, "{memory}");
        }
    }
}
