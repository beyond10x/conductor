//! Wave 06's follow-up, run as the built binary against the store under `--state-dir`:
//! `decision show`, a first line that names no message kind, and the `snapshot specifications`
//! and `snapshot record-specification` leaves.
//!
//! Each case has its own directory under this test target's temporary directory in `target/`: the
//! binary runs from `work/` in it and keeps its records in `state/` beside it, and every run leaves
//! `work/` empty. A refused command exits 1 with one line on standard error and writes nothing on
//! standard output, unless the case says otherwise.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder};
use conductor_model::observation::SnapshotState;
use serde_json::{Value, json};

/// The instant the decisions here are decided at.
const AT: &str = "2026-10-07T10:00:00Z";

/// One case's directories: where the binary runs, and the state directory it is pointed at.
struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("followup_w06")
            .join(case);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        let work = root.join("work");
        fs::create_dir_all(&work).expect("create the case's working directory");
        Self {
            work,
            state: root.join("state"),
        }
    }

    /// Runs `conductor --state-dir <state> <args>` from the case's working directory, and checks
    /// that it left the working directory empty.
    fn run(&self, args: &[&str]) -> Output {
        run(&self.work, &self.state, args)
    }

    /// Runs a command that must succeed, and answers its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}`: {}",
            args.join(" "),
            describe(&output)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// Runs a command that must fail: exit 1, nothing on standard output, and one line on
    /// standard error that starts with `conductor: <group> <command>: ` and holds `needle`.
    fn refused(&self, args: &[&str], needle: &str) -> String {
        let output = self.run(args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let command = format!("conductor: {} {}: ", args[0], args[1]);
        assert!(
            output.status.code() == Some(1)
                && output.stdout.is_empty()
                && stderr.starts_with(&command)
                && stderr.contains(needle)
                && stderr.ends_with('\n')
                && stderr.lines().count() == 1,
            "`conductor {}` should be refused naming {needle:?}: {}",
            args.join(" "),
            describe(&output)
        );
        stderr
    }

    /// The rows of `conductor <group> <view> --format jsonl`.
    fn rows(&self, group: &str, view: &str) -> Vec<Value> {
        self.ok(&[group, view, "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
            .collect()
    }

    /// Records a decision with `who`'s command, the id given, and `question`.
    fn record(&self, who: &str, decision: &str, class: &str, question: &str) {
        let printed = self.ok(&[
            "decision",
            who,
            "--decision-id",
            decision,
            "--decision-class",
            class,
            "--question",
            question,
            "--options",
            "A: the file store; B: a database",
            "--choice",
            "A",
            "--reason",
            "the file store is enough",
            "--evidence",
            "the pilot holds 200 records",
            "--decided-at",
            AT,
        ]);
        assert_eq!(printed, format!("{decision}\n"));
    }

    /// The arguments of `message receive-message` for a message from cortex with `first_line`,
    /// and `extra` flags.
    fn receive_args<'a>(first_line: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            "message",
            "receive-message",
            "--sender",
            "cortex",
            "--recipient",
            "conductor",
            "--first-line",
            first_line,
            "--transport",
            "SendMessage",
            "--received-at",
            "2026-10-07T09:05:00Z",
        ];
        args.extend_from_slice(extra);
        args
    }
}

/// Runs `conductor --state-dir <state> <args>` from `work`, and checks that it left `work` empty.
fn run(work: &Path, state: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
        .current_dir(work)
        .arg("--state-dir")
        .arg(state)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert_eq!(
        fs::read_dir(work).expect("read work/").count(),
        0,
        "`conductor {}` planted something in its working directory: {}",
        args.join(" "),
        describe(&output)
    );
    output
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---------------------------------------------------------------------------------------------
// decision show
// ---------------------------------------------------------------------------------------------

/// `decision show --decision-id <id>` renders that decision's row of the `decisions` view: one
/// `field: value` line each, in the order id, state, class, decided by, decided at, question,
/// options, choice, reason, evidence, with a line break inside a value written as a space; and
/// `--format json` gives one object with the view's values.
#[test]
fn decision_show_renders_one_decision_as_field_lines_or_one_json_object() {
    let case = Case::new("show");
    case.record(
        "record-conductor-decision",
        "DEC-20261007-01",
        "C",
        "Which store\nbacks the pilot?",
    );
    case.record(
        "record-operator-decision",
        "DEC-20261007-02",
        "O",
        "Spend on a database?",
    );

    let expected = "\
decision_id: DEC-20261007-01
state: InForce
decision_class: C
decided_by: Conductor
decided_at: 2026-10-07T10:00:00Z
question: Which store backs the pilot?
options: A: the file store; B: a database
choice: A
reason: the file store is enough
evidence: the pilot holds 200 records
";
    assert_eq!(
        case.ok(&["decision", "show", "--decision-id", "DEC-20261007-01"]),
        expected
    );
    assert_eq!(
        case.ok(&[
            "decision",
            "show",
            "--decision-id",
            "DEC-20261007-01",
            "--format",
            "text"
        ]),
        expected,
        "text is the default"
    );

    let printed = case.ok(&[
        "decision",
        "show",
        "--decision-id",
        "DEC-20261007-02",
        "--format",
        "json",
    ]);
    assert_eq!(
        printed.matches('\n').count(),
        1,
        "one object on one line: {printed:?}"
    );
    let object: Value = serde_json::from_str(&printed).expect("one JSON object");
    let keys: Vec<&str> = object
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    let mut sorted = vec![
        "decision_id",
        "state",
        "decision_class",
        "decided_by",
        "decided_at",
        "question",
        "options",
        "choice",
        "reason",
        "evidence",
    ];
    sorted.sort_unstable();
    assert_eq!(keys, sorted, "the object holds exactly the ten fields");
    let row = case
        .rows("decision", "decisions")
        .into_iter()
        .find(|row| row["decision_id"] == "DEC-20261007-02")
        .expect("the decisions view lists it");
    assert_eq!(object, row, "the object is the decision's row of the view");
    assert_eq!(object["decided_by"], "Operator");

    let order: Vec<&str> = printed
        .trim_start_matches('{')
        .split(",\"")
        .map(|field| {
            field
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap_or("")
        })
        .collect();
    assert_eq!(
        order,
        [
            "decision_id",
            "state",
            "decision_class",
            "decided_by",
            "decided_at",
            "question",
            "options",
            "choice",
            "reason",
            "evidence"
        ],
        "the object's keys are in the text's order: {printed}"
    );

    case.ok(&[
        "decision",
        "reverse-operator-decision",
        "--decision-id",
        "DEC-20261007-02",
        "--reason",
        "the pilot stays small",
    ]);
    let shown = case.ok(&["decision", "show", "--decision-id", "DEC-20261007-02"]);
    assert!(
        shown.lines().nth(1) == Some("state: Reversed"),
        "a reversed decision shows its state: {shown:?}"
    );
}

/// An id no decision has, a format other than text or json, and a missing `--decision-id` each
/// exit 1 with one line naming what is wrong. With no store, `decision show` answers as a view
/// does, naming the store it did not find, and creates none.
#[test]
fn decision_show_refuses_an_unknown_id_a_format_and_a_missing_flag_and_creates_no_store() {
    let case = Case::new("show-refused");
    let line = case.refused(
        &["decision", "show", "--decision-id", "DEC-20261007-09"],
        "no store",
    );
    assert!(
        !case.state.exists(),
        "decision show created {}: {line:?}",
        case.state.display()
    );

    case.record(
        "record-conductor-decision",
        "DEC-20261007-01",
        "C",
        "Which store?",
    );
    case.refused(
        &["decision", "show", "--decision-id", "DEC-20261007-09"],
        "DEC-20261007-09",
    );
    case.refused(
        &[
            "decision",
            "show",
            "--decision-id",
            "DEC-20261007-01",
            "--format",
            "yaml",
        ],
        "yaml",
    );
    case.refused(&["decision", "show"], "--decision-id");
}

// ---------------------------------------------------------------------------------------------
// A first line that names no kind
// ---------------------------------------------------------------------------------------------

/// `hello` names no kind: with no `--kind` it is received as kind `Unknown`, recorded straight
/// into `Rejected`, the format goes to standard output, and the command exits 1 naming the message.
/// `--kind Unknown` gives the same; `--kind Unknown` on a line whose tag names a kind is refused,
/// and nothing is written.
#[test]
fn a_first_line_that_names_no_kind_is_received_as_unknown_and_rejected() {
    let case = Case::new("unknown-kind");
    for (line, extra) in [
        ("hello", &[][..]),
        ("hello again", &["--kind", "Unknown"][..]),
    ] {
        let output = case.run(&Case::receive_args(line, extra));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{line:?}: {}",
            describe(&output)
        );
        assert!(
            stdout.starts_with("The first line of a message to conductor is one of:\n")
                && stdout.contains("[NEED <repo> <request-id>] <other-repo> <what is needed>"),
            "{line:?}: the format goes back to the sender: {stdout:?}"
        );
        let rows = case.rows("message", "messages");
        let row = rows
            .iter()
            .find(|row| row["first_line"] == line)
            .unwrap_or_else(|| panic!("{line:?} is recorded: {rows:?}"));
        assert_eq!(
            (row["kind"].as_str(), row["state"].as_str()),
            (Some("Unknown"), Some("Rejected")),
            "{line:?}: {row}"
        );
        let id = row["message_id"].as_str().expect("an id");
        assert!(
            stderr.starts_with("conductor: message receive-message: ")
                && stderr.contains(id)
                && stderr.lines().count() == 1,
            "{line:?}: standard error names the rejected message: {stderr:?}"
        );
    }

    let before = case.rows("message", "messages");
    case.refused(
        &Case::receive_args(
            "[NEED cortex N-26-10] delta list it",
            &["--kind", "Unknown"],
        ),
        "--kind",
    );
    case.refused(
        &Case::receive_args("hello", &["--parsed", "true"]),
        "--parsed",
    );
    assert_eq!(
        case.rows("message", "messages"),
        before,
        "the refused receipts wrote nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// snapshot specifications and snapshot record-specification
// ---------------------------------------------------------------------------------------------

/// The arguments of `snapshot record-specification` for `repository` into `snapshot`.
fn record_specification(snapshot: &str, repository: &str) -> Vec<String> {
    [
        "snapshot",
        "record-specification",
        "--snapshot-id",
        snapshot,
        "--repository",
        repository,
        "--presence",
        "Present",
        "--path",
        "spec",
        "--format",
        "ess/22",
        "--required-ess",
        "ess 0.55",
        "--validation",
        "Valid",
        "--validation-refusals",
        "0",
        "--scenarios",
        "272",
        "--synthesis-refusals",
        "0",
        "--conformance-status",
        "validated",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// `snapshot record-specification` records into the collecting snapshot of the store under
/// `--state-dir` and prints the observation's id; `snapshot specifications` lists it there.
/// Recording into the snapshot once it completed is refused `SnapshotNotCollecting`, and into an
/// id no snapshot has `SnapshotNotFound`.
#[test]
fn snapshot_specifications_and_record_specification_use_the_state_dir() {
    let case = Case::new("specifications");
    let record = |snapshot: &str, repository: &str| -> Output {
        let args = record_specification(snapshot, repository);
        case.run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    };

    // The binary records while the library holds the snapshot open: a collector runs it.
    let ran: Arc<Mutex<Option<Output>>> = Arc::default();
    let probe = {
        let (work, state, ran) = (case.work.clone(), case.state.clone(), Arc::clone(&ran));
        Collector::new("binary", move |record_into: &mut Recorder<'_>| {
            let snapshot = record_into.snapshot().0.0.clone();
            let output = run(
                &work,
                &state,
                &record_specification(&snapshot, "conductor")
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            );
            *ran.lock().expect("the probe's slot") = Some(output);
            Ok(())
        })
    };
    let taken = snapshot::take(
        conductor_cli::store::open(&case.state).expect("open the store"),
        &[probe],
    )
    .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let output = ran
        .lock()
        .expect("the probe's slot")
        .take()
        .expect("the probe ran");
    assert!(output.status.success(), "{}", describe(&output));
    let printed = String::from_utf8(output.stdout).expect("UTF-8");
    let observation = printed.trim_end();
    assert_eq!(
        observation.len(),
        36,
        "prints the observation's id: {printed:?}"
    );

    let snapshot = taken.snapshot_id.0.0.as_str();
    assert_eq!(
        case.rows("snapshot", "specifications"),
        vec![json!({
            "observation_id": observation,
            "snapshot_id": snapshot,
            "repository": "conductor",
            "presence": "Present",
            "path": "spec",
            "format": "ess/22",
            "required_ess": "ess 0.55",
            "validation": "Valid",
            "validation_refusals": 0,
            "scenarios": 272,
            "synthesis_refusals": 0,
            "conformance_status": "validated",
        })]
    );

    let closed = record(snapshot, "delta");
    assert_eq!(closed.status.code(), Some(1), "{}", describe(&closed));
    assert!(
        String::from_utf8_lossy(&closed.stderr).contains("SnapshotNotCollecting"),
        "{}",
        describe(&closed)
    );
    let unknown = record("01890a5d-ac96-774b-bcce-b302099a8057", "delta");
    assert_eq!(unknown.status.code(), Some(1), "{}", describe(&unknown));
    assert!(
        String::from_utf8_lossy(&unknown.stderr).contains("SnapshotNotFound"),
        "{}",
        describe(&unknown)
    );
    assert_eq!(
        case.rows("snapshot", "specifications").len(),
        1,
        "the refused records wrote nothing"
    );
}

/// `snapshot specifications` with no store answers as every view does: exit 1, one line naming
/// the store it did not find, and nothing created.
#[test]
fn snapshot_specifications_creates_no_store() {
    let case = Case::new("specifications-no-store");
    let output = case.run(&["snapshot", "specifications"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no store"),
        "{}",
        describe(&output)
    );
    assert!(
        !case.state.exists(),
        "the view created {}",
        case.state.display()
    );
}

// ---------------------------------------------------------------------------------------------
// The state directory reaches the collectors through the recorder
// ---------------------------------------------------------------------------------------------

/// A snapshot taken in a state directory (`snapshot::take_in`) keeps its store there and gives
/// that directory to every collector through `Recorder::state`. A snapshot taken over a store
/// opened elsewhere has no directory to give: a collector that asks for it fails the snapshot,
/// naming what is missing.
#[test]
fn a_collector_reads_the_state_directory_through_its_recorder() {
    let case = Case::new("recorder-state");
    let seen: Arc<Mutex<Option<PathBuf>>> = Arc::default();
    let probe = {
        let seen = Arc::clone(&seen);
        Collector::new("state", move |record: &mut Recorder<'_>| {
            *seen.lock().expect("the probe's slot") = Some(record.state()?.to_owned());
            Ok(())
        })
    };
    let taken = snapshot::take_in(&case.state, &[probe]).expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    assert_eq!(
        seen.lock().expect("the probe's slot").as_deref(),
        Some(case.state.as_path())
    );
    assert!(
        case.state.join("tree").is_dir(),
        "the store is kept under the state directory"
    );

    let elsewhere = Case::new("recorder-no-state");
    let asks = Collector::new("asks", |record: &mut Recorder<'_>| {
        record.state().map(|_| ())
    });
    let taken = snapshot::take(
        conductor_cli::store::open(&elsewhere.state).expect("open the store"),
        &[asks],
    )
    .expect("the driver ends the snapshot");
    assert_eq!(taken.state, SnapshotState::Failed);
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(
        reason.starts_with("asks: ") && reason.contains("state directory"),
        "{reason}"
    );
}
