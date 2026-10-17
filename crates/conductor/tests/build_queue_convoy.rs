//! `story:build-queue-convoy`: requests waiting in the host build queue do not serialize the
//! host. A waiting `build run` reads the ledger's generation without `host.lock` and takes the
//! lock only when the ledger changed, its own answer may have, or `thresholds.watchdog_every`
//! has passed since the host's last sweep; it opens the host store once and reuses the handle; and
//! a pass reads the open grants only, however many closed ones the ledger keeps.
//!
//! Each case runs [`build::run`] in threads over a case directory, as `tests/build_run.rs` does:
//! free disk is a byte count in `free.txt`, read through `Sources::free_bytes`, and each command
//! is a real `sh -c` in `work/` of the case. The counts are this process's
//! ([`host::passes`]) or a run's thread's ([`host::passes_on_this_thread`],
//! [`store::replays_on_this_thread`]), so the cases run one at a time ([`one_at_a_time`]). The
//! slow store of the convoy case is [`store::slow_replays`]: every replay of a tree waits, one at
//! a time, as on a disk saturated by builds.

mod active;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::ReclaimReport;
use conductor_cli::{host, store};
use conductor_model::host::{GrantId, HostGrantState};
use eventlog_core::{
    AtomicEventStore as _, CommandMeta, Expected, NewEvent, StreamAppend, StreamId, TenantId,
    new_event_id, request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const GIB: u64 = 1 << 30;

/// The counts these cases read are the process's: one case at a time.
fn one_at_a_time() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One case's directory: the instance's state, the host directory, and the commands' `work/`.
struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("build_queue_convoy")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("work")).expect("create the case's work directory");
        let case = Self { dir };
        case.free(20 * GIB);
        case
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn host(&self) -> PathBuf {
        self.dir.join("host")
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    fn free(&self, bytes: u64) {
        let file = self.dir.join("free.txt");
        let next = file.with_extension("next");
        fs::write(&next, bytes.to_string()).expect("write free.txt");
        fs::rename(&next, &file).expect("replace free.txt");
    }

    /// The case's sources: free disk from `free.txt`, a reclaim that frees nothing, no session
    /// list, the watchdog every `every` and the queue looked at every `poll`.
    fn sources(&self, every: Duration, poll: Duration) -> Sources {
        Sources {
            disk: self.dir.join("free.txt"),
            free_bytes: written_bytes,
            reclaim: reclaim_nothing,
            clock: OffsetDateTime::now_utc,
            every,
            poll,
            home: Some(self.dir.join("home").join("example-user")),
            var_tmp: self.dir.join("var-tmp"),
            scratch: self.dir.join("scratch"),
            say: |_, _| {},
            env: |_| None,
            sessions: Vec::new(),
            walk: build::walk,
        }
    }

    fn request(&self, reserve: u64, script: &str) -> Request {
        Request {
            reserve: Some(reserve),
            repository: Some("example-repo".to_owned()),
            session_name: None,
            command: vec!["sh".to_owned(), "-c".to_owned(), script.to_owned()],
            measure: Vec::new(),
            cwd: self.work(),
        }
    }

    /// The state of the grant whose command is `sh -c <script>`.
    fn state_of(&self, script: &str) -> Option<HostGrantState> {
        let command = format!("sh -c {script}");
        host::rows(&self.host())
            .ok()?
            .into_iter()
            .find(|row| row.command.as_deref() == Some(command.as_str()))
            .map(|row| row.state)
    }
}

/// The thresholds: a 10G floor; a head waits a minute for live reservations.
fn settings() -> Settings {
    Settings {
        instance: "example-org".to_owned(),
        fall_key: "example-org".to_owned(),
        floor: 10 * GIB,
        clear: 20 * GIB,
        gate_stop: 5 * GIB,
        gate_resume: 8 * GIB,
        wait: Duration::from_secs(60),
        cores: 20,
        cpu_reserve: 2,
        slots: 0,
    }
}

fn written_bytes(file: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(file)?.trim().parse()?)
}

fn reclaim_nothing(_state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

/// What a run's thread answers: its outcome, when it ended, and its own counts of ledger passes
/// and store replays.
struct Ran {
    outcome: anyhow::Result<Outcome>,
    ended: Instant,
    passes: usize,
    replays: usize,
}

/// Starts `build run` of `script` reserving `reserve` in a thread, and waits until its request is
/// on the ledger, so the next one arrives after it.
fn arrive(case: &Case, sources: &Sources, reserve: u64, script: &str) -> thread::JoinHandle<Ran> {
    arrive_under(case, sources, settings(), reserve, script)
}

/// [`arrive`] under `settings`.
fn arrive_under(
    case: &Case,
    sources: &Sources,
    settings: Settings,
    reserve: u64,
    script: &str,
) -> thread::JoinHandle<Ran> {
    let (state, host) = (case.state(), case.host());
    let (sources, request) = (sources.clone(), case.request(reserve, script));
    let runner = thread::spawn(move || {
        let outcome = build::run(&state, &host, &settings, &sources, &request);
        Ran {
            outcome,
            ended: Instant::now(),
            passes: host::passes_on_this_thread(),
            replays: store::replays_on_this_thread(),
        }
    });
    wait_until("the request is on the ledger", || {
        case.state_of(script).is_some()
    });
    runner
}

fn ran(runner: thread::JoinHandle<Ran>) -> Ran {
    let ran = runner.join().expect("the runner ends");
    assert!(
        matches!(ran.outcome, Ok(Outcome::Ran { code: 0, .. })),
        "{:?}",
        ran.outcome
    );
    ran
}

/// The command of the run that holds 6G until the case writes `stop` in `work/`.
const HOLDING: &str = "while [ ! -e stop ]; do sleep 0.02; done";

/// Starts the run that holds 6G of the 10G over the floor, and waits until it runs.
fn holding(case: &Case, sources: &Sources) -> thread::JoinHandle<Ran> {
    let runner = arrive(case, sources, 6 * GIB, HOLDING);
    wait_until("the holding run runs", || {
        case.state_of(HOLDING) == Some(HostGrantState::Running)
    });
    runner
}

/// Ends a slow store when a case ends, however it ends.
struct SlowStore;

impl Drop for SlowStore {
    fn drop(&mut self) {
        store::slow_replays(Duration::ZERO);
    }
}

/// Acceptance 1: eight runs wait (a head 2G short of what a running build holds, seven behind
/// it), and nothing changes the ledger. They read its generation without the lock, and the host
/// is swept at most once per `thresholds.watchdog_every`, whichever run sweeps it: not once per
/// run per pause.
#[test]
fn waiting_runs_take_no_lock_while_the_ledger_does_not_change() {
    let _one = one_at_a_time();
    let case = Case::new("no-change");
    let every = Duration::from_millis(500);
    let sources = case.sources(every, Duration::from_millis(20));
    let first = holding(&case, &sources);
    let head = arrive(&case, &sources, 6 * GIB, "touch head");
    let behind: Vec<_> = (0..7)
        .map(|n| arrive(&case, &sources, 2 * GIB, &format!("touch behind-{n}")))
        .collect();
    // Every waiter has passed once since the last request arrived.
    thread::sleep(Duration::from_millis(300));

    let window = Duration::from_secs(2);
    let from = host::passes();
    thread::sleep(window);
    let passes = host::passes() - from;
    eprintln!("no-change: {passes} locked passes in {window:?} with 8 runs waiting");
    // One sweep per `watchdog_every`, and one more where the window cuts an interval.
    let most = usize::try_from(window.as_millis() / every.as_millis()).unwrap_or(0) + 1;
    assert!(
        passes <= most,
        "{passes} locked passes in {window:?} while 8 runs waited and nothing changed the \
         ledger; at most {most}, one sweep of the host per watchdog_every"
    );

    fs::write(case.work().join("stop"), "").expect("stop the holding run");
    for runner in std::iter::once(first).chain([head]).chain(behind) {
        ran(runner);
    }
}

/// Acceptance 2: a run that waits while six more requests arrive behind it passes once for each,
/// and opens the host store once for all its passes: a pass that finds no change in the streams it
/// reads reopens nothing. Its end (`finish_run`, after its command ran) may open it once more.
#[test]
fn a_waiting_run_opens_the_host_store_once_however_often_it_passes() {
    let _one = one_at_a_time();
    let case = Case::new("store-once");
    // No sweep of the host is due while the case runs: every pass is one a change calls for.
    let sources = case.sources(Duration::from_secs(30), Duration::from_millis(20));
    let first = holding(&case, &sources);
    let head = arrive(&case, &sources, 6 * GIB, "touch head");
    let mut behind = Vec::new();
    for n in 0..6 {
        thread::sleep(Duration::from_millis(150));
        behind.push(arrive(
            &case,
            &sources,
            2 * GIB,
            &format!("touch behind-{n}"),
        ));
    }
    thread::sleep(Duration::from_millis(150));
    fs::write(case.work().join("stop"), "").expect("stop the holding run");
    let head = ran(head);
    eprintln!(
        "store-once: the head made {} passes and {} store replays",
        head.passes, head.replays
    );
    assert!(
        head.passes >= 6,
        "the head passed {} times; six requests arrived behind it while it waited",
        head.passes
    );
    assert!(
        head.replays <= 2,
        "the head replayed the host store {} times over {} passes: once to start, and at most \
         once more to release its grant",
        head.replays,
        head.passes
    );
    for runner in std::iter::once(first).chain(behind) {
        ran(runner);
    }
}

/// `n` closed disk grants, two days old, committed to the host store under `host` in one command
/// beside conductor, as a ledger of long standing holds them.
fn plant_closed(host: &Path, n: usize) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let log = runtime
        .block_on(TreeEventStore::open(host.join("tree")))
        .expect("open the host store");
    let tenant = TenantId::new("conductor").expect("the tenant");
    let old = (OffsetDateTime::now_utc() - time::Duration::days(2))
        .format(&Rfc3339)
        .expect("an instant");
    let states = [
        ("Released", Some("Exited")),
        ("Refused", None),
        ("Withdrawn", None),
    ];
    let appends: Vec<StreamAppend> = (0..n)
        .map(|at| {
            let id = format!("closed-{at:05}");
            let (state, reason) = states[at % states.len()];
            let body = json!({
                "state": state,
                "grant_id": id,
                "instance": "example-org",
                "repository": "example-repo",
                "kind": "Disk",
                "bytes": GIB,
                "command": "sh -c true",
                "requested_at": old,
                "released_at": old,
                "release_reason": reason,
            });
            StreamAppend {
                stream: StreamId::new(tenant.clone(), "conductor.host.HostGrant", id)
                    .expect("a stream"),
                expected: Expected::NoStream,
                events: vec![NewEvent::new("stored", 1, body).expect("an event")],
            }
        })
        .collect();
    let id = new_event_id();
    let meta = CommandMeta {
        idempotency_key: id.clone(),
        request_hash: request_hash(&appends.iter().map(|a| &a.events).collect::<Vec<_>>())
            .expect("a hash"),
        subject: "conductor".to_owned(),
        actor: "conductor".to_owned(),
        request_id: id.clone(),
        trace_id: id,
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
        claim: None,
    };
    let group = eventlog_core::AppendGroup {
        tenant,
        meta,
        appends,
    };
    runtime
        .block_on(log.append_group(&group))
        .expect("plant the closed grants");
}

/// One pass of a waiting run over `ledger`, for the request `own`: what it sweeps, the bytes the
/// live grants hold, the bytes asked ahead of it, and the requests bound to a session.
fn pass(ledger: &mut host::Ledger, own: &GrantId) -> (usize, u64, Vec<u64>, usize) {
    let at = host::now(OffsetDateTime::now_utc());
    let swept = ledger.sweep(&at).expect("the sweep");
    let reserved = ledger.reserved(Some(own)).expect("the reservations");
    let ahead = ledger.ahead_bytes(own).expect("the queue");
    let bound = ledger.session_bound().expect("the session-bound requests");
    (swept.len(), reserved, ahead, bound.len())
}

/// Acceptance 3: beside 5,000 closed grants older than a day, a pass of a run reads the three
/// open requests only, while `host grants` still lists every grant.
#[test]
fn a_pass_reads_only_the_open_grants_beside_5000_closed_ones() {
    let _one = one_at_a_time();
    let case = Case::new("closed-5000");
    let host = case.host();
    drop(host::Ledger::lock(&host).expect("the ledger opens"));
    plant_closed(&host, 5000);

    let at = host::now(OffsetDateTime::now_utc());
    let mut ledger = host::Ledger::lock(&host).expect("the ledger opens");
    let mut grants = Vec::new();
    let mut runs = Vec::new();
    for n in 0..3 {
        let grant = ledger
            .request_of(
                "example-org",
                Some("example-repo"),
                None,
                Some(&format!("session-{n}")),
                GIB,
                &format!("sh -c touch open-{n}"),
                &at,
            )
            .expect("the request");
        runs.push(host::hold_run(&host, &grant).expect("the run lock"));
        grants.push(grant);
    }
    // The ledger planted beside conductor is read whole once.
    pass(&mut ledger, &grants[2]);
    assert!(
        ledger.grants_read() >= 5000,
        "the first pass read {} grants: the planted ones are on the ledger",
        ledger.grants_read()
    );
    drop(ledger);

    // The next run's pass reads the open ones only.
    let mut ledger = host::Ledger::lock(&host).expect("the ledger opens");
    let (swept, reserved, ahead, bound) = pass(&mut ledger, &grants[2]);
    let read = ledger.grants_read();
    eprintln!("closed-5000: a pass read {read} grants; 3 are open, 5000 closed");
    assert_eq!(swept, 0, "every request is held by its run");
    assert_eq!(reserved, 0, "no grant is live");
    assert_eq!(ahead, vec![GIB, GIB], "two requests are ahead of the third");
    assert_eq!(bound, 3, "each request names its session");
    assert!(
        read <= 4 * 3,
        "one pass (sweep, reserved, ahead_bytes, session_bound) read {read} grants; 3 are open"
    );
    drop(ledger);
    drop(runs);
    assert_eq!(
        host::rows(&host).expect("host grants reads").len(),
        5003,
        "host grants lists the closed grants too"
    );
}

/// Acceptance 4: twenty runs wait on a slow store (every replay of a tree takes 50ms, one at a
/// time, as on a disk saturated by builds): a head 2G short of what a running build holds, and
/// nineteen behind it. Once the build ends and releases its grant, the head is admitted within
/// one pause: it ran within twice `Sources::poll` of the release.
#[test]
fn twenty_waiters_on_a_slow_store_admit_the_head_within_one_pause_of_a_release() {
    let _one = one_at_a_time();
    let case = Case::new("convoy");
    let poll = Duration::from_millis(200);
    let sources = case.sources(Duration::from_secs(1), poll);
    let _slow = SlowStore;
    store::slow_replays(Duration::from_millis(50));
    let first = holding(&case, &sources);
    let head = arrive(&case, &sources, 6 * GIB, "touch head");
    let behind: Vec<_> = (0..19)
        .map(|n| arrive(&case, &sources, 2 * GIB, &format!("touch behind-{n}")))
        .collect();
    thread::sleep(Duration::from_secs(1));

    let window = Duration::from_secs(3);
    let from = host::passes();
    thread::sleep(window);
    let quiet = host::passes() - from;

    fs::write(case.work().join("stop"), "").expect("stop the holding run");
    let released = ran(first).ended;
    wait_until("the head runs", || case.work().join("head").exists());
    let admitted = released.elapsed();
    let head = ran(head);
    let mut replays = head.replays;
    for runner in behind {
        replays += ran(runner).replays;
    }
    eprintln!(
        "convoy: 20 waiters on a slow store: {quiet} locked passes in {window:?} with no change; \
         the head ran {admitted:?} after the release; {replays} store replays by the 20 runs"
    );
    assert!(
        admitted <= 2 * poll,
        "the head ran {admitted:?} after the release; one pause is {poll:?}"
    );
}

/// Appends `line` to `stderr.txt` beside the state directory `state`.
fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let file = state
        .parent()
        .expect("the state lies in its case")
        .join("stderr.txt");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .expect("open the case's stderr.txt");
    writeln!(file, "{line}").expect("write the case's stderr.txt");
}

/// Acceptance 6: with `thresholds.build_slots` 1 and one build running, a second request that
/// fits on disk waits in the queue, names `slots` as what it waits for, is never refused for it,
/// and is granted once the running build releases its grant.
#[test]
fn with_one_build_slot_a_second_request_waits_for_the_release() {
    let _one = one_at_a_time();
    let case = Case::new("slots");
    // 100G free over the 10G floor: disk holds both.
    case.free(100 * GIB);
    let sources = Sources {
        say: said_to_file,
        ..case.sources(Duration::from_millis(50), Duration::from_millis(20))
    };
    let one = Settings {
        slots: 1,
        // A wait for live reservations would end in a refusal; a wait for a slot never does.
        wait: Duration::from_millis(200),
        ..settings()
    };
    let first = arrive_under(&case, &sources, one.clone(), 6 * GIB, HOLDING);
    wait_until("the first build runs", || {
        case.state_of(HOLDING) == Some(HostGrantState::Running)
    });
    let second = arrive_under(&case, &sources, one, GIB, "touch second");
    thread::sleep(Duration::from_millis(600));
    assert_eq!(
        case.state_of("touch second"),
        Some(HostGrantState::Requested),
        "the second request waits for the one build slot"
    );
    assert!(!case.work().join("second").exists());
    let said = fs::read_to_string(case.dir.join("stderr.txt")).unwrap_or_default();
    assert!(
        said.contains("waiting for slots"),
        "the queued line names slots as what it waits for: {said}"
    );

    fs::write(case.work().join("stop"), "").expect("stop the first build");
    ran(first);
    ran(second);
    assert!(case.work().join("second").exists(), "the second ran");
}
