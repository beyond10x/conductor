//! A short build does not wait behind long gates (`story:build-run-short-lane`).
//!
//! Each case runs [`build::run`] in threads over a case directory, as `build_queue_convoy.rs`
//! does: free disk from `free.txt`, a reclaim that frees nothing, the commands in `work/`. One
//! regular build slot (`thresholds.build_slots` 1) and one short-lane slot. A command is `sh -c
//! <script>`, which the ledger keys as `sh -c` whatever the script: the runs the ledger holds for
//! it are written by the case ([`Case::history`]), each a released run of a given length. A
//! command keyed `bash -c` has no history. A script that holds its slot waits for a file of
//! `work/`; [`Release`] writes every such file when the case ends, however it ends, so no command
//! a case started outlives it.

mod active;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::ReclaimReport;
use conductor_cli::host;
use conductor_model::host::{BuildLane, HostGrantState, ReleaseReason};
use time::OffsetDateTime;

const GIB: u64 = 1 << 30;

/// The cases share the process's signal flags with `build_run`'s; one at a time keeps each case's
/// waits its own.
fn one_at_a_time() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("build_short_lane")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("work")).expect("create the case's work directory");
        fs::write(dir.join("free.txt"), (100 * GIB).to_string()).expect("write free disk");
        Self { dir }
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

    fn sources(&self) -> Sources {
        Sources {
            disk: self.dir.join("free.txt"),
            free_bytes: |file| Ok(fs::read_to_string(file)?.trim().parse()?),
            reclaim: reclaim_nothing,
            clock: OffsetDateTime::now_utc,
            every: Duration::from_millis(50),
            poll: Duration::from_millis(20),
            home: Some(self.dir.join("home")),
            var_tmp: self.dir.join("var-tmp"),
            scratch: self.dir.join("scratch"),
            say: said_to_file,
            env: |_| None,
            sessions: Vec::new(),
            walk: build::walk,
            sccache: None,
        }
    }

    fn request(&self, shell: &str, script: &str) -> Request {
        Request {
            reserve: Some(GIB),
            repository: Some("example-repo".to_owned()),
            session_name: None,
            command: vec![shell.to_owned(), "-c".to_owned(), script.to_owned()],
            measure: Vec::new(),
            cwd: self.work(),
            output: None,
        }
    }

    /// Writes `runs` released runs of `sh -c` to the ledger, each taking that long, the last a
    /// minute ago.
    fn history(&self, runs: &[Duration]) {
        let host = self.host();
        for (n, length) in runs.iter().enumerate() {
            let ended = OffsetDateTime::now_utc()
                - time::Duration::minutes(1 + i64::try_from(n).expect("a count"));
            let started = ended - time::Duration::try_from(*length).expect("a length");
            let at = host::now(started);
            let grant = {
                let mut ledger = host::Ledger::lock(&host).expect("the ledger opens");
                let grant = ledger
                    .request_of(
                        "example-org",
                        Some("example-repo"),
                        None,
                        None,
                        GIB,
                        "sh -c earlier",
                        &at,
                    )
                    .expect("the request is kept");
                ledger.reserve(&grant, 0, 0).expect("granted");
                grant
            };
            host::start_run(&host, &grant, 4242, &at).expect("the run starts");
            host::release(
                &host,
                &grant,
                Some(ReleaseReason::Exited),
                &host::now(ended),
            )
            .expect("the grant is released");
        }
    }

    fn row(&self, script: &str) -> Option<conductor_model::host::Grants> {
        host::rows(&self.host()).ok()?.into_iter().find(|row| {
            row.command
                .as_deref()
                .is_some_and(|command| command.ends_with(&format!("-c {script}")))
        })
    }

    fn state_of(&self, script: &str) -> Option<HostGrantState> {
        self.row(script).map(|row| row.state)
    }

    fn said(&self) -> String {
        fs::read_to_string(self.dir.join("stderr.txt")).unwrap_or_default()
    }

    /// Starts `build run` of `shell -c script` in a thread, and waits until its request is on the
    /// ledger, so the next one arrives after it.
    fn arrive(
        &self,
        settings: &Settings,
        shell: &str,
        script: &str,
    ) -> thread::JoinHandle<anyhow::Result<Outcome>> {
        let (state, host, settings, sources, request) = (
            self.state(),
            self.host(),
            settings.clone(),
            self.sources(),
            self.request(shell, script),
        );
        let runner =
            thread::spawn(move || build::run(&state, &host, &settings, &sources, &request));
        wait_until("the request is on the ledger", || {
            self.state_of(script).is_some()
        });
        runner
    }
}

/// Writes every `go-*` file a holding script waits for when dropped, so each command ends.
struct Release(PathBuf, Vec<&'static str>);

impl Drop for Release {
    fn drop(&mut self) {
        for name in &self.1 {
            let _ = fs::write(self.0.join(name), "");
        }
    }
}

/// A script that holds its slot until `work/<go>` exists, then writes `work/<go>.ran`.
fn holding(go: &str) -> String {
    format!("while [ ! -e {go} ]; do sleep 0.02; done; touch {go}.ran")
}

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
        slots: 1,
        repo_cache: false,
        repo_cache_size: 2 * GIB,
        lean_debuginfo: true,
        short_slots: 1,
        short_max: Duration::from_secs(180),
    }
}

fn reclaim_nothing(_state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let case = state.parent().expect("the state lies in its case");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case.join("stderr.txt"))
        .expect("open the case's stderr.txt");
    writeln!(file, "{line}").expect("write the case's stderr.txt");
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

fn ran(runner: thread::JoinHandle<anyhow::Result<Outcome>>) -> Outcome {
    runner
        .join()
        .expect("the run's thread ends")
        .expect("the run ends")
}

const SECOND: Duration = Duration::from_secs(1);

/// Acceptances 2, 3 and 5: with the regular slot taken and a longer request waiting for it, a
/// request whose command the ledger knows short (3 runs in 14 days, p90 within `short_lane_max`)
/// takes the short-lane slot ahead of it, with at most 4 jobs, and its grant names the lane.
#[test]
fn a_short_request_takes_the_short_lane_past_a_longer_one() {
    let _one = one_at_a_time();
    let case = Case::new("past");
    let _release = Release(case.work(), vec!["go-long", "go-gate"]);
    case.history(&[SECOND, SECOND, 2 * SECOND]);
    let gate = case.arrive(&settings(), "bash", &holding("go-gate"));
    wait_until("the gate runs", || {
        case.state_of(&holding("go-gate")) == Some(HostGrantState::Running)
    });
    // A command the ledger knows nothing of: it waits for the regular slot.
    let long = case.arrive(&settings(), "bash", &holding("go-long"));
    let short = "echo \"$CARGO_BUILD_JOBS $MAKEFLAGS\" > jobs.txt";
    let outcome = ran(case.arrive(&settings(), "sh", short));
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    assert_eq!(
        fs::read_to_string(case.work().join("jobs.txt"))
            .unwrap_or_default()
            .trim(),
        "4 -j4",
        "a short-lane run has at most 4 jobs"
    );
    let row = case.row(short).expect("the short run's grant");
    assert_eq!(row.lane, Some(BuildLane::Short), "{row:?}");
    assert_eq!(
        case.state_of(&holding("go-long")),
        Some(HostGrantState::Requested),
        "the longer request still waits for the regular slot"
    );
    assert_eq!(
        case.row(&holding("go-gate")).and_then(|row| row.lane),
        Some(BuildLane::Regular)
    );
    fs::write(case.work().join("go-gate"), "").expect("end the gate");
    ran(gate);
    wait_until("the long request runs", || {
        case.state_of(&holding("go-long")) == Some(HostGrantState::Running)
    });
    fs::write(case.work().join("go-long"), "").expect("end the long run");
    ran(long);
    let said = case.said();
    assert!(
        said.contains("lane regular"),
        "the queued line names the lane: {said}"
    );
}

/// Acceptance 2: fewer than 3 earlier runs, or a p90 over `short_lane_max`, is no short command:
/// it waits for the regular slot.
#[test]
fn a_command_without_a_short_history_waits_for_the_regular_slot() {
    let _one = one_at_a_time();
    for (name, history) in [
        ("two-runs", vec![SECOND, SECOND]),
        ("slow-p90", vec![SECOND, SECOND, Duration::from_secs(400)]),
    ] {
        let case = Case::new(name);
        let _release = Release(case.work(), vec!["go-gate"]);
        case.history(&history);
        let gate = case.arrive(&settings(), "bash", &holding("go-gate"));
        wait_until("the gate runs", || {
            case.state_of(&holding("go-gate")) == Some(HostGrantState::Running)
        });
        let waiting = case.arrive(&settings(), "sh", "touch done");
        thread::sleep(Duration::from_millis(400));
        assert_eq!(
            case.state_of("touch done"),
            Some(HostGrantState::Requested),
            "{name}: it waits for the regular slot"
        );
        assert!(
            case.said().contains("lane regular"),
            "{name}: {}",
            case.said()
        );
        fs::write(case.work().join("go-gate"), "").expect("end the gate");
        ran(gate);
        ran(waiting);
        let row = case.row("touch done").expect("its grant");
        assert_eq!(row.lane, Some(BuildLane::Regular), "{name}: {row:?}");
    }
}

/// Acceptance 3: short requests keep their queue order among themselves.
#[test]
fn short_requests_keep_their_order() {
    let _one = one_at_a_time();
    let case = Case::new("order");
    let _release = Release(case.work(), vec!["go-gate", "go-first", "go-a", "go-b"]);
    case.history(&[SECOND, SECOND, SECOND]);
    let gate = case.arrive(&settings(), "bash", &holding("go-gate"));
    wait_until("the gate runs", || {
        case.state_of(&holding("go-gate")) == Some(HostGrantState::Running)
    });
    let first = case.arrive(&settings(), "sh", &holding("go-first"));
    wait_until("the first short run takes the short lane", || {
        case.state_of(&holding("go-first")) == Some(HostGrantState::Running)
    });
    let a = case.arrive(&settings(), "sh", &holding("go-a"));
    let b = case.arrive(&settings(), "sh", &holding("go-b"));
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        case.state_of(&holding("go-a")),
        Some(HostGrantState::Requested)
    );
    assert_eq!(
        case.state_of(&holding("go-b")),
        Some(HostGrantState::Requested)
    );
    assert!(case.said().contains("lane short"), "{}", case.said());
    fs::write(case.work().join("go-first"), "").expect("end the first short run");
    ran(first);
    wait_until("a takes the short lane", || {
        case.state_of(&holding("go-a")) == Some(HostGrantState::Running)
    });
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        case.state_of(&holding("go-b")),
        Some(HostGrantState::Requested),
        "b waits behind a"
    );
    fs::write(case.work().join("go-a"), "").expect("end a");
    ran(a);
    fs::write(case.work().join("go-b"), "").expect("end b");
    ran(b);
    fs::write(case.work().join("go-gate"), "").expect("end the gate");
    ran(gate);
}

/// Acceptance 4: a short-lane run that outlives twice `short_lane_max` is not killed, is marked
/// on its grant, and its command takes no short-lane slot for 24 hours.
#[test]
fn a_short_run_that_overruns_is_marked_and_loses_the_lane() {
    let _one = one_at_a_time();
    let case = Case::new("overrun");
    let _release = Release(case.work(), vec!["go-gate"]);
    let quick = Settings {
        short_max: Duration::from_millis(200),
        ..settings()
    };
    case.history(&[Duration::ZERO, Duration::ZERO, Duration::ZERO]);
    let gate = case.arrive(&quick, "bash", &holding("go-gate"));
    wait_until("the gate runs", || {
        case.state_of(&holding("go-gate")) == Some(HostGrantState::Running)
    });
    let slow = "sleep 1; touch slow.ran";
    let outcome = ran(case.arrive(&quick, "sh", slow));
    assert_eq!(outcome.exit_code(), 0, "not killed: {outcome:?}");
    assert!(case.work().join("slow.ran").exists(), "it ran to its end");
    let row = case.row(slow).expect("its grant");
    assert_eq!(row.lane, Some(BuildLane::Short), "{row:?}");
    assert_eq!(row.lane_overrun, Some(true), "{row:?}");
    let next = case.arrive(&quick, "sh", "touch next");
    thread::sleep(Duration::from_millis(400));
    assert_eq!(
        case.state_of("touch next"),
        Some(HostGrantState::Requested),
        "the command lost the short lane"
    );
    fs::write(case.work().join("go-gate"), "").expect("end the gate");
    ran(gate);
    ran(next);
    assert_eq!(
        case.row("touch next").and_then(|row| row.lane),
        Some(BuildLane::Regular)
    );
}

/// Acceptance 1: `short_lane_slots` 0 turns the lane off.
#[test]
fn no_short_lane_slots_turn_the_lane_off() {
    let _one = one_at_a_time();
    let case = Case::new("off");
    let _release = Release(case.work(), vec!["go-gate"]);
    case.history(&[SECOND, SECOND, SECOND]);
    let off = Settings {
        short_slots: 0,
        ..settings()
    };
    let gate = case.arrive(&off, "bash", &holding("go-gate"));
    wait_until("the gate runs", || {
        case.state_of(&holding("go-gate")) == Some(HostGrantState::Running)
    });
    let waiting = case.arrive(&off, "sh", "touch done");
    thread::sleep(Duration::from_millis(400));
    assert_eq!(case.state_of("touch done"), Some(HostGrantState::Requested));
    fs::write(case.work().join("go-gate"), "").expect("end the gate");
    ran(gate);
    ran(waiting);
}
