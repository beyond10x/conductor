//! `story:controller-commands`: the `conductor controller` group records each controller's
//! lifecycle in the store, one running or paused controller per repository, and `controller
//! controllers` answers from those records.
//!
//! Each case runs the built binary from its own working directory under this test target's
//! temporary directory in `target/`, with `--state-dir` naming a state directory beside it, as
//! `tests/state_dir.rs` does.

mod common;

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

    /// Runs the binary from the case's working directory with `HOME` naming the case's own home,
    /// so no case reads the operator's config, and neither config variable set.
    fn run(&self, args: &[&str]) -> Output {
        self.command(args)
            .output()
            .expect("the conductor binary runs")
    }

    /// [`Case::run`], with `CONDUCTOR_CONFIG` naming `config`.
    fn run_with(&self, config: &Path, args: &[&str]) -> Output {
        self.command(args)
            .env("CONDUCTOR_CONFIG", config)
            .output()
            .expect("the conductor binary runs")
    }

    fn command(&self, args: &[&str]) -> Command {
        let home = self.work.with_file_name("home");
        fs::create_dir_all(&home).expect("create the case's home");
        let mut command = common::conductor(home);
        command
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null());
        command
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

/// `story:instance-session-names`: a controller's `session_name` is `<session_prefix>-<repository>`
/// of the instance the binary runs, or the repository without a prefix; a `--session-name` that is
/// not that name is refused and records nothing.
#[test]
fn a_controller_s_session_name_carries_the_instance_s_prefix() {
    let case = Case::new("session-prefix");
    let config = case.work.with_file_name("conductor.yaml");
    fs::write(
        &config,
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   session_prefix: a\n\
         \x20   sources: [{github: alpha}]\n\
         \x20   checkouts: {root: /srv/alpha, trees: /srv/alpha-trees}\n",
    )
    .expect("write the config file");
    let start = |more: &[&str]| {
        let mut args = vec![
            "controller",
            "start-controller",
            "--repository",
            "x",
            "--harness",
            "Claude",
        ];
        args.extend_from_slice(more);
        case.run_with(&config, &args)
    };
    let refused_name = start(&["--session-name", "x"]);
    assert!(
        refused_name.status.code() == Some(1)
            && refused_name.stdout.is_empty()
            && String::from_utf8_lossy(&refused_name.stderr).contains("a-x"),
        "a session name other than a-x is refused, naming a-x: {}",
        describe(&refused_name)
    );
    let id = started(&start(&[]), "start x");
    let given = case.run_with(
        &config,
        &[
            "controller",
            "start-controller",
            "--repository",
            "y",
            "--harness",
            "Claude",
            "--session-name",
            "a-y",
        ],
    );
    let other = started(&given, "start y as a-y");
    let output = case.run_with(&config, &["controller", "controllers", "--format", "json"]);
    let rows: Value = serde_json::from_str(&succeeded(&output, "controllers")).expect("JSON rows");
    let mut x = row(&id, "Running", "x", 1);
    x["session_name"] = json!("a-x");
    let mut y = row(&other, "Running", "y", 1);
    y["session_name"] = json!("a-y");
    let mut rows = rows.as_array().expect("a list of rows").clone();
    rows.sort_by_key(|row| row["repository"].as_str().map(str::to_owned));
    assert_eq!(rows, [x, y]);

    let bare = Case::new("session-prefix-none");
    let id = started(&bare.start("x"), "start x without a config file");
    assert_eq!(bare.controllers(), json!([row(&id, "Running", "x", 1)]));
}
