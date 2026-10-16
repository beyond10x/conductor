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

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use conductor_cli::config::CONFIG_VARIABLE;
use serde_json::{Value, json};

/// The instant most records here are decided or raised at.
const AT: &str = "2026-10-07T10:00:00Z";

/// The session that records an operator's decision here (`--recorded-by`).
const RECORDED_BY: &str = "session-1";

/// The operator's words an operator's decision is recorded on here (`--operator-words`).
const OPERATOR_WORDS: &str = "A, go ahead";

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
    /// `more` after them. An operator's decision also names the session that records it and the
    /// operator's words it is recorded on.
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
        if who == "record-operator-decision" {
            args.extend_from_slice(&[
                "--recorded-by",
                RECORDED_BY,
                "--operator-words",
                OPERATOR_WORDS,
            ]);
        }
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
    case.quiet(
        &[
            &[
                "decision",
                "escalate-request",
                "--request-id",
                "cortex-1",
                "--escalation-class",
                "O",
                "--reason",
                "it changes who decides releases",
            ][..],
            &O_CHOICES[..],
        ]
        .concat(),
    );
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
            &[
                "decision",
                "escalate-request",
                "--request-id",
                "no-such-request",
                "--escalation-class",
                "O",
                "--reason",
                "unclear",
            ][..],
            &O_CHOICES[..],
        ]
        .concat(),
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
        "--no-dry-run",
        "a login has no step that runs without it",
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

    // A decision the operator recorded is theirs to reverse.
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
        let mut args = vec![
            "decision",
            "escalate-request",
            "--request-id",
            request,
            "--escalation-class",
            class,
            "--reason",
            "the operator's",
            "--no-dry-run",
            "no step runs without the operator",
        ];
        if class == "O" {
            args.extend_from_slice(&O_CHOICES);
        }
        case.quiet(&args);
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

/// `story:operator-items-age`: `decision hands-todo` shows each open class-H item's age in whole
/// days since it was raised. The fixture's item is raised two days before the fixture's now, the
/// clock the binary reads too, so it reads `2d`.
#[test]
fn the_hands_todo_shows_each_item_s_age_in_whole_days() {
    let case = Case::new("hands-todo-age");
    let now = time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("a whole second is a valid instant");
    let raised_at = (now - time::Duration::days(2))
        .format(&time::format_description::well_known::Rfc3339)
        .expect("an RFC 3339 instant");
    case.quiet(&[
        "decision",
        "raise-decision-request",
        "--request-id",
        "hands-1",
        "--repository",
        "llm",
        "--question",
        "which token?",
        "--options",
        "A: renew it; B: drop the job",
        "--recommendation",
        "A",
        "--raised-at",
        &raised_at,
    ]);
    case.quiet(&[
        "decision",
        "escalate-request",
        "--request-id",
        "hands-1",
        "--escalation-class",
        "H",
        "--reason",
        "the operator's hands",
        "--no-dry-run",
        "renewing a token has no step that runs without it",
    ]);

    let text = case.ok(&["decision", "hands-todo"]);
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    assert_eq!(
        header.split_whitespace().next_back(),
        Some("age"),
        "the text table ends on an age column: {text:?}"
    );
    assert!(
        lines.any(|line| line.contains("hands-1") && line.trim_end().ends_with("  2d")),
        "the item's row reads 2d: {text:?}"
    );

    let table = case.ok(&["decision", "hands-todo", "--format", "markdown"]);
    assert!(
        table
            .lines()
            .next()
            .is_some_and(|line| line.ends_with("| age |")),
        "the markdown table ends on an age column: {table:?}"
    );
    assert!(
        table
            .lines()
            .any(|line| line.contains("hands-1") && line.ends_with("| 2d |")),
        "the item's markdown row reads 2d: {table:?}"
    );

    // JSON keeps the specification's view: its fields, and no age.
    let todo = case.rows("hands-todo");
    assert_eq!(todo.len(), 1, "{todo:?}");
    assert_eq!(todo[0]["request_id"], "hands-1");
    assert_eq!(todo[0]["raised_at"], raised_at.as_str());
    assert!(todo[0].get("age").is_none(), "{todo:?}");
}

/// `story:hand-item-dry-run`: a class-H escalation carries its dry run (`--dry-run <file>`, the
/// output of every step of the operator's script that needs no credential) or why it has none
/// (`--no-dry-run <reason>`). Without either it is refused naming both flags, and the request
/// stays open; `decision hands-todo` prints each item's dry run or reason. A class-O escalation
/// needs neither.
#[test]
fn a_class_h_escalation_carries_its_dry_run_or_why_it_has_none() {
    let case = Case::new("hand-item-dry-run");
    case.raise("h-untested", "llm");
    case.raise("h-dry", "llm");
    case.raise("h-none", "llm");
    case.raise("o-plain", "llm");
    let escalate = |request: &str, class: &str, more: &[&str]| {
        let mut args = vec![
            "decision",
            "escalate-request",
            "--request-id",
            request,
            "--escalation-class",
            class,
            "--reason",
            "the operator's hands",
        ];
        args.extend_from_slice(more);
        args.into_iter().map(str::to_owned).collect::<Vec<_>>()
    };
    fn strs(args: &[String]) -> Vec<&str> {
        args.iter().map(String::as_str).collect()
    }

    // Neither: refused naming both flags, and nothing moved.
    let untested = escalate("h-untested", "H", &[]);
    let stderr = case.refused(&strs(&untested), "DryRunMissing");
    assert!(
        stderr.contains("--dry-run") && stderr.contains("--no-dry-run"),
        "the refusal names both flags: {stderr:?}"
    );
    assert_eq!(case.request("h-untested")["state"], "Open", "nothing moved");

    // A dry run, read from the file `--dry-run` names.
    let dry = case.work.join("dry-run.txt");
    fs::write(
        &dry,
        "step 1: size check: 12 MiB, under the 25 MiB limit\nstep 2: needs the token, not run\n",
    )
    .expect("write the dry-run file");
    let dry_path = dry.to_str().expect("a UTF-8 path").to_owned();
    case.quiet(&strs(&escalate("h-dry", "H", &["--dry-run", &dry_path])));

    // Or why there is none.
    case.quiet(&strs(&escalate(
        "h-none",
        "H",
        &["--no-dry-run", "every step needs the operator's login"],
    )));

    // A class-O escalation needs neither (it names what the operator chooses from instead,
    // story:decisions-page).
    case.quiet(&strs(&escalate("o-plain", "O", &O_CHOICES)));

    // A dry-run file that does not exist is named, and nothing moved.
    let missing = case.work.join("no-such-dry-run.txt");
    let missing_path = missing.to_str().expect("a UTF-8 path").to_owned();
    let stderr = case.failed(&strs(&escalate(
        "h-untested",
        "H",
        &["--dry-run", &missing_path],
    )));
    assert!(
        stderr.contains("no-such-dry-run.txt"),
        "the unreadable file is named: {stderr:?}"
    );
    assert_eq!(case.request("h-untested")["state"], "Open", "nothing moved");

    let todo = case.rows("hands-todo");
    let row = |request: &str| {
        todo.iter()
            .find(|row| row["request_id"] == request)
            .unwrap_or_else(|| panic!("hands-todo has no row for {request}: {todo:?}"))
            .clone()
    };
    assert_eq!(
        row("h-dry")["dry_run"],
        "step 1: size check: 12 MiB, under the 25 MiB limit\nstep 2: needs the token, not run",
        "{todo:?}"
    );
    assert_eq!(row("h-dry")["no_dry_run"], Value::Null, "{todo:?}");
    assert_eq!(
        row("h-none")["no_dry_run"],
        "every step needs the operator's login",
        "{todo:?}"
    );
    assert_eq!(row("h-none")["dry_run"], Value::Null, "{todo:?}");

    // The table a person reads prints both too.
    let text = case.ok(&["decision", "hands-todo"]);
    for shown in [
        "step 1: size check: 12 MiB, under the 25 MiB limit",
        "every step needs the operator's login",
    ] {
        assert!(text.contains(shown), "hands-todo prints {shown:?}: {text}");
    }
    let header = text.lines().next().unwrap_or_default();
    assert!(
        header.contains("dry_run") && header.contains("no_dry_run"),
        "the table has both columns: {header:?}"
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

/// `story:decision-id-free-in-records`: the instance's hand-written `decisions/*.jsonl` in the
/// records directory its config file names hold `DEC-20261007-01` and `-02`, which the store does
/// not. Given no `--decision-id`, each record command allocates an id free in both: `-03`, then
/// `-04`. A line that does not read, a file that is not `.jsonl` and another date's id are passed
/// over.
#[test]
fn an_allocated_decision_id_is_free_in_the_hand_written_decision_files() {
    let case = Case::new("ids-free-in-records");
    let root = case.work.parent().expect("the case's directory").to_owned();
    let decisions = root.join("home/records/decisions");
    fs::create_dir_all(&decisions).expect("create the records' decisions directory");
    fs::write(
        decisions.join("2026-10.jsonl"),
        "{\"id\":\"DEC-20261007-01\",\"class\":\"C\"}\n\
         not a JSON line\n\
         {\"id\":\"DEC-20261007-02\",\"class\":\"R\"}\n\
         {\"id\":\"DEC-20261008-09\",\"class\":\"C\"}\n",
    )
    .expect("write the hand-written decisions");
    fs::write(
        decisions.join("notes.txt"),
        "{\"id\":\"DEC-20261007-50\"}\n",
    )
    .expect("write a file that is not jsonl");
    let config = root.join("conductor.yaml");
    // The instance's `state` is the one `--state-dir` names: a command that writes records takes
    // no other (`story:instance-isolation`).
    fs::write(
        &config,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: ~/checkouts, trees: ~/trees}}\n\
             \x20   records: ~/records\n\
             \x20   state: {:?}\n",
            case.state
        ),
    )
    .expect("write the config file");

    let mut ids = Vec::new();
    for who in ["record-conductor-decision", "record-operator-decision"] {
        let args = Case::record_args(who, "C", AT, &[]);
        let output = case
            .command(&args)
            .env(CONFIG_VARIABLE, &config)
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "`conductor {}` exited {:?}; stderr: {}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        ids.push(
            String::from_utf8(output.stdout)
                .expect("stdout is UTF-8")
                .trim_end()
                .to_owned(),
        );
    }
    assert_eq!(ids, ["DEC-20261007-03", "DEC-20261007-04"]);
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
        "--no-dry-run",
        "setting a secret has no step that runs without it",
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
    let missing: [(&[&str], &str); 10] = [
        (
            &["decision", "amend-escalated-request", "--request-id", "r"],
            "--reason",
        ),
        (
            &["decision", "amend-escalated-request", "--reason", "x"],
            "--request-id",
        ),
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
        "--kind",
        "one-of",
        "--lettered-options",
        LETTERED,
        "--recommended-letters",
        "A",
        "--default-letters",
        "B",
        "--deadline",
        DEADLINE,
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
             \"evidence\":\"the request's own text\",\"decided_at\":\"{AT}\",\
             \"recorded_by\":null,\"operator_words\":null,\"repository\":null}}"
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
    // Eleven fields, and the five of what the operator chooses from (story:decisions-page).
    assert_eq!(keys.len(), 16);
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

/// `story:operator-decision-guarded`: an operator's decision keeps the session that recorded it
/// (`--recorded-by`) and the operator's words it was recorded on (`--operator-words`). Both are
/// required: one left out or blank is refused, naming its flag, before any store is opened.
/// `decision show` prints both, and a decision conductor takes carries neither.
#[test]
fn an_operator_decision_keeps_the_session_that_recorded_it_and_their_words() {
    let case = Case::new("operator-evidence");
    let full = Case::record_args("record-operator-decision", "O", AT, &[]);
    for flag in ["--recorded-by", "--operator-words"] {
        let at = full
            .iter()
            .position(|arg| *arg == flag)
            .expect("the flag is given");
        let missing = [&full[..at], &full[at + 2..]].concat();
        let stderr = case.failed(&missing);
        assert!(
            stderr.contains(flag),
            "a missing {flag} is named: {stderr:?}"
        );
        for blank in ["", "  "] {
            let mut empty = full.clone();
            empty[at + 1] = blank;
            let stderr = case.failed(&empty);
            assert!(stderr.contains(flag), "a blank {flag} is named: {stderr:?}");
        }
    }
    assert!(
        !case.state.exists(),
        "a refused operator's decision opened the store at {}",
        case.state.display()
    );

    let operators = case.operator_decision("O");
    let conductors = case.conductor_decision("C");
    let row = case.decision(&operators);
    assert_eq!(row["decided_by"], "Operator");
    assert_eq!(row["recorded_by"], RECORDED_BY);
    assert_eq!(row["operator_words"], OPERATOR_WORDS);
    let row = case.decision(&conductors);
    assert_eq!(row["recorded_by"], Value::Null);
    assert_eq!(row["operator_words"], Value::Null);

    let shown = case.ok(&["decision", "show", "--decision-id", &operators]);
    for line in [
        format!("recorded_by: {RECORDED_BY}"),
        format!("operator_words: {OPERATOR_WORDS}"),
    ] {
        assert!(
            shown.lines().any(|printed| printed == line),
            "`decision show` prints {line:?}: {shown:?}"
        );
    }
    let object: Value = serde_json::from_str(&case.ok(&[
        "decision",
        "show",
        "--decision-id",
        &operators,
        "--format",
        "json",
    ]))
    .expect("one JSON object");
    assert_eq!(object["recorded_by"], RECORDED_BY);
    assert_eq!(object["operator_words"], OPERATOR_WORDS);
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

/// The first event file, in name order, of any stream in the tree store under `state`.
fn an_event_file(state: &Path) -> PathBuf {
    fn first(directory: &Path) -> Option<PathBuf> {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("list a stream directory")
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        entries.sort();
        entries.into_iter().find_map(|path| {
            if path.is_dir() {
                first(&path)
            } else {
                path.extension()
                    .is_some_and(|ext| ext == "json")
                    .then_some(path)
            }
        })
    }
    first(&state.join("tree/tenants/conductor/streams")).expect("the tree holds an event file")
}

/// `story:store-open-under-lock`: a record command opens the store under the tree's writer lock.
/// While the test holds `state/tree/.lock`, the tree is as a commit in progress leaves it to a
/// reader that lists event files and commit files in two passes: a commit file names an event the
/// first pass did not see (here, one moved out of the tree). Opened without the lock, that reads
/// as a corrupt store and the command is refused; opened under it, the command waits, and records
/// once the tree is whole again and the lock is released.
#[test]
fn a_record_waits_for_the_stores_lock_before_it_opens_the_store() {
    let case = Case::new("open-under-lock");
    case.conductor_decision("C");
    let lock = File::options()
        .read(true)
        .write(true)
        .open(case.state.join("tree/.lock"))
        .expect("open the store's writer lock");
    lock.lock().expect("hold the writer lock");
    let event = an_event_file(&case.state);
    let aside = case.work.with_file_name("aside.json");
    fs::rename(&event, &aside).expect("move an event file out of the tree");

    let args = Case::record_args(
        "record-conductor-decision",
        "C",
        "2026-10-12T10:00:00Z",
        &[],
    );
    let mut child = case
        .command(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the conductor binary starts");
    thread::sleep(Duration::from_millis(500));
    let early = child.try_wait().expect("poll the record command");

    fs::rename(&aside, &event).expect("put the event file back");
    lock.unlock().expect("release the writer lock");
    let output = child.wait_with_output().expect("the conductor binary runs");
    assert_eq!(
        early,
        None,
        "the record did not wait for the lock: exit {:?}, stderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "the record failed once the lock was released: exit {:?}, stderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "DEC-20261012-01\n");
    assert_eq!(case.rows("decisions").len(), 2);
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

/// `story:dashboard-repo-view`: a decision names the repository it is about with `--repository`,
/// on either record command; the `decisions` view and `decision show` carry it, last, and a
/// decision recorded without it carries `null`.
#[test]
fn a_decision_names_its_repository_and_the_decisions_view_reads_it_back() {
    let case = Case::new("decision-repository");
    let conductors = case.record(
        "record-conductor-decision",
        "C",
        AT,
        &["--repository", "alpha"],
    );
    let operators = case.record(
        "record-operator-decision",
        "O",
        AT,
        &["--repository", "beta"],
    );
    let without = case.conductor_decision("C");

    assert_eq!(case.decision(&conductors)["repository"], "alpha");
    assert_eq!(case.decision(&operators)["repository"], "beta");
    assert_eq!(case.decision(&without)["repository"], Value::Null);

    let raw = case.ok(&["decision", "decisions", "--format", "jsonl"]);
    let first = raw.lines().next().expect("a row");
    assert!(
        first.ends_with(",\"operator_words\":null,\"repository\":\"alpha\"}"),
        "repository is the view's last field: {first:?}"
    );
    let shown = case.ok(&["decision", "show", "--decision-id", &operators]);
    assert_eq!(
        shown.lines().last(),
        Some("repository: beta"),
        "`decision show` prints it last: {shown:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// What the operator chooses from (story:decisions-page)
// ---------------------------------------------------------------------------------------------

/// The lettered options a class-O escalation here names: two, each with its cost.
const LETTERED: &str = r#"[{"letter":"A","label":"one way","cost":"an hour of one controller"},{"letter":"B","label":"the other","cost":"nothing now"}]"#;

/// The deadline a class-O escalation here names.
const DEADLINE: &str = "2026-10-08T10:00:00Z";

/// What a class-O escalation carries besides its class and reason (`story:decisions-page`): a
/// `one-of` with [`LETTERED`], `A` recommended, `B` the default, and [`DEADLINE`].
const O_CHOICES: [&str; 10] = [
    "--kind",
    "one-of",
    "--lettered-options",
    LETTERED,
    "--recommended-letters",
    "A",
    "--default-letters",
    "B",
    "--deadline",
    DEADLINE,
];

/// The arguments of `escalate-request` of `request` as `class`, with `more` after them.
fn escalation<'a>(request: &'a str, class: &'a str, more: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "decision",
        "escalate-request",
        "--request-id",
        request,
        "--escalation-class",
        class,
        "--reason",
        "the operator's call",
    ];
    args.extend_from_slice(more);
    args
}

/// `story:decisions-page`: a class-O escalation carries its kind, its lettered options with their
/// cost, the recommended and default letters and its deadline; without any one of them it is
/// refused with `EscalationIncomplete`, naming the flags, and the request stays open. With them
/// the `requests` view carries each, last in its row.
#[test]
fn a_class_o_escalation_names_its_kind_options_recommendation_default_and_deadline() {
    let case = Case::new("escalation-choices");
    case.raise("o-1", "alpha");
    for left_out in 0..5 {
        let mut more: Vec<&str> = Vec::new();
        for (pair, flag) in O_CHOICES.chunks(2).enumerate() {
            if pair != left_out {
                more.extend_from_slice(flag);
            }
        }
        let stderr = case.refused(&escalation("o-1", "O", &more), "EscalationIncomplete");
        for flag in [
            "--kind",
            "--lettered-options",
            "--recommended-letters",
            "--default-letters",
            "--deadline",
        ] {
            assert!(
                stderr.contains(flag),
                "the refusal names {flag}: {stderr:?}"
            );
        }
        assert_eq!(case.request("o-1")["state"], "Open", "nothing moved");
    }

    case.quiet(&escalation("o-1", "O", &O_CHOICES));
    let row = case.request("o-1");
    assert_eq!(row["state"], "Escalated");
    assert_eq!(row["kind"], "one-of");
    assert_eq!(
        row["lettered_options"],
        serde_json::from_str::<Value>(LETTERED).expect("the options are JSON")
    );
    assert_eq!(row["recommended_letters"], "A");
    assert_eq!(row["default_letters"], "B");
    assert_eq!(row["deadline"], DEADLINE);
    let raw = case.ok(&["decision", "requests", "--format", "jsonl"]);
    assert!(
        raw.lines()
            .next()
            .is_some_and(|line| line.ends_with(&format!("\"deadline\":\"{DEADLINE}\"}}"))),
        "the deadline is the view's last field: {raw:?}"
    );
}

/// `story:decisions-page`: a class-H escalation may carry the same, and needs none of it; a
/// request escalated without them reads them back as `null`.
#[test]
fn a_class_h_escalation_may_name_its_options_and_needs_none() {
    let case = Case::new("escalation-choices-h");
    case.raise("h-bare", "alpha");
    case.raise("h-full", "alpha");
    case.quiet(&escalation(
        "h-bare",
        "H",
        &["--no-dry-run", "a login has no step that runs without it"],
    ));
    let full = escalation(
        "h-full",
        "H",
        &[
            "--no-dry-run",
            "a login has no step that runs without it",
            "--kind",
            "yes-no",
            "--lettered-options",
            r#"[{"letter":"Y","label":"log in now","cost":"five minutes of the operator"},{"letter":"N","label":"leave it","cost":"the release waits"}]"#,
            "--recommended-letters",
            "Y",
        ],
    );
    case.quiet(&full);

    let bare = case.request("h-bare");
    for field in [
        "kind",
        "lettered_options",
        "recommended_letters",
        "default_letters",
        "deadline",
    ] {
        assert_eq!(bare[field], Value::Null, "{field}: {bare}");
    }
    let full = case.request("h-full");
    assert_eq!(full["kind"], "yes-no");
    assert_eq!(full["recommended_letters"], "Y");
    assert_eq!(full["default_letters"], Value::Null);
    assert_eq!(full["lettered_options"][1]["letter"], "N");
}

/// `story:decisions-page`: options that do not fit their kind are refused with `OptionsInvalid`
/// before any store is opened, naming what is wrong, and the request stays open.
#[test]
fn options_that_do_not_fit_their_kind_are_refused() {
    let case = Case::new("escalation-choices-invalid");
    case.raise("o-bad", "alpha");
    let with = |kind: &'static str,
                options: &'static str,
                recommended: &'static str,
                default: &'static str| {
        vec![
            "--kind",
            kind,
            "--lettered-options",
            options,
            "--recommended-letters",
            recommended,
            "--default-letters",
            default,
            "--deadline",
            DEADLINE,
        ]
    };
    let two = LETTERED;
    let doubled =
        r#"[{"letter":"A","label":"one","cost":"x"},{"letter":"A","label":"two","cost":"y"}]"#;
    let costless =
        r#"[{"letter":"A","label":"one","cost":""},{"letter":"B","label":"two","cost":"y"}]"#;
    let unlabelled = r#"[{"letter":"A","cost":"x"}]"#;
    for (more, says) in [
        (with("one-of", two, "C", "B"), "C"),
        (with("one-of", two, "A", "Z"), "Z"),
        (with("one-of", two, "A,B", "B"), "one letter"),
        (with("yes-no", two, "A", "B"), "Y and N"),
        (with("several-of", two, "", "B"), "at least one"),
        (with("one-of", doubled, "A", "A"), "\"A\" twice"),
        (with("one-of", costless, "A", "B"), "cost"),
        (with("one-of", unlabelled, "A", "A"), "label"),
        (with("one-of", "[]", "A", "A"), "no option"),
    ] {
        let stderr = case.refused(&escalation("o-bad", "O", &more), "OptionsInvalid");
        assert!(stderr.contains(says), "says {says:?}: {stderr:?}");
        assert_eq!(case.request("o-bad")["state"], "Open", "nothing moved");
    }

    // Several letters fit a several-of request.
    case.quiet(&escalation(
        "o-bad",
        "O",
        &with("several-of", two, "A,B", "A"),
    ));
    assert_eq!(case.request("o-bad")["recommended_letters"], "A,B");
}

/// The arguments of `amend-escalated-request` of `request`, with `more` after them.
fn amendment<'a>(request: &'a str, more: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "decision",
        "amend-escalated-request",
        "--request-id",
        request,
        "--reason",
        "the profile asks for a default",
    ];
    args.extend_from_slice(more);
    args
}

/// `story:request-amend-escalated`: an escalated request is completed in place. A field given
/// replaces the held one, a field left out stays, and the `requests` view reads the result.
#[test]
fn an_escalated_request_is_amended_in_place() {
    let case = Case::new("amend-escalated");
    case.raise("a-1", "alpha");
    case.quiet(&escalation("a-1", "O", &O_CHOICES));

    case.quiet(&amendment(
        "a-1",
        &[
            "--default-letters",
            "A",
            "--deadline",
            "2026-10-20T10:00:00Z",
        ],
    ));
    let row = case.request("a-1");
    assert_eq!(row["state"], "Escalated");
    assert_eq!(row["default_letters"], "A");
    assert_eq!(row["deadline"], "2026-10-20T10:00:00Z");
    assert_eq!(row["kind"], "one-of", "a field left out stays");
    assert_eq!(row["recommended_letters"], "A", "a field left out stays");
    assert_eq!(
        row["lettered_options"],
        serde_json::from_str::<Value>(LETTERED).expect("the options are JSON"),
        "the options are left as they were"
    );

    case.quiet(&amendment(
        "a-1",
        &[
            "--kind",
            "several-of",
            "--lettered-options",
            r#"[{"letter":"A","label":"keep the file store","cost":"nothing now"},{"letter":"C","label":"a third way","cost":"a day"}]"#,
            "--recommended-letters",
            "A,C",
            "--default-letters",
            "C",
            "--dry-run",
            "nothing to run",
        ],
    ));
    let row = case.request("a-1");
    assert_eq!(row["kind"], "several-of");
    assert_eq!(row["recommended_letters"], "A,C");
    assert_eq!(row["default_letters"], "C");
    assert_eq!(
        row["lettered_options"],
        json!([
            {"letter": "A", "label": "keep the file store", "cost": "nothing now"},
            {"letter": "C", "label": "a third way", "cost": "a day"},
        ])
    );
}

/// `story:request-amend-escalated`: an open, answered or withdrawn request is refused with
/// `RequestStateConflict` and stays as it was; so is one nobody raised.
#[test]
fn only_an_escalated_request_is_amended() {
    let case = Case::new("amend-not-escalated");
    case.raise("open-1", "alpha");
    let stderr = case.refused(
        &amendment("open-1", &["--default-letters", "A"]),
        "RequestStateConflict",
    );
    assert!(stderr.contains("open-1"), "{stderr:?}");
    assert_eq!(case.request("open-1")["default_letters"], Value::Null);

    case.raise("answered-1", "alpha");
    case.quiet(&escalation("answered-1", "O", &O_CHOICES));
    let decision = case.operator_decision("O");
    case.quiet(&[
        "decision",
        "answer-escalated-request",
        "--request-id",
        "answered-1",
        "--decision-id",
        &decision,
    ]);
    case.refused(
        &amendment("answered-1", &["--default-letters", "A"]),
        "RequestStateConflict",
    );
    assert_eq!(case.request("answered-1")["default_letters"], "B");

    case.raise("gone-1", "alpha");
    case.quiet(&[
        "decision",
        "withdraw-request",
        "--request-id",
        "gone-1",
        "--reason",
        "x",
    ]);
    case.refused(
        &amendment("gone-1", &["--default-letters", "A"]),
        "RequestStateConflict",
    );

    case.refused(
        &amendment("nobody-1", &["--default-letters", "A"]),
        "RequestNotFound",
    );
}

/// `story:request-amend-escalated`: a default or recommended letter that names no option, an
/// option without a cost and letters that do not fit the kind are refused with `OptionsInvalid`,
/// and the request is as it was.
#[test]
fn an_amendment_naming_an_unknown_letter_is_refused() {
    let case = Case::new("amend-invalid");
    case.raise("bad-1", "alpha");
    case.quiet(&escalation("bad-1", "O", &O_CHOICES));
    for (more, says) in [
        (vec!["--default-letters", "Z"], "Z"),
        (vec!["--recommended-letters", "Q"], "Q"),
        (
            vec![
                "--lettered-options",
                r#"[{"letter":"A","label":"one","cost":"x"}]"#,
            ],
            "\"B\"",
        ),
        (
            vec![
                "--lettered-options",
                r#"[{"letter":"A","label":"one","cost":""},{"letter":"B","label":"two","cost":"y"}]"#,
            ],
            "cost",
        ),
        (vec!["--kind", "yes-no"], "Y and N"),
        (vec!["--default-letters", "A,B"], "one letter"),
    ] {
        let stderr = case.refused(&amendment("bad-1", &more), "OptionsInvalid");
        assert!(stderr.contains(says), "says {says:?}: {stderr:?}");
        let row = case.request("bad-1");
        assert_eq!(row["default_letters"], "B", "nothing moved");
        assert_eq!(row["kind"], "one-of", "nothing moved");
    }
}
