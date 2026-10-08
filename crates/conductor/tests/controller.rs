//! `story:controller-commands`: the `conductor controller` group records each controller's
//! lifecycle in the store, one running or paused controller per repository, and `controller
//! controllers` answers from those records.
//!
//! Each case runs the built binary from its own working directory under this test target's
//! temporary directory in `target/`, with `--state-dir` naming a state directory beside it, as
//! `tests/state_dir.rs` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// One case: the directory the binary runs from, and the state directory `--state-dir` names.
struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("controller")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        let work = root.join("work");
        fs::create_dir_all(&work).expect("create the working directory");
        Self {
            work,
            state: root.join("state"),
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    fn start(&self, repository: &str) -> Output {
        self.run(&[
            "controller",
            "start-controller",
            "--repository",
            repository,
            "--harness",
            "Claude",
        ])
    }

    /// Runs `controller <command> --controller-id <id>` and any further arguments.
    fn steer(&self, command: &str, id: &str, more: &[&str]) -> Output {
        let mut args = vec!["controller", command, "--controller-id", id];
        args.extend_from_slice(more);
        self.run(&args)
    }

    /// The rows of `controller controllers --format json`.
    fn controllers(&self) -> Value {
        let output = self.run(&["controller", "controllers", "--format", "json"]);
        let text = succeeded(&output, "`controller controllers --format json`");
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("the view printed {text:?}: {error}"))
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Standard output of a command that succeeded.
fn succeeded(output: &Output, what: &str) -> String {
    assert!(output.status.success(), "{what}: {}", describe(output));
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

/// The controller id a successful `start-controller` printed: one UUID on one line.
fn started(output: &Output, what: &str) -> String {
    let text = succeeded(output, what);
    let id = text
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("{what} printed {text:?}, not one line"));
    let groups: Vec<usize> = id.split('-').map(str::len).collect();
    assert!(
        groups == [8, 4, 4, 4, 12] && id.bytes().all(|b| b == b'-' || b.is_ascii_hexdigit()),
        "{what} printed {id:?}, not a controller id"
    );
    id.to_owned()
}

/// A command that succeeded and printed nothing.
fn quiet(output: &Output, what: &str) {
    assert_eq!(succeeded(output, what), "", "{what} prints nothing");
}

/// A command refused with the declared `error`: exit 1, nothing on standard output, and one line
/// on standard error that names the error.
fn refused(output: &Output, what: &str, error: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && output.stdout.is_empty()
            && stderr.lines().count() == 1
            && stderr.contains(error),
        "{what} is refused with {error}: {}",
        describe(output)
    );
}

/// A `Controllers` row of a controller started for `repository` with harness `Claude`.
fn row(id: &str, state: &str, repository: &str, charter_revision: i64) -> Value {
    json!({
        "controller_id": id,
        "state": state,
        "repository": repository,
        "harness": "Claude",
        "session_name": repository,
        "charter_revision": charter_revision,
    })
}

#[test]
fn one_controller_runs_per_repository_through_pause_resume_revise_and_stop() {
    let case = Case::new("lifecycle");

    // 1. Start `x`.
    let first = started(&case.start("x"), "start x");
    assert_eq!(case.controllers(), json!([row(&first, "Running", "x", 1)]));

    // 2. Starting `x` again is refused with `ControllerAlreadyRunning`, and nothing starts.
    refused(
        &case.start("x"),
        "a second start of x",
        "ControllerAlreadyRunning",
    );
    assert_eq!(case.controllers(), json!([row(&first, "Running", "x", 1)]));

    // 3. Pause `x`; starting `x` again is still refused.
    quiet(&case.steer("pause-controller", &first, &[]), "pause x");
    assert_eq!(case.controllers(), json!([row(&first, "Paused", "x", 1)]));
    refused(
        &case.start("x"),
        "a start of paused x",
        "ControllerAlreadyRunning",
    );

    // 4. Resume `x`, then revise its charter twice: revision 3, Running.
    quiet(&case.steer("resume-controller", &first, &[]), "resume x");
    quiet(&case.steer("revise-charter", &first, &[]), "revise x once");
    quiet(&case.steer("revise-charter", &first, &[]), "revise x twice");
    assert_eq!(case.controllers(), json!([row(&first, "Running", "x", 3)]));

    // 5. Stop `x`; starting `x` now succeeds, as a new controller.
    quiet(
        &case.steer("stop-controller", &first, &["--reason", "the wave closed"]),
        "stop x",
    );
    let second = started(&case.start("x"), "start x after the stop");
    assert_ne!(first, second, "the new controller has an id of its own");
    assert_eq!(
        case.controllers(),
        json!([
            row(&first, "Stopped", "x", 3),
            row(&second, "Running", "x", 1)
        ])
    );

    // 6. Revising the charter of the stopped controller is refused with
    //    `ControllerStateConflict`, and nothing moves.
    refused(
        &case.steer("revise-charter", &first, &[]),
        "a revision of stopped x",
        "ControllerStateConflict",
    );
    assert_eq!(
        case.controllers(),
        json!([
            row(&first, "Stopped", "x", 3),
            row(&second, "Running", "x", 1)
        ])
    );
}

#[test]
fn a_running_controller_does_not_hold_another_repository() {
    let case = Case::new("two-repositories");
    let x = started(&case.start("x"), "start x");
    let y = started(&case.start("y"), "start y while x runs");
    assert_eq!(
        case.controllers(),
        json!([row(&x, "Running", "x", 1), row(&y, "Running", "y", 1)])
    );
}

#[test]
fn steering_an_unknown_controller_is_refused_with_controller_not_found() {
    let case = Case::new("unknown");
    let x = started(&case.start("x"), "start x");
    let unknown = "123e4567-e89b-12d3-a456-426614174000";
    for (command, more) in [
        ("pause-controller", &[][..]),
        ("resume-controller", &[][..]),
        ("revise-charter", &[][..]),
        ("stop-controller", &["--reason", "none"][..]),
    ] {
        refused(
            &case.steer(command, unknown, more),
            &format!("`{command}` of an unknown id"),
            "ControllerNotFound",
        );
    }
    assert_eq!(case.controllers(), json!([row(&x, "Running", "x", 1)]));
}

/// The names in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| {
            entry
                .expect("read an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn a_start_keeps_its_record_under_the_state_dir_flag_and_nothing_in_the_working_directory() {
    let case = Case::new("flag");
    started(&case.start("x"), "start x");
    assert!(
        case.state.join("tree").is_dir(),
        "the store is created under the --state-dir directory"
    );
    assert_eq!(
        entries(&case.work),
        Vec::<String>::new(),
        "nothing is created in the working directory"
    );
}
