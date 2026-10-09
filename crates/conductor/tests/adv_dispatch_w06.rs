//! Adversary cases for `story:decision-commands`, wave 06 U2 (the `dispatch` and `message`
//! groups), pass 1: the first-line reader against the § 6 forms it prints back to the sender,
//! the `DSP-YYYYMMDD-NN` allocation at the edges of the send instant's range, and allocation under
//! more concurrent writers than its retry bound.
//!
//! Each case runs the built binary with `--state-dir` pointed at its own directory under this test
//! target's temporary directory in `target/`.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::thread;

use serde_json::Value;

/// The first line of what `receive-message` prints for the sender of a line that does not parse.
const FORMAT_HEAD: &str = "The first line of a message to conductor is one of:";

/// A state directory for one case, emptied.
fn state_dir(case: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_dispatch_w06")
        .join(case);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case's directory");
    }
    fs::create_dir_all(&root).expect("create the case's directory");
    root.join("state")
}

/// Runs `conductor --state-dir <state> <args>`.
fn conductor(state: &PathBuf, args: &[&str]) -> Output {
    common::conductor(state.with_file_name("home"))
        .arg("--state-dir")
        .arg(state)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

/// Runs `conductor message receive-message` for a message from cortex with `first_line` and no
/// `--kind`, as conductor does for a line whose tag names the kind.
fn receive(state: &PathBuf, first_line: &str) -> Output {
    conductor(
        state,
        &[
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
        ],
    )
}

/// The rows of `group view --format jsonl`; none when there is no store yet.
fn rows(state: &PathBuf, group: &str, view: &str) -> Vec<Value> {
    if !state.join("tree").exists() {
        return Vec::new();
    }
    let output = conductor(state, &[group, view, "--format", "jsonl"]);
    assert!(
        output.status.success(),
        "`conductor {group} {view}` exits 0; stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("the view is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
        .collect()
}

/// The `messages` row whose first line is `first_line`, as `(kind, state)`.
fn recorded(state: &PathBuf, first_line: &str) -> Option<(String, String)> {
    rows(state, "message", "messages")
        .into_iter()
        .find(|row| row["first_line"] == first_line)
        .map(|row| {
            (
                row["kind"].as_str().unwrap_or_default().to_owned(),
                row["state"].as_str().unwrap_or_default().to_owned(),
            )
        })
}

/// One line of what a run printed: exit status, standard output, standard error.
fn shown(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn propose(state: &PathBuf, goal: &str) {
    let output = conductor(
        state,
        &[
            "goal",
            "propose-goal",
            "--goal-id",
            goal,
            "--title",
            "Ship the pilot",
            "--exit-evidence",
            "listed as met",
        ],
    );
    assert!(
        output.status.success(),
        "propose {goal}: {}",
        shown(&output)
    );
}

fn send(state: &PathBuf, sent_at: &str) -> Output {
    conductor(
        state,
        &[
            "dispatch",
            "send-dispatch",
            "--repository",
            "cortex",
            "--goal-id",
            "G9",
            "--brief",
            "Expose the store id",
            "--sent-at",
            sent_at,
            "--priority",
            "1",
        ],
    )
}

/// `Reading::kind` (`src/message.rs`): "`[NEED …` is a need even when the rest breaks the form."
/// A `[NEED` line whose closing bracket is missing, and the first line of a `[NEED` header split
/// over two lines, carry the tag, so each is recorded `Rejected` as a need and the sender gets the
/// format back. `read` finds the tag only after a `]`, so both are answered "--kind is missing,
/// and the first line names no kind; nothing was received": no record, and no format for the
/// sender.
#[test]
fn adv_w06_a_need_tag_without_its_closing_bracket_is_recorded_rejected_as_a_need() {
    let state = state_dir("need-unclosed");
    let mut problems = Vec::new();
    for line in [
        "[NEED cortex N-26-11 delta list the store ids",
        "[NEED cortex",
    ] {
        let output = receive(&state, line);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let row = recorded(&state, line);
        if output.status.code() != Some(1)
            || !stdout.starts_with(FORMAT_HEAD)
            || row != Some(("Need".to_owned(), "Rejected".to_owned()))
        {
            problems.push(format!("{line:?}: recorded {row:?}; {}", shown(&output)));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of 2 lines carrying the NEED tag are not recorded Rejected as a need with the format \
         printed:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// The forms `receive-message` prints back (`FORMAT`, `src/message.rs`) put a `<dispatch-id>` in
/// the brackets of `[DISPATCH …]` and `[REPORT …]`, and a `(<decision-id>)` at the end of
/// `[DECISION …]`; the specification declares both types by prefix (`DSP-`, `DEC-`). A line with
/// any other word in that place does not have its form, so it is recorded `Rejected` and the
/// sender gets the format back. `read` counts the bracketed fields without reading them, so each
/// is recorded `Received`, and conductor holds a report it cannot apply: `dispatch report-done`
/// refuses `--dispatch-id not-a-dispatch`.
#[test]
fn adv_w06_a_bracketed_id_not_of_its_declared_type_does_not_parse() {
    let state = state_dir("typed-ids");
    let control = "[REPORT cortex DSP-20261007-01] done origin/main 0123abc";
    let output = receive(&state, control);
    assert!(
        output.status.success(),
        "the control line parses: {}",
        shown(&output)
    );

    let mut problems = Vec::new();
    for (line, kind) in [
        (
            "[REPORT cortex not-a-dispatch] done origin/main 0123abc",
            "Report",
        ),
        ("[DISPATCH cortex 42] G9 expose the store id", "Dispatch"),
        (
            "[DECISION c-20261007-12] A the file store is enough (probably)",
            "Decision",
        ),
    ] {
        let output = receive(&state, line);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let row = recorded(&state, line);
        if output.status.code() != Some(1)
            || !stdout.starts_with(FORMAT_HEAD)
            || row != Some((kind.to_owned(), "Rejected".to_owned()))
        {
            problems.push(format!("{line:?}: recorded {row:?}; {}", shown(&output)));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of 3 lines with a bracketed id that is not of its type parse:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// `--sent-at` takes any RFC 3339 instant (`timestamp`, `src/cli.rs`), and the allocated id is
/// `DSP-<UTC date as YYYYMMDD>-NN`. An instant whose UTC date has no four-digit year is refused
/// with an error, and nothing is stored. `utc_day` (`src/dispatch.rs`) converts with
/// `OffsetDateTime::to_offset`, which panics past 9999-12-31 UTC, and formats a year before 0001
/// as `-001`: the first send exits 101 with a panic, the second stores `DSP--0011231-01`.
#[test]
fn adv_w06_a_send_instant_outside_four_digit_utc_years_is_refused_without_a_panic() {
    let state = state_dir("year-range");
    propose(&state, "G9");
    let mut problems = Vec::new();
    for sent_at in ["9999-12-31T23:30:00-01:00", "0000-01-01T00:30:00+01:00"] {
        let output = send(&state, sent_at);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success()
            || output.status.code() == Some(101)
            || stderr.contains("panicked")
            || !output.stdout.is_empty()
        {
            problems.push(format!("--sent-at {sent_at}: {}", shown(&output)));
        }
    }
    let stored: Vec<Value> = rows(&state, "dispatch", "dispatches")
        .into_iter()
        .map(|row| row["dispatch_id"].clone())
        .collect();
    if !stored.is_empty() {
        problems.push(format!("stored {stored:?}"));
    }
    assert!(
        problems.is_empty(),
        "send instants outside four-digit UTC years are not refused cleanly:\n  - {}",
        problems.join("\n  - ")
    );
}

/// Five `send-dispatch` calls at once, none naming an id. Each call that loses its allocated id
/// lost it to another call that was sent, so ids are left and every call is sent, with
/// `DSP-20261007-01` to `-05`. The calls retry in lockstep and `REALLOCATIONS` (`src/dispatch.rs`)
/// stops each after four allocations, so exactly four are sent and the fifth exits 1, "another
/// writer took each of the 4 ids allocated". Measured on this tree: 4 sent of 5, 6 and 8 calls,
/// in each of 3 rounds.
#[test]
fn adv_w06_five_concurrent_sends_are_all_sent_with_five_ids() {
    let state = state_dir("five-at-once");
    propose(&state, "G9");
    let state = &state;
    let outputs: Vec<Output> = thread::scope(|scope| {
        let calls: Vec<_> = (0..5)
            .map(|_| scope.spawn(move || send(state, "2026-10-07T09:00:00Z")))
            .collect();
        calls
            .into_iter()
            .map(|call| call.join().expect("a send-dispatch call"))
            .collect()
    });
    let refused: Vec<String> = outputs
        .iter()
        .filter(|output| !output.status.success())
        .map(shown)
        .collect();
    let mut ids: Vec<String> = outputs
        .iter()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .collect();
    ids.sort();
    assert!(
        refused.is_empty()
            && ids
                == [
                    "DSP-20261007-01",
                    "DSP-20261007-02",
                    "DSP-20261007-03",
                    "DSP-20261007-04",
                    "DSP-20261007-05",
                ],
        "five calls at once: sent {ids:?}; {} refused:\n  - {}",
        refused.len(),
        refused.join("\n  - ")
    );
}

/// The unit's report: "When the line does carry a tag (e.g. `[NEED …` with a broken rest), that
/// tag gives the kind." Every broken line in `tests/dispatch.rs` is received with `--kind`, so a
/// `read` that answered no kind for a line that does not parse would pass that suite. Here the
/// line is received without `--kind`.
#[test]
fn adv_w06_a_need_whose_rest_breaks_its_form_takes_its_kind_from_the_tag() {
    let state = state_dir("need-broken-rest");
    let line = "[NEED cortex N-26-09] delta";
    let output = receive(&state, line);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.code() == Some(1) && stdout.starts_with(FORMAT_HEAD),
        "{line:?} is refused with the format: {}",
        shown(&output)
    );
    assert_eq!(
        recorded(&state, line),
        Some(("Need".to_owned(), "Rejected".to_owned())),
        "{line:?} is recorded Rejected as a need"
    );
}
