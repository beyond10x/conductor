//! The instance runtime runs the instance's loop (design architecture-next § 2, the instance
//! runtime's row: "the loop (`conductor-run.md`), the instance API, the starter of the instance's
//! agents"; § 10: "the loop is the instance runtime's core"): `conductor instance serve` takes the
//! lock `conductor run` takes, `<state>/run/lock`, and runs the ticks while it holds it, so a
//! `conductor run` started by hand beside it is refused; a runtime that finds the lock held yields
//! the loop to its holder and takes it once it is released.
//!
//! Each case is a directory of its own under this test target's temporary directory, kept short
//! so the socket's path fits: `h/`, the `HOME` the program runs with; `r/`, the records; `s/`, the
//! state directory; `b/`, the stand-ins, alone on `PATH`: `claude` (`agents --json` prints `[]`),
//! `gh`, `glab` and `aep` (`[]`), `ais` (`{}`), and `tmux` and `notify-send`, which do nothing; and
//! the config file `h/conductor.yaml` of one instance, its store a SQLite file of the case, its
//! watch every second. The runtime writes what it says to `runtime.log` of the case
//! (`CONDUCTOR_API_LOG`). The free-disk readings say 500 GiB, so no `disk low` starts a `gc`.

mod active;
mod common;

use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Child, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// The token the case hands the runtime, as the daemon would.
const TOKEN: &str = "3333333333333333333333333333333333333333333333333333333333333333";

/// How long a runtime has to log its first line, and its loop to record a tick.
const BOUND: Duration = Duration::from_secs(60);

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

/// A running runtime, killed when dropped.
struct Runtime(Child);

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("rl")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        let case = Self { dir };
        for sub in ["r/dispatches", "r/decisions", "s", "h", "b"] {
            fs::create_dir_all(case.dir.join(sub)).expect("create the case's directories");
        }
        for (name, body) in [
            ("claude", "echo '[]'"),
            ("gh", "echo '[]'"),
            ("glab", "echo '[]'"),
            ("aep", "echo '[]'"),
            ("ais", "echo '{}'"),
            ("tmux", "exit 1"),
            ("notify-send", "exit 0"),
        ] {
            let file = case.dir.join("b").join(name);
            fs::write(&file, format!("#!/bin/sh\n{body}\n")).expect("write a stand-in");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
                .expect("make it runnable");
        }
        fs::write(case.dir.join("readings"), "536870912000\n").expect("write the readings");
        fs::write(case.file(), case.text()).expect("write the config file");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("h")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("s")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    fn log(&self) -> PathBuf {
        self.dir.join("runtime.log")
    }

    fn lock(&self) -> PathBuf {
        self.state().join("run").join("lock")
    }

    fn text(&self) -> String {
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   placement: local\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             \x20   cadence: {{watch: 1s}}\n",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            root = self.home().join("example-org"),
            trees = self.home().join("trees"),
            records = self.dir.join("r"),
            state = self.state(),
            cache = self.dir.join("c"),
        )
    }

    /// `conductor --config <file> <args>`, with the case's home, stand-ins and readings.
    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = common::conductor(self.home());
        command
            .arg("--config")
            .arg(self.file())
            .args(args)
            .current_dir(self.state())
            .env("PATH", self.dir.join("b"))
            .env(
                conductor_cli::gc::reclaim::READINGS_VARIABLE,
                self.dir.join("readings"),
            )
            .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
            .stdin(Stdio::null());
        command
    }

    fn conductor(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("conductor runs")
    }

    /// `conductor instance serve`, writing what it says to [`Case::log`], with `env` set besides.
    fn serve(&self, env: &[(&str, &str)]) -> Runtime {
        let mut command = self.command(&["instance", "serve"]);
        command
            .env(conductor_cli::instance::TOKEN_VARIABLE, TOKEN)
            .env(conductor_cli::instance::LOG_VARIABLE, self.log())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut runtime = Runtime(command.spawn().expect("instance serve starts"));
        self.wait_for(&mut runtime, "the runtime to serve /v1/", |case| {
            case.said().contains("serving /v1/")
        });
        runtime
    }

    /// What the runtime said so far.
    fn said(&self) -> String {
        fs::read_to_string(self.log()).unwrap_or_default()
    }

    /// The ticks the store holds.
    fn ticks(&self) -> Vec<Value> {
        let output = self.conductor(&["session", "run-ticks", "--format", "json"]);
        if !output.status.success() {
            return Vec::new();
        }
        serde_json::from_slice(&output.stdout).unwrap_or_default()
    }

    /// Waits up to [`BOUND`] for `done`; fails naming `what` and what the runtime said.
    fn wait_for(&self, runtime: &mut Runtime, what: &str, done: impl Fn(&Self) -> bool) {
        let until = Instant::now() + BOUND;
        while !done(self) {
            if let Ok(Some(status)) = runtime.0.try_wait() {
                panic!(
                    "instance serve exited {status} waiting for {what}; it said:\n{}",
                    self.said()
                );
            }
            assert!(
                Instant::now() < until,
                "no {what} within {BOUND:?}; the runtime said:\n{}",
                self.said()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// Asserts a `conductor run --once` started by hand now is refused, naming the lock.
    fn hand_run_refused(&self) {
        let output = self.conductor(&["run", "--once"]);
        assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(
            said.contains(&self.lock().display().to_string())
                && said.contains("one runs per instance"),
            "the refusal names the lock: {}",
            describe(&output)
        );
        assert_eq!(said.lines().count(), 1, "one line: {}", describe(&output));
    }
}

/// The runtime runs the loop: its ticks are recorded in the instance's store with no
/// `conductor run` started, and a `conductor run` started by hand beside it is refused by the lock
/// the runtime holds.
#[test]
fn the_runtime_runs_the_loop_and_a_run_started_by_hand_is_refused() {
    let case = Case::new("runs");
    let mut runtime = case.serve(&[]);
    case.wait_for(
        &mut runtime,
        "tick recorded by the runtime's loop",
        |case| !case.ticks().is_empty(),
    );
    let kinds: Vec<String> = case
        .ticks()
        .iter()
        .filter_map(|tick| tick["kind"].as_str().map(str::to_owned))
        .collect();
    assert!(
        kinds.iter().any(|kind| kind == "watch"),
        "a watch tick: {kinds:?}"
    );
    case.hand_run_refused();
    assert!(
        runtime.0.try_wait().expect("wait").is_none(),
        "the runtime still runs"
    );
}

/// A runtime that finds the loop's lock held (here by this test, as a `conductor run` started by
/// hand holds it) yields: it serves the API, says once that it yields and to what, and records
/// no tick; once the lock is released it takes it within a few watch periods and runs the loop.
#[test]
fn a_runtime_yields_the_loop_to_the_holder_of_the_lock_and_takes_it_once_released() {
    let case = Case::new("yielding");
    fs::create_dir_all(case.lock().parent().expect("the run directory"))
        .expect("create the run directory");
    let held: File = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(case.lock())
        .expect("open the lock");
    held.try_lock().expect("this test holds the lock");
    let mut runtime = case.serve(&[]);
    case.wait_for(&mut runtime, "word that the loop yields", |case| {
        case.said().contains("the loop yields")
    });
    std::thread::sleep(Duration::from_secs(3));
    let said = case.said();
    assert_eq!(
        said.matches("the loop yields").count(),
        1,
        "said once, not every period:\n{said}"
    );
    assert!(
        said.contains(&case.lock().display().to_string()),
        "names the lock:\n{said}"
    );
    assert!(case.ticks().is_empty(), "no tick while it yields");

    drop(held);
    case.wait_for(&mut runtime, "tick once the lock was released", |case| {
        !case.ticks().is_empty()
    });
    case.hand_run_refused();
}

/// With `CONDUCTOR_RUNTIME_LOOP=off` the runtime serves the API alone: it says so, and takes no
/// lock, so another process takes it.
#[test]
fn with_the_loop_off_the_runtime_serves_the_api_alone() {
    let case = Case::new("off");
    let mut runtime = case.serve(&[(conductor_cli::instance::LOOP_VARIABLE, "off")]);
    case.wait_for(&mut runtime, "word that the loop is off", |case| {
        case.said().contains("the loop is off")
    });
    std::thread::sleep(Duration::from_secs(2));
    fs::create_dir_all(case.lock().parent().expect("the run directory"))
        .expect("create the run directory");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(case.lock())
        .expect("open the lock");
    lock.try_lock().expect("no loop holds the lock");
    assert!(
        !case.said().contains("the loop runs"),
        "no loop:\n{}",
        case.said()
    );
}
