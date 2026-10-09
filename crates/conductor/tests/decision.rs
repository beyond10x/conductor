//! `story:decision-commands`, the `conductor decision` group: decision requests and the decisions
//! that answer them, run as the built binary against the store under `--state-dir`.
//!
//! Each case has its own directory under this test target's temporary directory in `target/`: the
//! binary runs from `work/` in it and keeps its records in `state/` beside it. A command that is
//! refused exits 1 with one line on standard error naming the command and the error the
//! specification declares, and writes nothing on standard output.
//!
//! `record-conductor-decision` and `record-operator-decision` print the id of the decision they
//! record. Given no `--decision-id`, they allocate `DEC-YYYYMMDD-NN`: the UTC date of
//! `--decided-at`, and one more than the highest number the store holds for that date.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// The instant most records here are decided or raised at.
const AT: &str = "2026-10-07T10:00:00Z";

/// One case's directories: where the binary runs, and the state directory it is pointed at.
struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("decision")
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

    fn command(&self, args: &[&str]) -> Command {
        let mut command = common::conductor(self.work.with_file_name("home"));
        command
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null());
        command
    }

    /// Runs `conductor --state-dir <state> <args>` from the case's working directory.
    fn run(&self, args: &[&str]) -> Output {
        self.command(args)
            .output()
            .expect("the conductor binary runs")
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

    /// Runs a command that must succeed and print nothing.
    fn quiet(&self, args: &[&str]) {
        assert_eq!(
            self.ok(args),
            "",
            "`conductor {}` prints nothing",
            args.join(" ")
        );
    }

    /// Runs a command that must fail with exit 1, nothing on standard output and one line on
    /// standard error starting with `conductor: <group> <command>: `, and answers that line.
    fn failed(&self, args: &[&str]) -> String {
        let output = self.run(args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let command = format!("conductor: {} {}: ", args[0], args[1]);
        assert!(
            output.status.code() == Some(1)
                && output.stdout.is_empty()
                && stderr.starts_with(&command)
                && stderr.ends_with('\n')
                && stderr.lines().count() == 1,
            "`conductor {}` should fail: exit 1, no stdout, one stderr line starting \
             {command:?}; got exit {:?}, stdout {:?}, stderr {stderr:?}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
        );
        stderr
    }

    /// Runs a command that the specification refuses with `error`.
    fn refused(&self, args: &[&str], error: &str) -> String {
        let stderr = self.failed(args);
        let named = format!("conductor: {} {}: {error}: ", args[0], args[1]);
        assert!(
            stderr.starts_with(&named),
            "`conductor {}` should be refused with {error}; stderr {stderr:?}",
            args.join(" ")
        );
        stderr
    }

    /// The rows of a `decision` view, read as `--format jsonl`.
    fn rows(&self, view: &str) -> Vec<Value> {
        self.ok(&["decision", view, "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
            .collect()
    }

    /// The row the `requests` view gives `request`.
    fn request(&self, request: &str) -> Value {
        let rows = self.rows("requests");
        rows.iter()
            .find(|row| row["request_id"] == request)
            .unwrap_or_else(|| panic!("the requests view has no row for {request}: {rows:?}"))
            .clone()
    }

    /// The row the `decisions` view gives `decision`.
    fn decision(&self, decision: &str) -> Value {
        let rows = self.rows("decisions");
        rows.iter()
            .find(|row| row["decision_id"] == decision)
            .unwrap_or_else(|| panic!("the decisions view has no row for {decision}: {rows:?}"))
            .clone()
    }

    fn raise(&self, request: &str, repository: &str) {
        self.quiet(&[
            "decision",
            "raise-decision-request",
            "--request-id",
            request,
            "--repository",
            repository,
            "--question",
            &format!("which way for {request}?"),
            "--options",
            "A: one way; B: the other",
            "--recommendation",
            "A",
            "--raised-at",
            AT,
        ]);
    }

    /// The arguments of a `record-<who>-decision` command of class `class`, decided at `at`, with
    /// `more` after them.
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
            "A: one way; B: the other",
            "--choice",
            "A",
            "--reason",
            "A is reversible",
            "--evidence",
            "the request's own text",
            "--decided-at",
            at,
        ];
        args.extend_from_slice(more);
        args
    }

    /// Records a decision and answers the id it printed: one line, nothing else.
    fn record(&self, who: &str, class: &str, at: &str, more: &[&str]) -> String {
        let args = Self::record_args(who, class, at, more);
        let printed = self.ok(&args);
        let id = printed
            .strip_suffix('\n')
            .filter(|id| !id.contains('\n'))
            .unwrap_or_else(|| {
                panic!(
                    "`conductor {}` printed {printed:?}, not one line",
                    args.join(" ")
                )
            });
        id.to_owned()
    }

    fn conductor_decision(&self, class: &str) -> String {
        self.record("record-conductor-decision", class, AT, &[])
    }

    fn operator_decision(&self, class: &str) -> String {
        self.record("record-operator-decision", class, AT, &[])
    }
}

#[test]
fn a_class_o_decision_recorded_as_conductor_is_refused_and_nothing_is_kept() {
    let case = Case::new("class-o-refused");
    for class in ["O", "H"] {
        let args = Case::record_args("record-conductor-decision", class, AT, &[]);
        case.refused(&args, "ReservedForOperator");
    }
    let given = Case::record_args(
        "record-conductor-decision",
        "O",
        AT,
        &["--decision-id", "DEC-20261007-05"],
    );
    let stderr = case.refused(&given, "ReservedForOperator");
    assert!(
        stderr.contains("DEC-20261007-05"),
        "names the decision: {stderr:?}"
    );
    assert_eq!(
        case.rows("decisions"),
        Vec::<Value>::new(),
        "nothing was recorded"
    );
}

#[test]
fn a_request_is_escalated_and_answered_by_the_operator() {
    let case = Case::new("escalate-and-answer");
    case.raise("cortex-1", "cortex");
    case.quiet(&[
        "decision",
        "escalate-request",
        "--request-id",
        "cortex-1",
        "--escalation-class",
        "O",
        "--reason",
        "it changes who decides releases",
    ]);
    let escalated = case.request("cortex-1");
    assert_eq!(escalated["state"], "Escalated");
    assert_eq!(escalated["escalation_class"], "O");
    assert_eq!(
        escalated["escalation_reason"],
        "it changes who decides releases"
    );

    case.raise("cortex-2", "cortex");
    case.refused(
        &[
            "decision",
            "escalate-request",
            "--request-id",
            "cortex-2",
            "--escalation-class",
            "C",
            "--reason",
            "conductor could decide it",
        ],
        "NotEscalatable",
    );
    assert_eq!(
        case.request("cortex-2")["state"],
        "Open",
        "a refused escalation moves nothing"
    );

    // An escalated request waits for the operator: conductor does not answer it.
    let conductors = case.conductor_decision("C");
    case.refused(
        &[
            "decision",
            "answer-request",
            "--request-id",
            "cortex-1",
            "--decision-id",
            &conductors,
        ],
        "RequestStateConflict",
    );

    let operators = case.operator_decision("O");
    case.quiet(&[
        "decision",
        "answer-escalated-request",
        "--request-id",
        "cortex-1",
        "--decision-id",
        &operators,
    ]);
    let answered = case.request("cortex-1");
    assert_eq!(answered["state"], "Answered");
    assert_eq!(answered["decision_id"], operators.as_str());

    let decision = case.decision(&operators);
    assert_eq!(decision["state"], "InForce");
    assert_eq!(decision["decision_class"], "O");
    assert_eq!(decision["decided_by"], "Operator");

    // An open request is conductor's to answer, with a decision in force.
    case.quiet(&[
        "decision",
        "answer-request",
        "--request-id",
        "cortex-2",
        "--decision-id",
        &conductors,
    ]);
    assert_eq!(case.request("cortex-2")["decision_id"], conductors.as_str());
    case.refused(
        &[
            "decision",
            "answer-request",
            "--request-id",
            "cortex-2",
            "--decision-id",
            "DEC-20261007-99",
        ],
        "DecisionNotFound",
    );
    case.refused(
        &[
            "decision",
            "answer-escalated-request",
            "--request-id",
            "no-such-request",
            "--decision-id",
            &operators,
        ],
        "RequestStateConflict",
    );
    case.refused(
        &[
            "decision",
            "escalate-request",
            "--request-id",
            "no-such-request",
            "--escalation-class",
            "O",
            "--reason",
            "unclear",
        ],
        "RequestNotFound",
    );
}

#[test]
fn a_request_is_withdrawn_once() {
    let case = Case::new("withdraw");
    case.raise("delta-1", "delta");
    let withdraw = [
        "decision",
        "withdraw-request",
        "--request-id",
        "delta-1",
        "--reason",
        "the controller found the answer in its charter",
    ];
    case.quiet(&withdraw);
    assert_eq!(case.request("delta-1")["state"], "Withdrawn");
    case.refused(&withdraw, "RequestStateConflict");
    case.refused(
        &[
            "decision",
            "withdraw-request",
            "--request-id",
            "no-such-request",
            "--reason",
            "gone",
        ],
        "RequestNotFound",
    );

    // An escalated request may be withdrawn too.
    case.raise("delta-2", "delta");
    case.quiet(&[
        "decision",
        "escalate-request",
        "--request-id",
        "delta-2",
        "--escalation-class",
        "H",
        "--reason",
        "it needs a login",
    ]);
    case.quiet(&[
        "decision",
        "withdraw-request",
        "--request-id",
        "delta-2",
        "--reason",
        "the login was not needed",
    ]);
    assert_eq!(case.request("delta-2")["state"], "Withdrawn");
}

#[test]
fn a_reversed_decision_answers_no_request() {
    let case = Case::new("reverse");
    let decision = case.conductor_decision("C");
    case.quiet(&[
        "decision",
        "reverse-conductor-decision",
        "--decision-id",
        &decision,
        "--reason",
        "the weekly review found it wrong",
    ]);
    assert_eq!(case.decision(&decision)["state"], "Reversed");

    case.raise("cortex-3", "cortex");
    case.refused(
        &[
            "decision",
            "answer-request",
            "--request-id",
            "cortex-3",
            "--decision-id",
            &decision,
        ],
        "DecisionReversed",
    );
    assert_eq!(
        case.request("cortex-3")["state"],
        "Open",
        "the request stays open"
    );

    case.refused(
        &[
            "decision",
            "reverse-conductor-decision",
            "--decision-id",
            &decision,
            "--reason",
            "again",
        ],
        "DecisionStateConflict",
    );
    case.refused(
        &[
            "decision",
            "reverse-conductor-decision",
            "--decision-id",
            "DEC-20261007-99",
            "--reason",
            "unknown",
        ],
        "DecisionNotFound",
    );

    // A decision the operator recorded is his to reverse.
    let operators = case.operator_decision("C");
    case.refused(
        &[
            "decision",
            "reverse-conductor-decision",
            "--decision-id",
            &operators,
            "--reason",
            "conductor disagrees",
        ],
        "ReversalReservedForOperator",
    );
    assert_eq!(case.decision(&operators)["state"], "InForce");
    case.quiet(&[
        "decision",
        "reverse-operator-decision",
        "--decision-id",
        &operators,
        "--reason",
        "the operator changed his mind",
    ]);
    assert_eq!(case.decision(&operators)["state"], "Reversed");
    case.refused(
        &[
            "decision",
            "reverse-operator-decision",
            "--decision-id",
            &operators,
            "--reason",
            "again",
        ],
        "DecisionStateConflict",
    );
}

#[test]
fn a_request_escalated_as_class_h_is_on_the_hands_todo_until_answered() {
    let case = Case::new("hands-todo");
    case.raise("llm-1", "llm");
    case.raise("llm-2", "llm");
    case.raise("llm-3", "llm");
    for (request, class) in [("llm-1", "H"), ("llm-2", "O")] {
        case.quiet(&[
            "decision",
            "escalate-request",
            "--request-id",
            request,
            "--escalation-class",
            class,
            "--reason",
            "the operator's",
        ]);
    }
    let todo = case.rows("hands-todo");
    let ids: Vec<&Value> = todo.iter().map(|row| &row["request_id"]).collect();
    assert_eq!(
        ids,
        [&json!("llm-1")],
        "only the class-H escalation: {todo:?}"
    );
    assert_eq!(todo[0]["repository"], "llm");
    assert_eq!(todo[0]["escalation_class"], "H");

    let operators = case.operator_decision("H");
    case.quiet(&[
        "decision",
        "answer-escalated-request",
        "--request-id",
        "llm-1",
        "--decision-id",
        &operators,
    ]);
    assert_eq!(
        case.rows("hands-todo"),
        Vec::<Value>::new(),
        "answered, so off the list"
    );
}

#[test]
fn decision_ids_are_allocated_per_utc_date() {
    let case = Case::new("ids");
    // Two calls in the same second get two ids.
    let first = case.conductor_decision("C");
    let second = case.conductor_decision("C");
    assert_eq!(first, "DEC-20261007-01");
    assert_eq!(second, "DEC-20261007-02");

    // The date is decided_at's, in UTC.
    let late = case.record(
        "record-operator-decision",
        "O",
        "2026-10-07T23:30:00-02:00",
        &[],
    );
    assert_eq!(late, "DEC-20261008-01");

    // An id given by hand is kept as given, and allocation continues above it.
    let given = case.record(
        "record-conductor-decision",
        "C",
        AT,
        &["--decision-id", "DEC-20261007-07"],
    );
    assert_eq!(given, "DEC-20261007-07");
    assert_eq!(case.conductor_decision("C"), "DEC-20261007-08");

    // 100 follows 99.
    case.record(
        "record-operator-decision",
        "C",
        "2026-10-09T08:00:00Z",
        &["--decision-id", "DEC-20261009-99"],
    );
    assert_eq!(
        case.record(
            "record-conductor-decision",
            "C",
            "2026-10-09T09:00:00Z",
            &[]
        ),
        "DEC-20261009-100"
    );

    // An id given by hand that exists is refused, naming it, and nothing is written.
    let before = case.ok(&["decision", "decisions", "--format", "jsonl"]);
    for who in ["record-conductor-decision", "record-operator-decision"] {
        let args = Case::record_args(
            who,
            "C",
            "2026-10-07T12:00:00Z",
            &["--decision-id", "DEC-20261007-07"],
        );
        let stderr = case.failed(&args);
        assert!(
            stderr.contains("DEC-20261007-07"),
            "the refusal names the id: {stderr:?}"
        );
    }
    assert_eq!(
        case.ok(&["decision", "decisions", "--format", "jsonl"]),
        before,
        "a refused id writes nothing"
    );
}

/// Four writers at once: a writer loses an allocated id only to another writer's record, so each
/// loses at most three times and its fourth allocation at the latest is its own.
#[test]
fn concurrent_records_get_distinct_ids() {
    let case = Case::new("concurrent-ids");
    case.conductor_decision("C");
    let args = Case::record_args(
        "record-conductor-decision",
        "C",
        "2026-10-10T10:00:00Z",
        &[],
    );
    let children: Vec<_> = (0..4)
        .map(|_| {
            case.command(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the conductor binary starts")
        })
        .collect();
    let mut ids: Vec<String> = children
        .into_iter()
        .map(|child| {
            let output = child.wait_with_output().expect("the conductor binary runs");
            assert!(
                output.status.success(),
                "a concurrent record failed: exit {:?}, stderr {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout)
                .expect("stdout is UTF-8")
                .trim_end()
                .to_owned()
        })
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            "DEC-20261010-01",
            "DEC-20261010-02",
            "DEC-20261010-03",
            "DEC-20261010-04"
        ]
    );
    assert_eq!(case.rows("decisions").len(), 5);
}

#[test]
fn a_request_id_is_given_and_never_reused() {
    let case = Case::new("request-ids");
    let stderr = case.failed(&[
        "decision",
        "raise-decision-request",
        "--repository",
        "cortex",
        "--question",
        "which way?",
        "--options",
        "A or B",
        "--recommendation",
        "A",
        "--raised-at",
        AT,
    ]);
    assert!(
        stderr.contains("--request-id"),
        "names the flag: {stderr:?}"
    );
    assert!(
        !case.state.join("tree").exists(),
        "a missing flag is named before any store is opened"
    );

    case.raise("cortex-1", "cortex");
    case.quiet(&[
        "decision",
        "escalate-request",
        "--request-id",
        "cortex-1",
        "--escalation-class",
        "H",
        "--reason",
        "a secret",
    ]);
    let before = case.ok(&["decision", "requests", "--format", "jsonl"]);
    let stderr = case.failed(&[
        "decision",
        "raise-decision-request",
        "--request-id",
        "cortex-1",
        "--repository",
        "cortex",
        "--question",
        "again?",
        "--options",
        "A or B",
        "--recommendation",
        "B",
        "--raised-at",
        AT,
    ]);
    assert!(stderr.contains("cortex-1"), "names the request: {stderr:?}");
    assert_eq!(
        case.ok(&["decision", "requests", "--format", "jsonl"]),
        before,
        "the held request is unchanged"
    );
}

#[test]
fn every_command_names_a_missing_flag() {
    let case = Case::new("missing-flags");
    let missing: [(&[&str], &str); 8] = [
        (
            &[
                "decision",
                "record-conductor-decision",
                "--decision-class",
                "C",
            ],
            "--question",
        ),
        (
            &[
                "decision",
                "record-operator-decision",
                "--decision-class",
                "C",
            ],
            "--question",
        ),
        (
            &["decision", "answer-request", "--request-id", "r"],
            "--decision-id",
        ),
        (
            &[
                "decision",
                "answer-escalated-request",
                "--decision-id",
                "DEC-x",
            ],
            "--request-id",
        ),
        (
            &[
                "decision",
                "escalate-request",
                "--request-id",
                "r",
                "--reason",
                "x",
            ],
            "--escalation-class",
        ),
        (
            &["decision", "withdraw-request", "--request-id", "r"],
            "--reason",
        ),
        (
            &["decision", "reverse-conductor-decision", "--reason", "x"],
            "--decision-id",
        ),
        (
            &[
                "decision",
                "reverse-operator-decision",
                "--decision-id",
                "DEC-x",
            ],
            "--reason",
        ),
    ];
    for (args, flag) in missing {
        let stderr = case.failed(args);
        assert!(
            stderr.contains(flag),
            "`conductor {}` names {flag}: {stderr:?}",
            args.join(" ")
        );
    }
    assert!(
        !case.state.join("tree").exists(),
        "a missing flag is named before any store is opened"
    );
}

#[test]
fn each_export_lists_rows_in_the_order_first_stored_and_is_byte_identical() {
    let case = Case::new("exports");
    let first = case.conductor_decision("C");
    let second = case.operator_decision("O");
    case.quiet(&[
        "decision",
        "reverse-conductor-decision",
        "--decision-id",
        &first,
        "--reason",
        "reversed after the second",
    ]);
    case.raise("b-request", "delta");
    case.raise("a-request", "cortex");
    case.quiet(&[
        "decision",
        "escalate-request",
        "--request-id",
        "b-request",
        "--escalation-class",
        "O",
        "--reason",
        "the operator's",
    ]);

    for view in ["decisions", "requests", "hands-todo"] {
        let args = ["decision", view, "--format", "jsonl"];
        let once = case.run(&args);
        let twice = case.run(&args);
        assert!(once.status.success() && twice.status.success());
        assert_eq!(
            once.stdout, twice.stdout,
            "`decision {view}` is byte-identical"
        );
    }

    let decisions = case.ok(&["decision", "decisions", "--format", "jsonl"]);
    let lines: Vec<&str> = decisions.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!(
            "{{\"decision_id\":\"{first}\",\"state\":\"Reversed\",\"decision_class\":\"C\",\
             \"decided_by\":\"Conductor\",\"choice\":\"A\",\"question\":\"which way?\",\
             \"options\":\"A: one way; B: the other\",\"reason\":\"A is reversible\",\
             \"evidence\":\"the request's own text\",\"decided_at\":\"{AT}\"}}"
        ),
        "the view's fields in the specification's order"
    );
    let second_row: Value = serde_json::from_str(lines[1]).expect("a JSON object");
    assert_eq!(second_row["decision_id"], second.as_str());

    let requests = case.rows("requests");
    let ids: Vec<&Value> = requests.iter().map(|row| &row["request_id"]).collect();
    assert_eq!(ids, [&json!("b-request"), &json!("a-request")]);
    let keys: Vec<&String> = requests[1]
        .as_object()
        .expect("a JSON object")
        .keys()
        .collect();
    assert_eq!(keys.len(), 11);
    assert_eq!(requests[1]["blocker"], Value::Null);
    assert_eq!(requests[1]["escalation_class"], Value::Null);
    let raw = case.ok(&["decision", "requests", "--format", "jsonl"]);
    assert!(
        raw.lines().nth(1).is_some_and(|line| line.starts_with(
            "{\"escalation_class\":null,\"escalation_reason\":null,\"request_id\":\"a-request\",\
             \"state\":\"Open\",\"repository\":\"cortex\",\"decision_id\":null,"
        )),
        "the view's fields in the specification's order: {raw:?}"
    );
}

#[test]
fn a_view_without_a_store_creates_none() {
    let case = Case::new("no-store");
    for view in ["decisions", "requests", "hands-todo"] {
        let stderr = case.failed(&["decision", view]);
        assert!(stderr.contains("no store"), "{view}: {stderr:?}");
    }
    assert!(!case.state.exists(), "a view created the state directory");
}

// ---------------------------------------------------------------------------------------------
// Wave 06 U7: decision ids (decisions 2 and 3 of its brief) and the markdown rendering
// (decision 5).
// ---------------------------------------------------------------------------------------------

/// Both record commands refuse a `--decided-at` whose UTC date has no year from 1 to 9999, with
/// exit 1 and one line naming the flag, and record nothing: a year of 0, a year of -1 in UTC and
/// a year of 10000 in UTC. Neither panics. The first and last instants of that range are
/// recorded.
#[test]
fn a_decided_at_outside_utc_years_1_to_9999_is_refused() {
    let case = Case::new("decided-at-years");
    case.conductor_decision("C");
    for who in ["record-conductor-decision", "record-operator-decision"] {
        for at in [
            "0000-06-01T00:00:00Z",
            "0000-01-01T00:30:00+01:00",
            "9999-12-31T23:30:00-01:00",
        ] {
            let stderr = case.failed(&Case::record_args(who, "C", at, &[]));
            assert!(stderr.contains("--decided-at"), "{who} {at}: {stderr:?}");
        }
    }
    assert_eq!(case.rows("decisions").len(), 1, "nothing more was recorded");
    assert_eq!(
        case.record(
            "record-conductor-decision",
            "C",
            "0001-01-01T00:00:00Z",
            &[]
        ),
        "DEC-00010101-01"
    );
    assert_eq!(
        case.record("record-operator-decision", "C", "9999-12-31T23:59:59Z", &[]),
        "DEC-99991231-01"
    );
}

/// Class O and H are refused before an id is allocated, so the refusal of a decision recorded
/// without `--decision-id` names no decision id, and the next decision takes the next number.
#[test]
fn a_class_o_or_h_refusal_names_no_allocated_id() {
    let case = Case::new("reserved-names-no-id");
    assert_eq!(case.conductor_decision("C"), "DEC-20261007-01");
    for class in ["O", "H"] {
        let stderr = case.refused(
            &Case::record_args("record-conductor-decision", class, AT, &[]),
            "ReservedForOperator",
        );
        assert!(!stderr.contains("DEC-"), "class {class}: {stderr:?}");
    }
    assert_eq!(case.conductor_decision("C"), "DEC-20261007-02");
}

/// A decision id given by hand is in the allocator's form, `DEC-YYYYMMDD-NN`: a date, then a
/// number of two digits, or one of 100 or more without a leading zero. Any other is refused by
/// both record commands with exit 1, naming the id and the form, before any store is opened; so
/// no two ids carry one number of one date.
#[test]
fn a_hand_given_decision_id_outside_the_allocators_form_is_refused() {
    let case = Case::new("hand-id-form");
    for who in ["record-conductor-decision", "record-operator-decision"] {
        for given in [
            "DEC-20261007-1",
            "DEC-20261007-001",
            "DEC-20261007-0100",
            "DEC-20261007-01a",
            "DEC-20261007-+1",
            "DEC-20261007-",
            "DEC-20261007",
            "DEC-2026107-01",
            "DEC-20261307-01",
            "DEC-20261032-01",
            "DEC-00001007-01",
            "DEC-x",
        ] {
            let stderr = case.failed(&Case::record_args(who, "C", AT, &["--decision-id", given]));
            assert!(
                stderr.contains(&format!("{given:?}")) && stderr.contains("DEC-YYYYMMDD-NN"),
                "{who} {given}: names the id and the form: {stderr:?}"
            );
        }
    }
    assert!(
        !case.state.exists(),
        "a refused id opened the store at {}",
        case.state.display()
    );
    for given in [
        "DEC-20261007-00",
        "DEC-20261007-42",
        "DEC-20261007-100",
        "DEC-20261007-1234",
    ] {
        assert_eq!(
            case.record(
                "record-operator-decision",
                "C",
                AT,
                &["--decision-id", given]
            ),
            given
        );
    }
}

/// Six writers at once, none naming an id. A writer loses an allocated id only to another
/// writer's record, so it allocates again while the date's highest number moves, and all six are
/// recorded, numbered 01 to 06.
#[test]
fn six_concurrent_records_are_all_recorded() {
    let case = Case::new("six-at-once");
    let args = Case::record_args(
        "record-conductor-decision",
        "C",
        "2026-10-11T10:00:00Z",
        &[],
    );
    let children: Vec<_> = (0..6)
        .map(|_| {
            case.command(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the conductor binary starts")
        })
        .collect();
    let mut ids = Vec::new();
    let mut refused = Vec::new();
    for child in children {
        let output = child.wait_with_output().expect("the conductor binary runs");
        if output.status.success() {
            ids.push(
                String::from_utf8(output.stdout)
                    .expect("stdout is UTF-8")
                    .trim_end()
                    .to_owned(),
            );
        } else {
            refused.push(String::from_utf8_lossy(&output.stderr).into_owned());
        }
    }
    ids.sort();
    assert!(
        refused.is_empty()
            && ids
                == [
                    "DEC-20261011-01",
                    "DEC-20261011-02",
                    "DEC-20261011-03",
                    "DEC-20261011-04",
                    "DEC-20261011-05",
                    "DEC-20261011-06",
                ],
        "six at once: recorded {ids:?}; refused {refused:?}"
    );
}

/// The cells of one Markdown table row, each read back as GitHub-flavoured Markdown reads it: a
/// `|` not escaped ends a cell, and `\` escapes the character after it.
fn markdown_cells(row: &str) -> Vec<String> {
    let inner = row
        .strip_prefix("| ")
        .and_then(|row| row.strip_suffix(" |"))
        .expect("a row starts with `| ` and ends with ` |`");
    let mut cells = vec![String::new()];
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                if let Some(next) = characters.next() {
                    cells.last_mut().expect("a cell").push(next);
                }
            }
            '|' => cells.push(String::new()),
            other => cells.last_mut().expect("a cell").push(other),
        }
    }
    cells.iter().map(|cell| cell.trim().to_owned()).collect()
}

/// In `--format markdown`, every value is one cell that reads back as the value, whatever
/// backslashes and pipes it holds: the renderer escapes `\` as well as `|`.
#[test]
fn a_markdown_cell_reads_back_as_its_value() {
    let case = Case::new("markdown-cells");
    let questions = [
        r"grep 'a\|b'?",
        r"a|b",
        r"ends in a backslash\",
        r"\\|",
        r"\|\\|\\\",
        r"C:\path\to",
    ];
    for question in questions {
        let mut args = Case::record_args("record-operator-decision", "C", AT, &[]);
        let at = args
            .iter()
            .position(|arg| *arg == "--question")
            .expect("a --question flag")
            + 1;
        args[at] = question;
        case.ok(&args);
    }
    let table = case.ok(&["decision", "decisions", "--format", "markdown"]);
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(lines.len(), 2 + questions.len(), "{table:?}");
    let header = markdown_cells(lines[0]);
    let question = header
        .iter()
        .position(|column| column == "question")
        .expect("a question column");
    for (row, expected) in lines[2..].iter().zip(questions) {
        let cells = markdown_cells(row);
        assert_eq!(cells.len(), header.len(), "{row:?}");
        assert_eq!(cells[question], expected, "{row:?}");
    }
}
