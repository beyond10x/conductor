//! The build verbs (`story:tool-build-commit-report`, acceptance 2; design § 6.3): `test`,
//! `check` and `fmt` run cargo in the caller's working directory through the host ledger the
//! broker is given. Success prints one line, `ok test <n> passed`; a failure prints the failing
//! tests' names and the first line of each one's panic, at most 40 lines, and the log path. A run
//! that waits for a slot holds no other call of the broker.
//!
//! The ledger here is a fake: it runs the command as `build run` would (in the working directory,
//! its output into the log the broker names), and a case can hold it closed, as a full queue
//! would. The crate it runs on is a fixture written into the case's directory, its own workspace,
//! building into its own `target/`.

mod support;

use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use conductor_broker::{BuildEnd, BuildJob, CallOutcome, FAILURE_LINES, Ledger, Services};
use conductor_model::tool::Verb;
use serde_json::json;
use support::{Case, INSTANCE, ask, grant, lines};

/// A ledger that runs each job at once, unless a case closed it; it tells `arrived` of each job
/// it takes, and refuses every job when `refuse` names a reason.
struct FakeLedger {
    open: Mutex<bool>,
    opened: Condvar,
    arrived: Mutex<Option<Sender<BuildJob>>>,
    jobs: Mutex<Vec<BuildJob>>,
    refuse: Option<String>,
}

impl FakeLedger {
    fn new() -> Self {
        Self {
            open: Mutex::new(true),
            opened: Condvar::new(),
            arrived: Mutex::new(None),
            jobs: Mutex::new(Vec::new()),
            refuse: None,
        }
    }

    fn refusing(reason: &str) -> Self {
        Self {
            refuse: Some(reason.to_owned()),
            ..Self::new()
        }
    }

    /// Closes the ledger: each job waits until [`FakeLedger::release`]. Answers where each job
    /// that arrives is told.
    fn close(&self) -> Receiver<BuildJob> {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = false;
        let (send, receive) = mpsc::channel();
        *self.arrived.lock().unwrap_or_else(PoisonError::into_inner) = Some(send);
        receive
    }

    fn release(&self) {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.opened.notify_all();
    }

    fn jobs(&self) -> Vec<BuildJob> {
        self.jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Ledger for FakeLedger {
    fn run(&self, job: &BuildJob) -> BuildEnd {
        self.jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(job.clone());
        if let Some(reason) = &self.refuse {
            return BuildEnd::Refused {
                reason: reason.clone(),
            };
        }
        if let Some(arrived) = &*self.arrived.lock().unwrap_or_else(PoisonError::into_inner) {
            let _ = arrived.send(job.clone());
        }
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        while !*open {
            open = self
                .opened
                .wait(open)
                .unwrap_or_else(PoisonError::into_inner);
        }
        drop(open);
        let log = File::create(&job.log).expect("create the job's log");
        let status = Command::new(&job.command[0])
            .args(&job.command[1..])
            .current_dir(&job.cwd)
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("the log twice"))
            .stderr(log)
            .status()
            .expect("run the job's command");
        BuildEnd::Ran {
            code: status.code().unwrap_or(1),
        }
    }
}

fn services(ledger: &Arc<FakeLedger>) -> Services {
    let ledger: Arc<dyn Ledger> = ledger.clone();
    Services {
        ledger,
        ..Services::none()
    }
}

const MANIFEST: &str = "\
[package]
name = \"fixture\"
version = \"0.1.0\"
edition = \"2021\"

[workspace]
";

const PASSING: &str = "\
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn first_adds() {
        assert_eq!(super::add(1, 1), 2);
    }

    #[test]
    fn second_adds() {
        assert_eq!(super::add(2, 2), 4);
    }
}
";

const FAILING: &str = "\
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn first_adds() {
        assert_eq!(super::add(1, 1), 2);
    }

    #[test]
    fn adds_wrong() {
        assert_eq!(super::add(1, 1), 3, \"arithmetic\");
    }
}
";

/// A fixture crate named `fixture` in the case's directory `name`, its library `lib`.
fn fixture(case: &Case, name: &str, lib: &str) -> std::path::PathBuf {
    case.workdir(name, &[("Cargo.toml", MANIFEST), ("src/lib.rs", lib)])
}

fn log_of(text: &str) -> &Path {
    Path::new(
        text.lines()
            .last()
            .and_then(|line| line.strip_prefix("log: "))
            .expect("the last line names the log"),
    )
}

#[test]
fn test_that_passes_prints_one_line_with_the_count() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Test]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "test", json!({}));

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(answer.text, "ok test 2 passed\n");
    let jobs = ledger.jobs();
    assert_eq!(jobs.len(), 1, "one run through the ledger");
    assert_eq!(&jobs[0].command[..2], ["cargo", "test"]);
    assert_eq!(jobs[0].cwd, dir);
    assert_eq!(jobs[0].session, "example-org-a");
    assert_eq!(jobs[0].instance, INSTANCE);
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].verb, Verb::Test);
    assert_eq!(calls[0].outcome, CallOutcome::Done);
}

#[test]
fn test_with_a_package_and_a_filter_runs_only_those() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Test]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(
        &socket,
        "test",
        json!({"package": "fixture", "filter": "second"}),
    );

    assert_eq!(answer.text, "ok test 1 passed\n");
    let command = &ledger.jobs()[0].command;
    assert!(
        command.windows(2).any(|pair| pair == ["-p", "fixture"]),
        "{command:?}"
    );
    assert_eq!(command.last().map(String::as_str), Some("second"));
}

#[test]
fn test_that_fails_names_the_failing_test_its_assertion_and_the_log() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", FAILING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Test]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "test", json!({}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    let shown = lines(&answer);
    assert!(shown.len() <= FAILURE_LINES, "{}", answer.text);
    assert_eq!(shown[0], "BuildFailed: test: 1 failed, 1 passed");
    assert!(
        shown.iter().any(|line| line.contains("tests::adds_wrong")),
        "{}",
        answer.text
    );
    assert!(
        shown
            .iter()
            .any(|line| line.contains("assertion `left == right` failed: arithmetic")),
        "{}",
        answer.text
    );
    assert!(
        !shown.iter().any(|line| line.contains("first_adds")),
        "a passing test is not named: {}",
        answer.text
    );
    let log = fs::read_to_string(log_of(&answer.text)).expect("read the log");
    assert!(log.contains("test result: FAILED"), "{log}");
    let calls = case.recorder.calls();
    assert_eq!(calls[0].outcome, CallOutcome::Failed);
}

#[test]
fn a_test_that_does_not_compile_names_the_compiler_error() {
    let mut case = Case::new();
    let dir = fixture(
        &case,
        "a",
        "pub fn add(a: i64) -> i64 {\n    a + missing\n}\n",
    );
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Test]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "test", json!({}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    let shown = lines(&answer);
    assert!(shown.len() <= FAILURE_LINES, "{}", answer.text);
    assert!(
        shown[0].starts_with("BuildFailed: test: "),
        "{}",
        answer.text
    );
    assert!(
        shown
            .iter()
            .any(|line| line.contains("cannot find value `missing`")),
        "{}",
        answer.text
    );
    assert!(
        shown.iter().any(|line| line.contains("src/lib.rs:2")),
        "{}",
        answer.text
    );
}

#[test]
fn a_test_that_waits_for_a_slot_holds_no_other_call() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    fs::write(dir.join("notes.txt"), "alpha\n").expect("write a file to read");
    let socket = case.register(
        "example-org-a",
        INSTANCE,
        &dir,
        grant(&[Verb::Test, Verb::Show]),
    );
    let ledger = Arc::new(FakeLedger::new());
    let arrived = ledger.close();
    case.serve_with(services(&ledger));

    let waiting = {
        let socket = socket.clone();
        thread::spawn(move || ask(&socket, "test", json!({})))
    };
    let job = arrived
        .recv_timeout(Duration::from_secs(60))
        .expect("the test reaches the ledger");
    let read = ask(&socket, "show", json!({"path": "notes.txt"}));
    let still_waiting = !waiting.is_finished();
    ledger.release();
    let answer = waiting.join().expect("the waiting call ends");

    assert_eq!(job.cwd, dir);
    assert_eq!(read.exit, 0, "{}", read.text);
    assert!(read.text.starts_with("alpha\n"), "{}", read.text);
    assert!(still_waiting, "the test waited for its slot");
    assert_eq!(answer.text, "ok test 2 passed\n");
}

#[test]
fn a_build_the_ledger_refuses_fails_with_its_reason_and_runs_nothing() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Test]));
    let ledger = Arc::new(FakeLedger::refusing("12G short of the floor"));
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "test", json!({}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(
        lines(&answer)[0],
        "BuildFailed: test: the host ledger refused it: 12G short of the floor"
    );
    assert!(
        lines(&answer)
            .last()
            .is_some_and(|line| line.starts_with("log: "))
    );
    assert!(!dir.join("target").exists(), "nothing was built");
}

#[test]
fn check_names_each_lint_with_its_place() {
    let mut case = Case::new();
    let dir = fixture(
        &case,
        "a",
        "pub fn add(a: i64, b: i64) -> i64 {\n    let unused = 1;\n    a + b\n}\n",
    );
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Check]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "check", json!({}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    let shown = lines(&answer);
    assert!(
        shown[0].starts_with("BuildFailed: check: "),
        "{}",
        answer.text
    );
    assert!(
        shown
            .iter()
            .any(|line| line.contains("unused variable") && line.contains("src/lib.rs:2")),
        "{}",
        answer.text
    );
    let command = &ledger.jobs()[0].command;
    assert_eq!(&command[..2], ["cargo", "clippy"]);
    assert!(
        command.windows(2).any(|pair| pair == ["-D", "warnings"]),
        "{command:?}"
    );
}

#[test]
fn check_of_a_clean_crate_prints_ok() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Check]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "check", json!({}));

    assert_eq!(answer.text, "ok check\n");
}

#[test]
fn fmt_formats_and_counts_the_files_it_changed() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", "pub fn add(a:i64,b:i64)->i64{a+b}\n");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Fmt]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "fmt", json!({}));
    let again = ask(&socket, "fmt", json!({}));

    assert_eq!(answer.text, "ok fmt 1 files changed\n");
    assert_eq!(again.text, "ok fmt 0 files changed\n");
    assert_eq!(
        fs::read_to_string(dir.join("src/lib.rs")).expect("read the formatted file"),
        "pub fn add(a: i64, b: i64) -> i64 {\n    a + b\n}\n"
    );
    assert_eq!(&ledger.jobs()[0].command[..2], ["cargo", "fmt"]);
}

#[test]
fn a_build_verb_outside_the_grant_is_refused_and_never_reaches_the_ledger() {
    let mut case = Case::new();
    let dir = fixture(&case, "a", PASSING);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Show]));
    let ledger = Arc::new(FakeLedger::new());
    case.serve_with(services(&ledger));

    let answer = ask(&socket, "test", json!({}));

    assert_eq!(answer.text, "NotGranted: tools: test\n");
    assert!(ledger.jobs().is_empty());
    assert_eq!(case.recorder.calls()[0].outcome, CallOutcome::NotGranted);
}
