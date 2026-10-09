//! Adversary, wave 06 U1 (`story:decision-commands`, the decision group), pass 1: decision ids.
//!
//! Each case runs the built binary against a store of its own under this test target's temporary
//! directory, as `tests/decision.rs` does.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::Value;

const AT: &str = "2026-10-07T10:00:00Z";

struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adv_decision_ids")
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

    fn run(&self, args: &[&str]) -> Output {
        common::conductor(self.work.with_file_name("home"))
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    fn record_args<'a>(
        who: &'a str,
        class: &'a str,
        at: &'a str,
        more: &[&'a str],
    ) -> Vec<&'a str> {
        let mut args = vec![
            "decision",
            who,
            "--decision-class",
            class,
            "--question",
            "which way?",
            "--options",
            "A or B",
            "--choice",
            "A",
            "--reason",
            "A is reversible",
            "--evidence",
            "the request",
            "--decided-at",
            at,
        ];
        args.extend_from_slice(more);
        args
    }

    /// Records a decision that must be accepted, and answers the id it printed.
    fn record(&self, who: &str, class: &str, more: &[&str]) -> String {
        let output = self.run(&Self::record_args(who, class, AT, more));
        assert!(
            output.status.success(),
            "the record failed: exit {:?}, stderr {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("stdout is UTF-8")
            .trim_end()
            .to_owned()
    }

    fn decision_ids(&self) -> Vec<String> {
        let output = self.run(&["decision", "decisions", "--format", "jsonl"]);
        assert!(output.status.success(), "the decisions view runs");
        String::from_utf8(output.stdout)
            .expect("stdout is UTF-8")
            .lines()
            .map(|line| {
                let row: Value = serde_json::from_str(line).expect("a JSON object");
                row["decision_id"].as_str().expect("an id").to_owned()
            })
            .collect()
    }
}

/// `decide.rs` refuses a `--decided-at` whose UTC year is outside 0000 to 9999 with one stderr line
/// (`utc_date`). On the far side of 9999 the conversion to UTC panics before that check runs:
/// `9999-12-31T23:30:00-01:00` is accepted by the flag's parser and is 10000-01-01 in UTC.
#[test]
fn adv_decided_at_past_9999_in_utc_is_refused_not_a_panic() {
    let case = Case::new("year-10000");
    let args = Case::record_args(
        "record-operator-decision",
        "C",
        "9999-12-31T23:30:00-01:00",
        &[],
    );
    let output = case.run(&args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a decided_at past 9999 in UTC is refused with exit 1, like one before 0000; got exit \
         {:?}, stderr {stderr:?}",
        output.status.code()
    );
    assert!(
        stderr.starts_with("conductor: decision record-operator-decision: ")
            && stderr.contains("--decided-at")
            && stderr.lines().count() == 1,
        "one line naming --decided-at: {stderr:?}"
    );
}

/// Recording a class-O decision as conductor without `--decision-id` allocates an id first and
/// names it in the `ReservedForOperator` refusal ("decision \"DEC-…\" is class O … nothing was
/// recorded"). Nothing was recorded, so the next decision is allocated the same id: the refusal
/// names, as the refused decision, an id that then belongs to another one.
#[test]
fn adv_an_id_a_refusal_names_is_not_given_to_another_decision() {
    let case = Case::new("refusal-names-next-id");
    case.record("record-conductor-decision", "C", &[]);
    let refused = case.run(&Case::record_args(
        "record-conductor-decision",
        "O",
        AT,
        &[],
    ));
    assert_eq!(refused.status.code(), Some(1), "class O is refused");
    let stderr = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(stderr.contains("ReservedForOperator"), "{stderr:?}");

    let next = case.record("record-conductor-decision", "C", &[]);
    assert!(
        !stderr.contains(&next),
        "the refusal named {next} as the refused class-O decision, and {next} was then given to \
         a different, class-C decision; refusal: {stderr:?}"
    );
}

/// The allocator reads `DEC-20261007-1`, `-01` and `-001` as the same number, 1 (`allocated`
/// parses the suffix). The check that refuses a hand-given id that exists compares strings, so a
/// hand-given `DEC-20261007-1` or `DEC-20261007-001` is recorded beside the allocated
/// `DEC-20261007-01`: three decisions numbered 1 on one date.
#[test]
fn adv_a_hand_id_numbering_a_held_decision_is_refused() {
    let case = Case::new("hand-id-same-number");
    let allocated = case.record("record-conductor-decision", "C", &[]);
    assert_eq!(allocated, "DEC-20261007-01");
    let mut kept = Vec::new();
    for given in ["DEC-20261007-1", "DEC-20261007-001"] {
        let output = case.run(&Case::record_args(
            "record-operator-decision",
            "C",
            AT,
            &["--decision-id", given],
        ));
        if output.status.success() {
            kept.push(given);
        }
    }
    assert_eq!(
        kept,
        Vec::<&str>::new(),
        "each is decision number 1 of 2026-10-07, which DEC-20261007-01 already is; the store now \
         holds {:?}",
        case.decision_ids()
    );
}
