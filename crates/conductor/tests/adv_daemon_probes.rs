//! Adversary cases on conductord's probes (`story:daemon-probes-and-metrics`), wave 51 pass 1.
//!
//! - Readiness is what the watcher's own store handle reads. The reconciler writes through another
//!   handle, opened once, and a handle that failed once refuses every later write: after one
//!   failed write `GET /readyz` goes on answering `200` while the daemon records nothing, for as
//!   long as the process runs.
//! - Readiness answers `200` as soon as the store answers, before the reconciler published, while
//!   `GET /v1/instances` still answers the nothing before it.
//!
//! No case reaches the host's service manager: a daemon started here runs on a `PATH` of the
//! case's own, which holds stand-ins that know and start no unit.

mod active;
mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::daemon::api;
use conductor_cli::store;
use conductor_model::behaviour::ReconciliationStorage;
use serde_json::json;

/// The daemon's token of a case.
const TOKEN: &str = "4444444444444444444444444444444444444444444444444444444444444444";

/// What the daemon's loop writes before a pass's failure.
const FAILED: &str = "conductor: daemon serve: ";

/// A case's directory; removed after a passing run, with the binary the daemon copied into it.
struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adp")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file: the daemon on `port` with `store` as its `daemon.store`, and the
    /// one instance `alpha`.
    fn config(&self, port: u16, store: &str) {
        let root = self.dir.join("alpha");
        let text = format!(
            "version: conductor.config/2\ndaemon: {{port: {port}, store: {store:?}}}\n\
             instances:\n  - name: alpha\n    placement: box-a\n    session_prefix: alpha\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
             state: {state:?}\n    cache: {cache:?}\n",
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    /// Changes the config file's text and nothing it says, so the daemon's next tick runs a pass.
    fn touch(&self, mark: &str) {
        let mut text = fs::read_to_string(self.file()).expect("read the config file");
        text.push_str(&format!("# {mark}\n"));
        fs::write(self.file(), text).expect("write the config file");
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

/// `GET target` on `port` as the listener's own host: its status and body; none while nothing
/// listens there.
fn ask(port: u16, target: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    stream
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    let (head, body) = answer.split_once("\r\n\r\n")?;
    let status = head.split(' ').nth(1)?.parse().ok()?;
    Some((status, body.to_owned()))
}

fn get(port: u16, target: &str) -> (u16, String) {
    ask(port, target).unwrap_or_else(|| panic!("no answer to GET {target}"))
}

/// A `daemon serve` of a case, killed when the case ends, with what it printed.
struct Served {
    child: Child,
    port: u16,
    printed: Arc<Mutex<String>>,
}

impl Served {
    /// Starts `daemon serve` on the case's config file. Its `PATH` holds two stand-ins and
    /// nothing else, and the variables that name the user's service manager are gone, so its
    /// reconcile pass reaches none: a `systemctl` that knows no unit and a `systemd-run` that
    /// starts none.
    fn spawn(case: &Case, port: u16) -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let programs = case.dir.join("programs");
        fs::create_dir_all(&programs).expect("create the PATH directory");
        for (name, script) in [
            (
                "systemctl",
                "#!/bin/sh\necho LoadState=not-found\necho ActiveState=inactive\n",
            ),
            (
                "systemd-run",
                "#!/bin/sh\necho 'this case starts no unit' >&2\nexit 1\n",
            ),
        ] {
            let program = programs.join(name);
            fs::write(&program, script).expect("write the stand-in");
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
                .expect("make the stand-in runnable");
        }
        let mut child = common::conductor(case.home())
            .arg("--config")
            .arg(case.file())
            .env("PATH", &programs)
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .args(["daemon", "serve", "--state-dir"])
            .arg(case.dir.join("daemon"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run daemon serve");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        Self {
            child,
            port,
            printed,
        }
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }

    /// How many failures of a pass the daemon's loop wrote.
    fn failures(&self) -> usize {
        self.printed().matches(FAILED).count()
    }

    fn runs(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Waits until `holds`, while the daemon runs.
    fn until(&mut self, what: &str, holds: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !holds(self) {
            assert!(self.runs(), "the daemon exited: {}", self.printed());
            assert!(
                Instant::now() < deadline,
                "never held: {what}: {}",
                self.printed()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Waits at most `bound` for `holds`; whether it held.
    fn within(&self, bound: Duration, holds: impl Fn(&Self) -> bool) -> bool {
        let deadline = Instant::now() + bound;
        while Instant::now() < deadline {
            if holds(self) {
                return true;
            }
            thread::sleep(Duration::from_millis(500));
        }
        holds(self)
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// How many reconciliations the store at `url` holds, read through a handle of the case's own.
fn recorded(url: &str) -> usize {
    let store =
        store::open_url(url, conductor_cli::daemon::TENANT).expect("open the daemon's store");
    ReconciliationStorage::list(&store).len()
}

/// The store takes no write for longer than a writer waits (another writer holds the database:
/// a backup, a migration, a long commit of a runtime that shares it), while it answers every
/// read. The daemon's pass fails to record once. The store then takes writes again. From there
/// on the daemon either records what it does, or its readiness says it is not ready; it may not
/// answer `200` on `/readyz` and record nothing.
#[test]
fn after_a_write_the_store_refused_the_daemon_records_again_or_reads_not_ready() {
    let case = Case::new("refused-write");
    let port = free_port();
    let file = case.dir.join("shared.sqlite");
    let url = format!("sqlite:{}", file.display());
    case.config(port, &url);
    let mut served = Served::spawn(&case, port);
    served.until("the first pass acted on the instance", |served| {
        served.printed().contains("alpha: Start")
    });
    served.until("readiness answers 200", |served| {
        ask(served.port, "/readyz").is_some_and(|(status, _)| status == 200)
    });
    let before = recorded(&url);
    assert!(before > 0, "the first pass recorded nothing");
    let failures = served.failures();

    // Another writer holds the database; a read is answered all the while.
    let writer = rusqlite::Connection::open(&file).expect("open the store as another writer");
    writer
        .execute_batch("BEGIN IMMEDIATE")
        .expect("take the store's write lock");
    case.touch("a pass while the store takes no write");
    let refused = served.within(Duration::from_secs(40), |served| {
        served.failures() > failures
    });
    let while_held = get(port, "/readyz").0;
    writer
        .execute_batch("ROLLBACK")
        .expect("release the store's write lock");
    drop(writer);
    assert!(served.runs(), "the daemon exited: {}", served.printed());
    let held = recorded(&url);

    // The store takes writes again: a pass that acts is recorded, or the daemon is not ready.
    case.touch("a pass after the store took writes again");
    let recovered = served.within(Duration::from_secs(40), |served| {
        recorded(&url) > held || get(served.port, "/readyz").0 != 200
    });
    let (ready, body) = get(port, "/readyz");
    let now = recorded(&url);
    assert!(
        recovered,
        "/readyz answers {ready} {} while the daemon records nothing: {now} reconciliations in \
         the store, {held} when the store took writes again (a pass failed while it took none: \
         {refused}; /readyz answered {while_held} meanwhile). The daemon printed:\n{}",
        body.trim(),
        served.printed()
    );
}

/// The daemon is ready once its store answers and its reconciler published what it read. A
/// daemon whose store was found not to answer at its start answers every request without
/// waiting; when the store then answers, readiness may not say `200` before the reconciler
/// opened it and published, since `GET /v1/instances` answers nothing until then.
#[test]
fn readiness_is_not_200_before_the_reconciler_published() {
    active::isolate();
    let daemon =
        Arc::new(api::Api::new(TOKEN.to_owned(), "operator".to_owned(), None).starting(Vec::new()));
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&daemon);
    thread::spawn(move || api::serve(&listener, &served));

    // `daemon serve` found the store not to answer and went on serving; now the store answers
    // the watcher, and the reconciler has neither opened it nor published.
    daemon.started();
    daemon.publish_store(true);
    let (_, listed) = get(port, "/v1/instances");
    let (ready, body) = get(port, "/readyz");
    assert_eq!(
        ready,
        503,
        "/readyz answers 200 {} while the reconciler published nothing and /v1/instances \
         answers {}",
        body.trim(),
        listed.trim()
    );

    // Once it published, the daemon is ready and lists what it read.
    daemon.publish(
        vec![json!({"name": "example-one", "state": "Running"})],
        std::collections::BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(get(port, "/readyz").0, 200);
    assert!(get(port, "/v1/instances").1.contains("example-one"));
}
