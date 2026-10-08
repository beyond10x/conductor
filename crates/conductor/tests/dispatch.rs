//! `story:decision-commands`, the `conductor dispatch` and `conductor message` groups, run as the
//! built binary against the store under `--state-dir`.
//!
//! Each case has its own directory under this test target's temporary directory in `target/`: the
//! binary runs from `work/` in it and keeps its records in `state/` beside it. A command that is
//! refused exits 1 with one line on standard error naming the error the specification declares,
//! and writes nothing on standard output.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;

use serde_json::{Value, json};

/// The send instant every dispatch of a case carries unless it says otherwise.
const SENT_AT: &str = "2026-10-07T09:00:00Z";

/// The receipt instant every message of a case carries.
const RECEIVED_AT: &str = "2026-10-07T09:05:00Z";

/// One case's directories: where the binary runs, and the state directory it is pointed at.
struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dispatch")
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

    /// Runs `conductor --state-dir <state> <args>` from the case's working directory.
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

    /// Every command and view the case ran honoured `--state-dir`: the store is under it, and the
    /// working directory, where `state/` is looked for without the flag, holds nothing.
    fn kept_under_the_state_dir(&self) {
        assert!(
            self.state.join("tree").is_dir(),
            "no store under --state-dir {}",
            self.state.display()
        );
        let planted: Vec<PathBuf> = fs::read_dir(&self.work)
            .expect("read the working directory")
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        assert!(
            planted.is_empty(),
            "a handler ignored --state-dir and wrote under the working directory: {planted:?}"
        );
    }

    /// Runs a command that must succeed, and answers its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}` exited {:?}; stderr: {}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// Runs a command that must succeed and print one line, and answers that line.
    fn printed(&self, args: &[&str]) -> String {
        let printed = self.ok(args);
        let line = printed.strip_suffix('\n').unwrap_or_else(|| {
            panic!(
                "`conductor {}` prints one line, got {printed:?}",
                args.join(" ")
            )
        });
        assert!(
            !line.is_empty() && !line.contains('\n'),
            "`conductor {}` prints one line, got {printed:?}",
            args.join(" ")
        );
        line.to_owned()
    }

    /// Runs a command that is refused: exit 1, nothing on standard output, and one line on
    /// standard error that starts with `conductor: <group> <command>: ` and contains `needle`.
    /// Answers that line.
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
            "`conductor {}` should be refused naming {needle:?}: exit 1, no stdout, one stderr \
             line starting {command:?}; got exit {:?}, stdout {:?}, stderr {stderr:?}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
        );
        stderr
    }

    /// Runs a command that the specification refuses with `error`.
    fn declined(&self, args: &[&str], error: &str) {
        let line = self.refused(args, error);
        let command = format!("conductor: {} {}: {error}: ", args[0], args[1]);
        assert!(
            line.starts_with(&command),
            "`conductor {}` names {error} first, got {line:?}",
            args.join(" ")
        );
    }

    /// The rows of a view, read as `--format jsonl`.
    fn rows(&self, group: &str, view: &str) -> Vec<Value> {
        self.ok(&[group, view, "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
            .collect()
    }

    /// The row the `dispatches` view gives `dispatch`.
    fn dispatch(&self, dispatch: &str) -> Value {
        let rows = self.rows("dispatch", "dispatches");
        rows.iter()
            .find(|row| row["dispatch_id"] == dispatch)
            .unwrap_or_else(|| panic!("the dispatches view has no row for {dispatch}: {rows:?}"))
            .clone()
    }

    /// The row the `messages` view gives `message`.
    fn message(&self, message: &str) -> Value {
        let rows = self.rows("message", "messages");
        rows.iter()
            .find(|row| row["message_id"] == message)
            .unwrap_or_else(|| panic!("the messages view has no row for {message}: {rows:?}"))
            .clone()
    }

    fn propose(&self, goal: &str) {
        self.ok(&[
            "goal",
            "propose-goal",
            "--goal-id",
            goal,
            "--title",
            "Ship the pilot",
            "--exit-evidence",
            &format!("{goal} is listed as met in NORTHSTAR.md"),
        ]);
    }

    /// Sends a dispatch against `goal` for `repository` at `sent_at`, with `extra` flags, and
    /// answers the id the command prints.
    fn send(&self, repository: &str, goal: &str, sent_at: &str, extra: &[&str]) -> String {
        let mut args = vec![
            "dispatch",
            "send-dispatch",
            "--repository",
            repository,
            "--goal-id",
            goal,
            "--brief",
            "Expose the store id",
            "--sent-at",
            sent_at,
            "--priority",
            "1",
        ];
        args.extend_from_slice(extra);
        self.printed(&args)
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
            RECEIVED_AT,
        ];
        args.extend_from_slice(extra);
        args
    }

    /// Receives a message whose first line parses, and answers the message id it prints.
    fn receive(&self, first_line: &str) -> String {
        let id = self.printed(&Self::receive_args(first_line, &[]));
        assert!(
            is_uuid(&id),
            "receive-message prints the message's id, a UUID; got {id:?}"
        );
        id
    }
}

/// Whether `text` is a UUID in its canonical lower-case rendering.
fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

/// Dispatch lifecycle: a dispatch sent against G9 moves sent → started → blocked → started →
/// done. A second dispatch is cancelled. A dispatch against an unknown goal is refused with
/// `GoalNotFound`, and nothing is written.
#[test]
fn a_dispatch_against_g9_moves_sent_started_blocked_started_done_and_another_is_cancelled() {
    let case = Case::new("lifecycle");
    case.propose("G9");

    let first = case.send("cortex", "G9", SENT_AT, &[]);
    assert_eq!(first, "DSP-20261007-01", "the first dispatch of the day");
    assert_eq!(
        case.dispatch(&first),
        json!({
            "dispatch_id": "DSP-20261007-01",
            "state": "Sent",
            "repository": "cortex",
            "goal_id": "G9",
            "brief": "Expose the store id",
            "sent_at": SENT_AT,
            "priority": 1,
            "waits_on": null,
        })
    );

    case.ok(&["dispatch", "report-started", "--dispatch-id", &first]);
    assert_eq!(case.dispatch(&first)["state"], "Started");
    case.ok(&[
        "dispatch",
        "report-blocked",
        "--dispatch-id",
        &first,
        "--reason",
        "waits on a decision",
    ]);
    assert_eq!(case.dispatch(&first)["state"], "Blocked");
    case.ok(&["dispatch", "report-unblocked", "--dispatch-id", &first]);
    assert_eq!(case.dispatch(&first)["state"], "Started");
    case.ok(&[
        "dispatch",
        "report-done",
        "--dispatch-id",
        &first,
        "--evidence",
        "origin/main 0123abc",
    ]);
    assert_eq!(case.dispatch(&first)["state"], "Done");

    case.declined(
        &["dispatch", "report-started", "--dispatch-id", &first],
        "DispatchStateConflict",
    );
    case.declined(
        &[
            "dispatch",
            "report-failed",
            "--dispatch-id",
            &first,
            "--reason",
            "too late",
        ],
        "DispatchStateConflict",
    );

    let second = case.send("cortex", "G9", SENT_AT, &["--waits-on", &first]);
    assert_eq!(
        second, "DSP-20261007-02",
        "one more than the highest of the day"
    );
    assert_eq!(case.dispatch(&second)["waits_on"], first.as_str());
    case.ok(&[
        "dispatch",
        "cancel-dispatch",
        "--dispatch-id",
        &second,
        "--reason",
        "superseded",
    ]);
    assert_eq!(case.dispatch(&second)["state"], "Cancelled");
    case.declined(
        &[
            "dispatch",
            "cancel-dispatch",
            "--dispatch-id",
            &second,
            "--reason",
            "again",
        ],
        "DispatchStateConflict",
    );

    let before = case.rows("dispatch", "dispatches");
    case.declined(
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            "cortex",
            "--goal-id",
            "G404",
            "--brief",
            "Expose the store id",
            "--sent-at",
            SENT_AT,
            "--priority",
            "1",
        ],
        "GoalNotFound",
    );
    case.declined(
        &[
            "dispatch",
            "report-done",
            "--dispatch-id",
            "DSP-20261007-99",
            "--evidence",
            "none",
        ],
        "DispatchNotFound",
    );
    assert_eq!(
        case.rows("dispatch", "dispatches"),
        before,
        "the refused commands wrote nothing"
    );
    case.kept_under_the_state_dir();
}

/// The specification's other refusals of `send-dispatch`: a negative priority, a dispatch that
/// waits on itself, and a goal that is met.
#[test]
fn send_dispatch_refuses_a_negative_priority_a_self_wait_and_a_met_goal() {
    let case = Case::new("send-refusals");
    case.propose("G9");
    case.declined(
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            "cortex",
            "--goal-id",
            "G9",
            "--brief",
            "b",
            "--sent-at",
            SENT_AT,
            "--priority",
            "-1",
        ],
        "InvalidPriority",
    );
    case.declined(
        &[
            "dispatch",
            "send-dispatch",
            "--dispatch-id",
            "DSP-20261007-05",
            "--waits-on",
            "DSP-20261007-05",
            "--repository",
            "cortex",
            "--goal-id",
            "G9",
            "--brief",
            "b",
            "--sent-at",
            SENT_AT,
            "--priority",
            "0",
        ],
        "WaitsOnItself",
    );
    case.ok(&["goal", "confirm-goal", "--goal-id", "G9"]);
    case.ok(&[
        "goal",
        "mark-goal-met",
        "--goal-id",
        "G9",
        "--evidence",
        "met",
    ]);
    case.declined(
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            "cortex",
            "--goal-id",
            "G9",
            "--brief",
            "b",
            "--sent-at",
            SENT_AT,
            "--priority",
            "0",
        ],
        "GoalClosed",
    );
    assert_eq!(
        case.rows("dispatch", "dispatches"),
        Vec::<Value>::new(),
        "nothing was sent"
    );
}

/// DSP ids: `send-dispatch` without `--dispatch-id` allocates `DSP-YYYYMMDD-NN` from the UTC date
/// of the send instant, `NN` one more than the highest stored for that date and at least two
/// digits. An id given by hand is used as given; one that already exists is refused, naming it,
/// and nothing is written.
#[test]
fn send_dispatch_allocates_dsp_ids_per_utc_day_and_refuses_a_hand_given_id_that_exists() {
    let case = Case::new("dsp-ids");
    case.propose("G9");
    assert_eq!(case.send("cortex", "G9", SENT_AT, &[]), "DSP-20261007-01");
    assert_eq!(case.send("cortex", "G9", SENT_AT, &[]), "DSP-20261007-02");

    // 23:30 at UTC-02:00 is 01:30 UTC the next day.
    assert_eq!(
        case.send("cortex", "G9", "2026-10-07T23:30:00-02:00", &[]),
        "DSP-20261008-01",
        "the date is the UTC date of the send instant"
    );

    assert_eq!(
        case.send(
            "cortex",
            "G9",
            SENT_AT,
            &["--dispatch-id", "DSP-20261007-09"]
        ),
        "DSP-20261007-09",
        "an id given by hand is used as given"
    );
    assert_eq!(
        case.send("cortex", "G9", SENT_AT, &[]),
        "DSP-20261007-10",
        "one more than the highest number stored for that date"
    );

    let before = case.rows("dispatch", "dispatches");
    let line = case.refused(
        &[
            "dispatch",
            "send-dispatch",
            "--dispatch-id",
            "DSP-20261007-02",
            "--repository",
            "delta",
            "--goal-id",
            "G9",
            "--brief",
            "a different brief",
            "--sent-at",
            SENT_AT,
            "--priority",
            "3",
        ],
        "DSP-20261007-02",
    );
    assert!(
        line.contains("already"),
        "the refusal says the id exists: {line:?}"
    );
    assert_eq!(
        case.rows("dispatch", "dispatches"),
        before,
        "the refused dispatch wrote nothing"
    );
    assert_eq!(case.dispatch("DSP-20261007-02")["repository"], "cortex");
    case.kept_under_the_state_dir();
}

/// Four `send-dispatch` calls at once, none naming an id, each get a different one: a call that
/// finds its allocated id taken by another allocates again.
#[test]
fn concurrent_send_dispatch_calls_get_four_different_ids() {
    let case = Case::new("dsp-concurrent");
    case.propose("G9");
    let case = &case;
    let printed: Vec<String> = thread::scope(|scope| {
        let calls: Vec<_> = (0..4)
            .map(|_| scope.spawn(move || case.send("cortex", "G9", SENT_AT, &[])))
            .collect();
        calls
            .into_iter()
            .map(|call| call.join().expect("a send-dispatch call"))
            .collect()
    });
    let mut ids = printed.clone();
    ids.sort();
    assert_eq!(
        ids,
        [
            "DSP-20261007-01",
            "DSP-20261007-02",
            "DSP-20261007-03",
            "DSP-20261007-04"
        ],
        "four calls at once printed {printed:?}"
    );
    assert_eq!(case.rows("dispatch", "dispatches").len(), 4);
}

/// A dispatch event missing a required field is refused with the field's name, before any store
/// is opened.
#[test]
fn a_dispatch_event_missing_a_required_field_is_refused_with_the_fields_name() {
    let case = Case::new("missing-field");
    let full = [
        ("--repository", "cortex"),
        ("--goal-id", "G9"),
        ("--brief", "Expose the store id"),
        ("--sent-at", SENT_AT),
        ("--priority", "1"),
    ];
    for (left_out, _) in full {
        let mut args = vec!["dispatch", "send-dispatch"];
        for (flag, value) in full {
            if flag != left_out {
                args.extend([flag, value]);
            }
        }
        case.refused(&args, left_out);
    }
    case.refused(&["dispatch", "report-started"], "--dispatch-id");
    case.refused(
        &["dispatch", "report-blocked", "--dispatch-id", "DSP-1"],
        "--reason",
    );
    case.refused(&["dispatch", "report-unblocked"], "--dispatch-id");
    case.refused(
        &["dispatch", "report-done", "--dispatch-id", "DSP-1"],
        "--evidence",
    );
    case.refused(
        &["dispatch", "report-failed", "--dispatch-id", "DSP-1"],
        "--reason",
    );
    case.refused(
        &["dispatch", "cancel-dispatch", "--dispatch-id", "DSP-1"],
        "--reason",
    );
    case.refused(
        &[
            "message",
            "receive-message",
            "--sender",
            "cortex",
            "--recipient",
            "conductor",
            "--transport",
            "SendMessage",
            "--received-at",
            RECEIVED_AT,
        ],
        "--first-line",
    );
    case.refused(&["message", "handle-message"], "--message-id");
    case.refused(
        &[
            "message",
            "route-need",
            "--message-id",
            "01890a5d-ac96-774b-bcce-b302099a8057",
        ],
        "--dispatch-id",
    );
    assert!(
        !case.state.exists(),
        "a refused flag check opened no store at {}",
        case.state.display()
    );
}

/// Messages: `[DECISION-REQUEST cortex cortex-1] …` is received as kind `DecisionRequest` and then
/// handled. `hello` does not parse: it is recorded `Rejected` straight away, the command prints
/// the first-line format for the sender and exits 1.
#[test]
fn a_decision_request_is_received_and_handled_and_hello_is_rejected_with_the_format() {
    let case = Case::new("messages");
    let request = case.receive("[DECISION-REQUEST cortex cortex-1] Which store backs the pilot?");
    assert_eq!(
        case.message(&request),
        json!({
            "message_id": request,
            "state": "Received",
            "sender": "cortex",
            "recipient": "conductor",
            "kind": "DecisionRequest",
            "first_line": "[DECISION-REQUEST cortex cortex-1] Which store backs the pilot?",
            "transport": "SendMessage",
            "received_at": RECEIVED_AT,
            "body_path": null,
            "dispatch_id": null,
        })
    );
    case.ok(&["message", "handle-message", "--message-id", &request]);
    assert_eq!(case.message(&request)["state"], "Handled");
    case.declined(
        &["message", "handle-message", "--message-id", &request],
        "MessageStateConflict",
    );

    let output = case.run(&Case::receive_args("hello", &["--kind", "Report"]));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unparsed first line exits 1; stdout {stdout:?}, stderr {stderr:?}"
    );
    for format in [
        "[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>",
        "[REPORT <repo> <dispatch-id>] <started|progress|blocked|unblocked|done|failed> <one line>",
        "[DECISION-REQUEST <repo> <request-id>] <question>",
        "[DECISION <request-id>] <option> <one-line reason> (<decision-id>)",
        "[NEED <repo> <request-id>] <other-repo> <what is needed>",
        "[RESOURCE <repo>] <build-slot|disk|usage> <amount>",
        "[ESCALATION <request-id>] <O|H> <question>",
    ] {
        assert!(
            stdout.contains(format),
            "the format back to the sender holds {format:?}: {stdout:?}"
        );
    }
    let rows = case.rows("message", "messages");
    let hello: Vec<&Value> = rows
        .iter()
        .filter(|row| row["first_line"] == "hello")
        .collect();
    assert_eq!(hello.len(), 1, "hello is recorded once: {rows:?}");
    assert_eq!(
        hello[0]["state"], "Rejected",
        "recorded straight into Rejected"
    );
    let hello_id = hello[0]["message_id"].as_str().expect("message_id is text");
    assert!(
        stderr.contains(hello_id) && stderr.lines().count() == 1,
        "standard error names the rejected message: {stderr:?}"
    );
    case.declined(
        &["message", "handle-message", "--message-id", hello_id],
        "MessageStateConflict",
    );

    let report = case.receive("[REPORT cortex DSP-20261007-01] progress half the crates build");
    assert_eq!(case.message(&report)["kind"], "Report");
    case.ok(&[
        "message",
        "reject-message",
        "--message-id",
        &report,
        "--reason",
        "no such dispatch",
    ]);
    assert_eq!(case.message(&report)["state"], "Rejected");
    case.declined(
        &[
            "message",
            "reject-message",
            "--message-id",
            &report,
            "--reason",
            "again",
        ],
        "MessageStateConflict",
    );
    case.declined(
        &[
            "message",
            "handle-message",
            "--message-id",
            "01890a5d-ac96-774b-bcce-b302099a8057",
        ],
        "MessageNotFound",
    );
    case.kept_under_the_state_dir();
}

/// Each § 6 first line parses into its kind; a line that breaks its format is recorded
/// `Rejected` under the kind its tag names. A `--kind` or `--parsed` that disagrees with the line
/// is refused, and nothing is written.
#[test]
fn receive_message_parses_each_first_line_format_into_its_kind() {
    let case = Case::new("parse");
    for (line, kind) in [
        (
            "[DISPATCH cortex DSP-20261007-01] G9 expose the store id",
            "Dispatch",
        ),
        ("[REPORT cortex DSP-20261007-01] started on it", "Report"),
        (
            "[REPORT cortex DSP-20261007-01] done origin/main 0123abc",
            "Report",
        ),
        (
            "[DECISION-REQUEST cortex c-20261007-12] Which store?",
            "DecisionRequest",
        ),
        (
            "[DECISION c-20261007-12] A the file store is enough (DEC-20261007-04)",
            "Decision",
        ),
        ("[NEED cortex N-26-09] delta list the store ids", "Need"),
        ("[RESOURCE cortex] build-slot 1", "Resource"),
        ("[RESOURCE cortex] disk 20G", "Resource"),
        ("[ESCALATION c-20261007-12] O Which store?", "Escalation"),
        (
            "[ESCALATION c-20261007-12] H Plug in the device",
            "Escalation",
        ),
    ] {
        let id = case.receive(line);
        let row = case.message(&id);
        assert_eq!(
            (row["kind"].as_str(), row["state"].as_str()),
            (Some(kind), Some("Received")),
            "{line:?} parses as {kind}"
        );
    }

    for (line, kind) in [
        ("[REPORT cortex DSP-20261007-01] finished it", "Report"),
        ("[REPORT cortex] started on it", "Report"),
        ("[RESOURCE cortex] cpu 4", "Resource"),
        ("[ESCALATION c-20261007-12] X Which store?", "Escalation"),
        ("[DECISION c-20261007-12] A no decision id", "Decision"),
        ("[NEED cortex N-26-09] delta", "Need"),
        ("[DISPATCH cortex DSP-1]", "Dispatch"),
        ("REPORT cortex DSP-1 started", "Need"),
        ("[RESTART conductor] new profile", "Report"),
    ] {
        let output = case.run(&Case::receive_args(line, &["--kind", kind]));
        assert_eq!(
            output.status.code(),
            Some(1),
            "{line:?} does not parse; stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rows = case.rows("message", "messages");
        let row = rows
            .iter()
            .find(|row| row["first_line"] == line)
            .unwrap_or_else(|| panic!("{line:?} is recorded: {rows:?}"));
        assert_eq!(
            (row["kind"].as_str(), row["state"].as_str()),
            (Some(kind), Some("Rejected")),
            "{line:?} is recorded Rejected as {kind}"
        );
    }

    let before = case.rows("message", "messages");
    case.refused(
        &Case::receive_args("[NEED cortex N-26-10] delta list it", &["--kind", "Report"]),
        "--kind",
    );
    case.refused(
        &Case::receive_args(
            "[NEED cortex N-26-10] delta list it",
            &["--parsed", "false"],
        ),
        "--parsed",
    );
    case.refused(
        &Case::receive_args("hello", &["--kind", "Report", "--parsed", "true"]),
        "--parsed",
    );
    assert_eq!(
        case.rows("message", "messages"),
        before,
        "the refused receipts wrote nothing"
    );
}

/// Route-need: a `[NEED cortex cortex-2] ekr …` is routed into a dispatch for
/// `epistemic-knowledge-runtime`; the need reads `Handled` with the dispatch it opened. Handling a
/// need without routing it is refused with `NeedNotRouted`, and routing a message that is not a
/// need with `NotANeed`.
#[test]
fn a_need_is_routed_into_a_dispatch_for_epistemic_knowledge_runtime() {
    let case = Case::new("route-need");
    case.propose("G9");
    let need = case.receive("[NEED cortex cortex-2] ekr expose the store id");
    assert_eq!(case.message(&need)["kind"], "Need");
    case.declined(
        &["message", "handle-message", "--message-id", &need],
        "NeedNotRouted",
    );
    assert_eq!(case.message(&need)["state"], "Received");

    let dispatch = case.send("epistemic-knowledge-runtime", "G9", SENT_AT, &[]);
    case.ok(&[
        "message",
        "route-need",
        "--message-id",
        &need,
        "--dispatch-id",
        &dispatch,
    ]);
    let row = case.message(&need);
    assert_eq!(
        (row["state"].as_str(), row["dispatch_id"].as_str()),
        (Some("Handled"), Some(dispatch.as_str())),
        "the need is handled and records the dispatch it opened: {row}"
    );
    assert_eq!(
        case.dispatch(&dispatch)["repository"],
        "epistemic-knowledge-runtime"
    );

    let request = case.receive("[DECISION-REQUEST cortex cortex-3] Which store?");
    case.declined(
        &[
            "message",
            "route-need",
            "--message-id",
            &request,
            "--dispatch-id",
            &dispatch,
        ],
        "NotANeed",
    );
    case.declined(
        &[
            "message",
            "route-need",
            "--message-id",
            &need,
            "--dispatch-id",
            &dispatch,
        ],
        "MessageStateConflict",
    );
    case.kept_under_the_state_dir();
}

/// Exports: `dispatch dispatches --format jsonl` and `message messages --format jsonl` list rows
/// in the order they were first stored, and two runs over one store are byte-identical.
#[test]
fn each_jsonl_export_lists_rows_in_first_stored_order_and_runs_byte_identical() {
    let case = Case::new("exports");
    case.propose("G9");
    let first = case.send("cortex", "G9", SENT_AT, &[]);
    let second = case.send("delta", "G9", SENT_AT, &[]);
    case.ok(&["dispatch", "report-started", "--dispatch-id", &first]);
    let one = case.receive("[REPORT cortex DSP-20261007-01] started on it");
    let two = case.receive("[RESOURCE delta] disk 20G");
    case.ok(&["message", "handle-message", "--message-id", &one]);

    let dispatches = case.ok(&["dispatch", "dispatches", "--format", "jsonl"]);
    assert_eq!(
        dispatches,
        case.ok(&["dispatch", "dispatches", "--format", "jsonl"]),
        "two runs of the dispatches export are byte-identical"
    );
    let order: Vec<Value> = case
        .rows("dispatch", "dispatches")
        .into_iter()
        .map(|row| row["dispatch_id"].clone())
        .collect();
    assert_eq!(order, [json!(first), json!(second)], "first-stored order");
    assert_eq!(dispatches.lines().count(), 2);

    let messages = case.ok(&["message", "messages", "--format", "jsonl"]);
    assert_eq!(
        messages,
        case.ok(&["message", "messages", "--format", "jsonl"]),
        "two runs of the messages export are byte-identical"
    );
    let order: Vec<Value> = case
        .rows("message", "messages")
        .into_iter()
        .map(|row| row["message_id"].clone())
        .collect();
    assert_eq!(order, [json!(one), json!(two)], "first-stored order");
}

/// A view opens only an existing store: with none under `--state-dir` it exits 1 naming it, and
/// creates nothing.
#[test]
fn the_views_create_no_store() {
    let case = Case::new("no-store");
    for (group, view) in [("dispatch", "dispatches"), ("message", "messages")] {
        let output = case.run(&[group, view]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.code() == Some(1) && stderr.contains("no store at"),
            "`conductor {group} {view}` without a store: exit {:?}, stderr {stderr:?}",
            output.status.code()
        );
    }
    assert!(
        !case.state.exists(),
        "a view created {}",
        case.state.display()
    );
}

// ---------------------------------------------------------------------------------------------
// Wave 06 U7: dispatch ids (decisions 2 and 3 of its brief) and the first-line reader
// (decision 4).
// ---------------------------------------------------------------------------------------------

/// A dispatch id given by hand is in the allocator's form, `DSP-YYYYMMDD-NN`: a date, then a
/// number of two digits, or one of 100 or more without a leading zero. Any other is refused with
/// exit 1, naming the id and the form, and nothing is sent; so no two ids carry one number of one
/// date.
#[test]
fn a_hand_given_dispatch_id_outside_the_allocators_form_is_refused() {
    let case = Case::new("dsp-hand-id-form");
    case.propose("G9");
    assert_eq!(case.send("cortex", "G9", SENT_AT, &[]), "DSP-20261007-01");
    for given in [
        "DSP-20261007-1",
        "DSP-20261007-001",
        "DSP-20261007-0100",
        "DSP-20261007-01a",
        "DSP-20261007-+1",
        "DSP-20261007-",
        "DSP-20261007",
        "DSP-2026107-01",
        "DSP-20261307-01",
        "DSP-00001007-01",
        "DSP-self",
    ] {
        let line = case.refused(
            &[
                "dispatch",
                "send-dispatch",
                "--dispatch-id",
                given,
                "--repository",
                "cortex",
                "--goal-id",
                "G9",
                "--brief",
                "b",
                "--sent-at",
                SENT_AT,
                "--priority",
                "1",
            ],
            "DSP-YYYYMMDD-NN",
        );
        assert!(line.contains(given), "names {given}: {line:?}");
    }
    assert_eq!(
        case.rows("dispatch", "dispatches").len(),
        1,
        "nothing was sent"
    );
    for given in ["DSP-20261007-00", "DSP-20261007-42", "DSP-20261007-100"] {
        assert_eq!(
            case.send("cortex", "G9", SENT_AT, &["--dispatch-id", given]),
            given
        );
    }
}

/// A send instant whose UTC date has no year from 1 to 9999 is refused, naming `--sent-at`, and
/// nothing is sent: a year of 0 here (`tests/adv_dispatch_w06.rs` holds -1 and 10000). The first
/// and last instants of that range are sent.
#[test]
fn a_sent_at_outside_utc_years_1_to_9999_is_refused() {
    let case = Case::new("dsp-years");
    case.propose("G9");
    let line = case.refused(
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            "cortex",
            "--goal-id",
            "G9",
            "--brief",
            "b",
            "--sent-at",
            "0000-06-01T00:00:00Z",
            "--priority",
            "1",
        ],
        "--sent-at",
    );
    assert!(line.contains("0000-06-01T00:00:00Z"), "{line:?}");
    assert_eq!(case.rows("dispatch", "dispatches"), Vec::<Value>::new());
    assert_eq!(
        case.send("cortex", "G9", "0001-01-01T00:00:00Z", &[]),
        "DSP-00010101-01"
    );
    assert_eq!(
        case.send("cortex", "G9", "9999-12-31T23:59:59Z", &[]),
        "DSP-99991231-01"
    );
}

/// The tag is the first word after `[`, with or without the closing `]`: for every § 6 tag, a
/// first line that opens with it and breaks off is recorded `Rejected` with the tag's kind, and
/// the sender gets the format back, with no `--kind` given.
#[test]
fn every_tag_names_its_kind_without_its_closing_bracket() {
    let case = Case::new("unclosed-tags");
    let mut problems = Vec::new();
    for (tag, kind) in [
        ("DISPATCH", "Dispatch"),
        ("REPORT", "Report"),
        ("DECISION-REQUEST", "DecisionRequest"),
        ("DECISION", "Decision"),
        ("NEED", "Need"),
        ("RESOURCE", "Resource"),
        ("ESCALATION", "Escalation"),
    ] {
        for line in [
            format!("[{tag}"),
            format!("[{tag} cortex"),
            format!("  [{tag} cortex DSP-20261007-01 done it"),
        ] {
            let output = case.run(&Case::receive_args(&line, &[]));
            let stdout = String::from_utf8_lossy(&output.stdout);
            let recorded = case
                .rows("message", "messages")
                .into_iter()
                .find(|row| row["first_line"] == line.as_str())
                .map(|row| (row["kind"].clone(), row["state"].clone()));
            if output.status.code() != Some(1)
                || !stdout.starts_with("The first line of a message to conductor is one of:")
                || recorded != Some((json!(kind), json!("Rejected")))
            {
                problems.push(format!(
                    "{line:?}: recorded {recorded:?}; exit {:?}, stderr {:?}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} unclosed tags are not recorded Rejected with their kind:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}
