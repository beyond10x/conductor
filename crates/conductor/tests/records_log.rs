//! `story:records-from-commands`: each command that records a dispatch, report, message, resource
//! answer, charter revision, decision or request appends exactly one line to the records' log of
//! its kind, `dispatches/YYYY-MM.jsonl` or `decisions/YYYY-MM.jsonl`, with the repository under
//! the key `repo`; and `conductor records check` names a decision id the dispatch log cites that
//! nothing holds, a line with an unknown key set, and a decision line in the dispatch log.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`, the
//! `HOME` the binary runs with; `records/`, the instance's records directory, where the binary
//! runs; `state/`; and the config file `conductor.yaml` naming them, under the session prefix
//! `ex`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{Value, json};

/// A day no real record carries, for the decision ids a fixture writes by hand.
const DAY: &str = "20991231";

/// The instant every command of a case is given.
const AT: &str = "2026-10-07T09:00:00Z";

/// A decision id of [`DAY`] numbered `number`, in the allocator's form.
fn decision_id(number: u8) -> String {
    format!("DEC-{DAY}-{number:02}")
}

struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("records_log")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "state"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        fs::write(
            case.config(),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n",
                root = case.dir.join("home/example-org").display(),
                trees = case.dir.join("trees").display(),
                records = case.records().display(),
                state = case.dir.join("state").display(),
            ),
        )
        .expect("write the config file");
        case
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// `conductor <args>` from the records directory, with the case's home and config file.
    fn run(&self, args: &[&str]) -> Output {
        common::conductor(self.dir.join("home"))
            .args(args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.config())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// Runs a command that must succeed, and answers its standard output, trimmed.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}`: {}",
            args.join(" "),
            describe(&output)
        );
        String::from_utf8(output.stdout)
            .expect("stdout is UTF-8")
            .trim()
            .to_owned()
    }

    /// Runs a command that must succeed and append exactly one line to `log`, and answers that
    /// line, after checking it is in the file of the month of its `at` and spells the repository
    /// `repo` only.
    fn appends(&self, log: &str, args: &[&str]) -> (String, Value) {
        let before = lines(&self.records().join(log));
        let printed = self.ok(args);
        let after = lines(&self.records().join(log));
        assert_eq!(
            after.len(),
            before.len() + 1,
            "`conductor {}` appends exactly one line to {log}/: {after:#?}",
            args.join(" ")
        );
        let (file, line) = after.last().expect("a line").clone();
        let at = line["at"].as_str().expect("the line has an at");
        assert_eq!(
            file,
            format!("{}.jsonl", &at[..7]),
            "the line is in its month's file: {line}"
        );
        assert!(line.get("repo").is_some(), "the line has repo: {line}");
        assert!(
            line.get("repository").is_none(),
            "the line never spells repository: {line}"
        );
        (printed, line)
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

/// Every line of every `*.jsonl` file of `dir`, files in name order, each with its file's name.
fn lines(dir: &Path) -> Vec<(String, Value)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|path| {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            fs::read_to_string(path)
                .expect("read a log file")
                .lines()
                .map(|line| {
                    (
                        name.clone(),
                        serde_json::from_str(line).expect("each log line is JSON"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Asserts `line` holds each of `expected`'s keys with its value.
fn holds(line: &Value, expected: &Value) {
    for (key, value) in expected.as_object().expect("an object") {
        assert_eq!(&line[key], value, "{key} of {line}");
    }
}

#[test]
fn each_command_that_records_a_dispatch_event_appends_one_line_to_the_dispatch_log() {
    let case = Case::new("dispatch-log");
    case.ok(&[
        "goal",
        "propose-goal",
        "--goal-id",
        "G1",
        "--title",
        "Ship the pilot",
        "--exit-evidence",
        "G1 is listed as met",
    ]);
    let send = |brief: &str| {
        case.appends(
            "dispatches",
            &[
                "dispatch",
                "send-dispatch",
                "--repository",
                "alpha",
                "--goal-id",
                "G1",
                "--brief",
                brief,
                "--sent-at",
                AT,
                "--priority",
                "2",
            ],
        )
    };
    let report = |command: &str, dispatch: &str, extra: &[&str]| {
        let mut args = vec!["dispatch", command, "--dispatch-id", dispatch];
        args.extend_from_slice(extra);
        case.appends("dispatches", &args).1
    };

    let (first, line) = send("Expose the store id");
    holds(
        &line,
        &json!({"event": "sent", "repo": "alpha", "id": first, "to": "ex-alpha", "goal": "G1",
                "brief": "Expose the store id", "priority": 2}),
    );
    holds(
        &report("report-started", &first, &[]),
        &json!({"event": "started", "repo": "alpha", "id": first}),
    );
    holds(
        &report("report-blocked", &first, &["--reason", "waits on beta"]),
        &json!({"event": "blocked", "repo": "alpha", "id": first, "reason": "waits on beta"}),
    );
    holds(
        &report("report-unblocked", &first, &[]),
        &json!({"event": "unblocked", "repo": "alpha", "id": first}),
    );
    holds(
        &report("report-done", &first, &["--evidence", "CI green on main"]),
        &json!({"event": "done", "repo": "alpha", "id": first, "evidence": "CI green on main"}),
    );

    let (second, _) = send("Split the reader");
    report("report-started", &second, &[]);
    holds(
        &report("report-failed", &second, &["--reason", "no slot"]),
        &json!({"event": "failed", "repo": "alpha", "id": second, "reason": "no slot"}),
    );
    let (third, _) = send("Rename the flag");
    holds(
        &report("cancel-dispatch", &third, &["--reason", "superseded"]),
        &json!({"event": "cancelled", "repo": "alpha", "id": third, "reason": "superseded"}),
    );

    let first_line = format!("[REPORT alpha {first}] done CI green on main");
    let (message, line) = case.appends(
        "dispatches",
        &[
            "message",
            "receive-message",
            "--sender",
            "ex-alpha",
            "--recipient",
            "conductor",
            "--first-line",
            &first_line,
            "--transport",
            "SendMessage",
            "--received-at",
            AT,
        ],
    );
    holds(
        &line,
        &json!({"event": "message-received", "repo": "alpha", "id": message, "from": "ex-alpha",
                "to": "conductor", "kind": "Report", "first_line": first_line}),
    );

    let request = |amount: &str| {
        case.ok(&[
            "resource",
            "request-resource",
            "--repository",
            "alpha",
            "--resource",
            "Disk",
            "--amount",
            amount,
            "--requested-at",
            AT,
        ])
    };
    let granted = request("10");
    let (_, line) = case.appends(
        "dispatches",
        &[
            "resource",
            "grant-resource",
            "--resource-request-id",
            &granted,
            "--disk-free-bytes",
            "64424509440",
        ],
    );
    holds(
        &line,
        &json!({"event": "resource-granted", "repo": "alpha", "id": granted, "resource": "Disk",
                "amount": 10}),
    );
    let refused = request("20");
    let (_, line) = case.appends(
        "dispatches",
        &[
            "resource",
            "refuse-resource",
            "--resource-request-id",
            &refused,
            "--reason",
            "the disk is short",
        ],
    );
    holds(
        &line,
        &json!({"event": "resource-refused", "repo": "alpha", "id": refused, "resource": "Disk",
                "amount": 20, "reason": "the disk is short"}),
    );

    let controller = case.ok(&[
        "controller",
        "start-controller",
        "--repository",
        "alpha",
        "--harness",
        "Claude",
    ]);
    let (_, line) = case.appends(
        "dispatches",
        &[
            "controller",
            "revise-charter",
            "--controller-id",
            &controller,
        ],
    );
    holds(
        &line,
        &json!({"event": "charter-revised", "repo": "alpha", "controller_id": controller,
                "to": "ex-alpha", "charter_revision": 2}),
    );

    assert!(
        lines(&case.records().join("decisions")).is_empty(),
        "no dispatch event is written to the decision log"
    );
    let output = case.run(&["records", "check"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the lines the commands wrote pass the check: {}",
        describe(&output)
    );
}

#[test]
fn each_command_that_records_a_decision_or_request_appends_one_line_to_the_decision_log() {
    let case = Case::new("decision-log");
    let (decision, line) = case.appends(
        "decisions",
        &[
            "decision",
            "record-conductor-decision",
            "--decision-class",
            "C",
            "--question",
            "Which first?",
            "--options",
            "A or B",
            "--choice",
            "A",
            "--reason",
            "reversible",
            "--evidence",
            "the plan",
            "--decided-at",
            AT,
            "--repository",
            "alpha",
        ],
    );
    holds(
        &line,
        &json!({"event": "decided", "repo": "alpha", "id": decision, "class": "C",
                "decided_by": "Conductor", "question": "Which first?", "options": "A or B",
                "choice": "A", "reason": "reversible", "evidence": "the plan"}),
    );

    let (_, line) = case.appends(
        "decisions",
        &[
            "decision",
            "raise-decision-request",
            "--request-id",
            "alpha-1",
            "--repository",
            "alpha",
            "--question",
            "Which store?",
            "--options",
            "A or B",
            "--recommendation",
            "A",
            "--raised-at",
            AT,
        ],
    );
    holds(
        &line,
        &json!({"event": "requested", "repo": "alpha", "id": "alpha-1",
                "question": "Which store?", "options": "A or B", "recommendation": "A"}),
    );

    let (_, line) = case.appends(
        "decisions",
        &[
            "decision",
            "answer-request",
            "--request-id",
            "alpha-1",
            "--decision-id",
            &decision,
        ],
    );
    holds(
        &line,
        &json!({"event": "answered", "repo": "alpha", "id": "alpha-1", "decision_id": decision}),
    );

    let (_, line) = case.appends(
        "decisions",
        &[
            "decision",
            "reverse-conductor-decision",
            "--decision-id",
            &decision,
            "--reason",
            "the plan changed",
        ],
    );
    holds(
        &line,
        &json!({"event": "reversed", "repo": "alpha", "id": decision,
                "decided_by": "Conductor", "reason": "the plan changed"}),
    );

    assert!(
        lines(&case.records().join("dispatches")).is_empty(),
        "no decision event is written to the dispatch log"
    );
    let output = case.run(&["records", "check"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the lines the commands wrote pass the check: {}",
        describe(&output)
    );
}

#[test]
fn a_refused_command_appends_nothing() {
    let case = Case::new("refused");
    let output = case.run(&["dispatch", "report-started", "--dispatch-id", "DSP-1"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        lines(&case.records().join("dispatches")).is_empty(),
        "a refusal writes no line"
    );
}

/// Without a config file the records are the working directory: a command run where `conductor
/// init` made no `dispatches/` plants none.
#[test]
fn without_a_config_file_a_command_plants_no_log_in_the_working_directory() {
    let case = Case::new("no-config");
    let work = case.dir.join("work");
    fs::create_dir_all(&work).expect("create the working directory");
    let run = |args: &[&str]| {
        let output = common::conductor(case.dir.join("home"))
            .args(["--state-dir", &case.dir.join("state").display().to_string()])
            .args(args)
            .current_dir(&work)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs");
        assert!(output.status.success(), "{}", describe(&output));
    };
    run(&[
        "decision",
        "raise-decision-request",
        "--request-id",
        "alpha-1",
        "--repository",
        "alpha",
        "--question",
        "Which store?",
        "--options",
        "A or B",
        "--recommendation",
        "A",
        "--raised-at",
        AT,
    ]);
    let planted: Vec<PathBuf> = fs::read_dir(&work)
        .expect("read the working directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert!(planted.is_empty(), "nothing planted: {planted:?}");
}

/// Over a fixture with one of each, `records check` prints three findings, one per line, and
/// exits 1: a decision id cited in `dispatches/` that neither `decisions/` nor the store holds, a
/// line with an unknown key set, and a decision line in the dispatch file.
#[test]
fn records_check_names_an_unknown_decision_an_unknown_key_set_and_a_decision_in_the_dispatch_log() {
    let case = Case::new("check");
    let known = decision_id(1);
    let unknown = decision_id(2);
    let decided = json!({"at": AT, "event": "decided", "repo": "alpha", "id": known, "class": "C",
                         "decided_by": "Conductor", "choice": "A", "reason": "reversible"});
    fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
    fs::write(
        case.records().join("decisions/2026-10.jsonl"),
        format!("{decided}\n"),
    )
    .expect("write the decision log");
    let dispatches = [
        // Clean: it cites a decision the decision log holds.
        json!({"at": AT, "event": "blocked", "repo": "alpha", "id": "DSP-1",
               "reason": format!("waits on {known}")}),
        // UnknownDecision.
        json!({"at": AT, "event": "unblocked", "repo": "alpha", "id": "DSP-1",
               "reason": format!("answered by {unknown}")}),
        // UnknownKeySet: `repository`, never `repo`.
        json!({"at": AT, "event": "started", "repository": "alpha", "id": "DSP-2"}),
        // DecisionInDispatchLog, and only that.
        decided,
    ];
    fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
    fs::write(
        case.records().join("dispatches/2026-10.jsonl"),
        dispatches
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write the dispatch log");

    let output = case.run(&["records", "check"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let findings: Vec<&str> = stdout.lines().collect();
    assert_eq!(findings.len(), 3, "three findings: {stdout}");
    assert!(
        findings[0].starts_with("dispatches/2026-10.jsonl:2: UnknownDecision: ")
            && findings[0].contains(&unknown),
        "{findings:?}"
    );
    assert!(
        findings[1].starts_with("dispatches/2026-10.jsonl:3: UnknownKeySet: ")
            && findings[1].contains("repository"),
        "{findings:?}"
    );
    assert!(
        findings[2].starts_with("dispatches/2026-10.jsonl:4: DecisionInDispatchLog: "),
        "{findings:?}"
    );

    let output = case.run(&["records", "check", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let kinds: Vec<&str> = rows
        .as_array()
        .expect("an array")
        .iter()
        .map(|row| row["finding"].as_str().expect("a finding"))
        .collect();
    assert_eq!(
        kinds,
        ["UnknownDecision", "UnknownKeySet", "DecisionInDispatchLog"]
    );
    assert!(
        !case.dir.join("state/tree").exists(),
        "records check is a view: it creates no store"
    );
}

/// Lines written by hand before the first line a command wrote are legacy: `records check`
/// counts them on standard error and does not judge their key set, so a log kept by hand before
/// the commands wrote it does not fail every check after the upgrade.
#[test]
fn records_check_does_not_judge_the_hand_written_lines_before_the_first_command_line() {
    let case = Case::new("check-legacy");
    let known = decision_id(1);
    let decided = json!({"at": AT, "event": "decided", "repo": "alpha", "id": known, "class": "C",
                         "decided_by": "Conductor", "choice": "A", "reason": "reversible"});
    let hand = json!({"ts": AT, "decision": known, "repository": "alpha"});
    fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
    fs::write(
        case.records().join("decisions/2026-10.jsonl"),
        format!("{hand}\n{decided}\n"),
    )
    .expect("write the decision log");
    let dispatches = [
        // Legacy: a hand-written line with another key set.
        json!({"ts": AT, "verb": "sent", "repository": "alpha", "dispatch": "DSP-1"}),
        // The first line a command wrote.
        json!({"at": AT, "event": "blocked", "repo": "alpha", "id": "DSP-1",
               "reason": format!("waits on {known}")}),
        // After it, a line with an unknown key set is a finding again.
        json!({"at": AT, "event": "started", "repository": "alpha", "id": "DSP-2"}),
    ];
    fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
    fs::write(
        case.records().join("dispatches/2026-10.jsonl"),
        dispatches
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .expect("write the dispatch log");

    let output = case.run(&["records", "check"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let findings: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        findings.len(),
        1,
        "only the line after the first command line: {stdout}"
    );
    assert!(
        findings[0].starts_with("dispatches/2026-10.jsonl:3: UnknownKeySet: "),
        "{findings:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("2 line(s) written before the first a command wrote"),
        "{}",
        describe(&output)
    );
}
