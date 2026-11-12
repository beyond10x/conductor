//! `story:control-plane-work-handler`, the model and the dispatch commands: a work source hands an
//! instance one unit of work as a dispatch that carries the caller's `reference` instead of a
//! goal, and a link (a merge or pull request URL) is recorded on a dispatch.
//!
//! - `dispatch send-dispatch --reference <r>` without `--goal-id` sends a dispatch; the same
//!   reference again creates nothing and prints the id of the dispatch that holds it, whatever its
//!   state; a reference for a repository with an open dispatch of another reference or none is
//!   refused (`RepositoryBusy`); neither a goal nor a reference is refused
//!   (`GoalOrReferenceMissing`); a reference outside 1 to 64 characters of `A-Z a-z 0-9 . _ -`,
//!   or one that starts with none of the letters and digits, is refused (`InvalidReference`).
//! - `dispatch record-link` appends a link the dispatch does not hold, is a no-op for one it holds,
//!   and refuses an unknown dispatch, a text that is no `http://` or `https://` URL or is longer
//!   than 2048 characters, and a 33rd link.
//! - `records check` accepts every line these commands write, and still names a key no line
//!   declares.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`, the
//! `HOME` the binary runs with; `records/`, the instance's records directory, where the binary
//! runs; `state/`; and the config file `conductor.yaml` naming them, under the session prefix
//! `ex`. Every fact a case reads comes from a command's output, a view or the records' log file.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{Value, json};

/// The instant every dispatch of a case is sent at.
const AT: &str = "2026-11-12T09:00:00Z";

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
            .join("dispatch_reference")
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

    /// Runs a command the specification refuses with `error`: exit 1, nothing on standard output,
    /// one line on standard error naming the command and then the error.
    fn declined(&self, args: &[&str], error: &str) {
        let output = self.run(args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let head = format!("conductor: {} {}: {error}: ", args[0], args[1]);
        assert!(
            output.status.code() == Some(1)
                && output.stdout.is_empty()
                && stderr.starts_with(&head)
                && stderr.lines().count() == 1,
            "`conductor {}` should be refused as {error}: {}",
            args.join(" "),
            describe(&output)
        );
    }

    /// `dispatch send-dispatch` for `repository` with `brief` at [`AT`], and `extra` flags.
    fn send_args<'a>(repository: &'a str, brief: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            "dispatch",
            "send-dispatch",
            "--repository",
            repository,
            "--brief",
            brief,
            "--sent-at",
            AT,
        ];
        args.extend_from_slice(extra);
        args
    }

    /// Sends a dispatch that must be accepted, and answers the one line it prints.
    fn send(&self, repository: &str, brief: &str, extra: &[&str]) -> String {
        let printed = self.ok(&Self::send_args(repository, brief, extra));
        assert!(
            printed.starts_with("DSP-") && !printed.contains('\n'),
            "send-dispatch prints one dispatch id, got {printed:?}"
        );
        printed
    }

    /// The rows of `dispatch dispatches`, read as `--format jsonl`.
    fn dispatches(&self) -> Vec<Value> {
        self.ok(&["dispatch", "dispatches", "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
            .collect()
    }

    /// The `dispatches` row of `dispatch`.
    fn dispatch(&self, dispatch: &str) -> Value {
        let rows = self.dispatches();
        rows.iter()
            .find(|row| row["dispatch_id"] == dispatch)
            .unwrap_or_else(|| panic!("no dispatches row for {dispatch}: {rows:#?}"))
            .clone()
    }

    /// Every line of the records' dispatch log.
    fn log(&self) -> Vec<Value> {
        lines(&self.records().join("dispatches"))
    }

    /// Runs `args`, which must succeed, append no line to the dispatch log and create no
    /// dispatch, and answers what it printed.
    fn changes_nothing(&self, args: &[&str]) -> String {
        let (log, rows) = (self.log(), self.dispatches());
        let printed = self.ok(args);
        assert_eq!(
            self.log(),
            log,
            "`conductor {}` writes no line",
            args.join(" ")
        );
        assert_eq!(
            self.dispatches(),
            rows,
            "`conductor {}` changes no dispatch",
            args.join(" ")
        );
        printed
    }

    /// Runs `args`, which the specification refuses with `error`, and checks it wrote nothing.
    fn refuses(&self, args: &[&str], error: &str) {
        let (log, rows) = (self.log(), self.dispatches());
        self.declined(args, error);
        assert_eq!(self.log(), log, "a refused command writes no line");
        assert_eq!(
            self.dispatches(),
            rows,
            "a refused command changes no dispatch"
        );
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

/// Every line of every `*.jsonl` file of `dir`, files in name order.
fn lines(dir: &Path) -> Vec<Value> {
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
            fs::read_to_string(path)
                .expect("read a log file")
                .lines()
                .map(|line| serde_json::from_str(line).expect("each log line is JSON"))
                .collect::<Vec<Value>>()
        })
        .collect()
}

/// `owned` as the argument slice [`Case`] takes.
fn args(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}

/// Asserts `value` holds each of `expected`'s keys with its value.
fn holds(value: &Value, expected: &Value) {
    for (key, want) in expected.as_object().expect("an object") {
        assert_eq!(&value[key], want, "{key} of {value}");
    }
}

#[test]
fn a_reference_without_a_goal_sends_a_dispatch_and_the_same_reference_answers_it_again() {
    let case = Case::new("reference");
    let first = case.send(
        "alpha",
        "Fix the login redirect",
        &[
            "--reference",
            "WI-1042",
            "--start-ref",
            "main",
            "--issue",
            "https://forge.example.org/example-org/alpha/-/issues/7",
        ],
    );
    holds(
        &case.dispatch(&first),
        &json!({"state": "Sent", "repository": "alpha", "goal_id": null, "reference": "WI-1042",
                "start_ref": "main",
                "issue": "https://forge.example.org/example-org/alpha/-/issues/7",
                "links": [], "brief": "Fix the login redirect"}),
    );
    let sent = case.log();
    assert_eq!(sent.len(), 1, "one sent line: {sent:#?}");
    holds(
        &sent[0],
        &json!({"event": "sent", "repo": "alpha", "id": first, "to": "ex-alpha",
                "reference": "WI-1042", "start_ref": "main",
                "issue": "https://forge.example.org/example-org/alpha/-/issues/7",
                "brief": "Fix the login redirect"}),
    );
    assert!(sent[0].get("goal").is_none(), "no goal key: {}", sent[0]);

    // The same reference creates nothing and answers the first dispatch, whatever else is sent.
    let again = case.changes_nothing(&Case::send_args(
        "beta",
        "Something else entirely",
        &["--reference", "WI-1042", "--priority", "3"],
    ));
    assert_eq!(again, first, "the held reference answers its dispatch");

    case.ok(&["dispatch", "report-started", "--dispatch-id", &first]);
    case.ok(&[
        "dispatch",
        "report-done",
        "--dispatch-id",
        &first,
        "--evidence",
        "merged",
    ]);
    let after_done = case.changes_nothing(&Case::send_args(
        "alpha",
        "Fix the login redirect",
        &["--reference", "WI-1042"],
    ));
    assert_eq!(
        after_done, first,
        "a done dispatch still holds its reference"
    );
    assert_eq!(case.dispatches().len(), 1, "{:#?}", case.dispatches());
}

#[test]
fn a_reference_for_a_repository_with_an_open_dispatch_is_refused_until_it_ends() {
    let case = Case::new("busy");
    let first = case.send("alpha", "First unit", &["--reference", "WI-1"]);
    let second = Case::send_args("alpha", "Second unit", &["--reference", "WI-2"]);
    case.refuses(&second, "RepositoryBusy");
    case.ok(&["dispatch", "report-started", "--dispatch-id", &first]);
    case.refuses(&second, "RepositoryBusy");
    case.ok(&[
        "dispatch",
        "report-blocked",
        "--dispatch-id",
        &first,
        "--reason",
        "waits on review",
    ]);
    case.refuses(&second, "RepositoryBusy");
    case.ok(&["dispatch", "report-unblocked", "--dispatch-id", &first]);
    case.ok(&[
        "dispatch",
        "report-done",
        "--dispatch-id",
        &first,
        "--evidence",
        "merged",
    ]);
    let admitted = case.send("alpha", "Second unit", &["--reference", "WI-2"]);
    assert_ne!(admitted, first);
    holds(
        &case.dispatch(&admitted),
        &json!({"state": "Sent", "reference": "WI-2"}),
    );

    // An open dispatch with no reference makes the repository busy for a reference as well.
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
    let goal_dispatch = case.send("beta", "Goal work", &["--goal-id", "G1"]);
    holds(
        &case.dispatch(&goal_dispatch),
        &json!({"goal_id": "G1", "reference": null}),
    );
    case.refuses(
        &Case::send_args("beta", "Third unit", &["--reference", "WI-3"]),
        "RepositoryBusy",
    );
    // A dispatch without a reference behaves as before: a busy repository does not refuse it.
    let beside = case.send("alpha", "More goal work", &["--goal-id", "G1"]);
    holds(
        &case.dispatch(&beside),
        &json!({"state": "Sent", "goal_id": "G1"}),
    );
}

#[test]
fn a_dispatch_with_neither_a_goal_nor_a_reference_is_refused() {
    let case = Case::new("neither");
    case.declined(
        &Case::send_args("alpha", "Nothing to serve", &[]),
        "GoalOrReferenceMissing",
    );
    assert!(case.log().is_empty(), "{:#?}", case.log());
}

#[test]
fn a_reference_outside_its_characters_or_length_is_refused() {
    let case = Case::new("bad-reference");
    let long = "a".repeat(65);
    for bad in [
        "",
        long.as_str(),
        "a/b",
        "a?b",
        "a b",
        "-ab",
        ".ab",
        "_ab",
        "ä1",
    ] {
        let flag = format!("--reference={bad}");
        case.declined(
            &Case::send_args("alpha", "Bad reference", &[flag.as_str()]),
            "InvalidReference",
        );
    }
    assert!(case.log().is_empty(), "{:#?}", case.log());
    let longest = "A".repeat(64);
    let held = case.send("alpha", "Longest", &["--reference", &longest]);
    holds(&case.dispatch(&held), &json!({"reference": longest}));
    let other = case.send("beta", "Every character", &["--reference", "9z.Y_x-0"]);
    holds(&case.dispatch(&other), &json!({"reference": "9z.Y_x-0"}));
}

#[test]
fn a_dispatch_sent_by_the_control_plane_records_it_as_its_actor() {
    let case = Case::new("control-plane");
    let sent = case.send(
        "alpha",
        "Work from a control plane",
        &[
            "--reference",
            "WI-7",
            "--actor",
            r#"{"kind":"ControlPlane"}"#,
        ],
    );
    holds(&case.dispatch(&sent), &json!({"actor": "control-plane"}));
}

#[test]
fn record_link_appends_a_link_once_and_refuses_what_is_no_link() {
    let case = Case::new("links");
    let dispatch = case.send("alpha", "Linked work", &["--reference", "WI-9"]);
    let link = "https://forge.example.org/example-org/alpha/-/merge_requests/12";
    let record = |link: &str| -> Vec<String> {
        vec![
            "dispatch".to_owned(),
            "record-link".to_owned(),
            "--dispatch-id".to_owned(),
            dispatch.clone(),
            "--link".to_owned(),
            link.to_owned(),
        ]
    };

    let before = case.log().len();
    case.ok(&args(&record(link)));
    let log = case.log();
    assert_eq!(log.len(), before + 1, "one linked line: {log:#?}");
    holds(
        log.last().expect("a line"),
        &json!({"event": "linked", "repo": "alpha", "id": dispatch, "link": link}),
    );
    holds(&case.dispatch(&dispatch), &json!({"links": [link]}));

    case.changes_nothing(&args(&record(link)));
    holds(&case.dispatch(&dispatch), &json!({"links": [link]}));

    let too_long = format!("https://forge.example.org/{}", "a".repeat(2049 - 26));
    assert_eq!(too_long.chars().count(), 2049);
    for bad in [
        "ftp://forge.example.org/x",
        "not a link",
        "https://",
        "http://",
        "https://forge.example.org/a b",
        too_long.as_str(),
    ] {
        case.refuses(&args(&record(bad)), "InvalidLink");
    }
    case.refuses(
        &[
            "dispatch",
            "record-link",
            "--dispatch-id",
            "DSP-20991231-01",
            "--link",
            link,
        ],
        "DispatchNotFound",
    );

    // A link may be recorded in any state: here the dispatch is done.
    case.ok(&[
        "dispatch",
        "report-done",
        "--dispatch-id",
        &dispatch,
        "--evidence",
        "merged",
    ]);
    let mut held = vec![link.to_owned()];
    for number in 2..=32 {
        let next = format!("http://forge.example.org/example-org/alpha/pull/{number}");
        case.ok(&args(&record(&next)));
        held.push(next);
    }
    holds(&case.dispatch(&dispatch), &json!({"links": held}));
    case.refuses(
        &args(&record(
            "https://forge.example.org/example-org/alpha/pull/33",
        )),
        "DispatchLinksFull",
    );
    // A link the full dispatch holds is still a no-op, not a refusal.
    case.changes_nothing(&args(&record(link)));
}

#[test]
fn records_check_accepts_every_line_the_new_commands_write_and_names_an_unknown_key() {
    let case = Case::new("records-check");
    let dispatch = case.send(
        "alpha",
        "Checked work",
        &[
            "--reference",
            "WI-11",
            "--start-ref",
            "release/1.4",
            "--issue",
            "#11",
        ],
    );
    case.ok(&[
        "dispatch",
        "record-link",
        "--dispatch-id",
        &dispatch,
        "--link",
        "https://forge.example.org/example-org/alpha/pull/11",
    ]);
    let events: Vec<Value> = case
        .log()
        .iter()
        .map(|line| line["event"].clone())
        .collect();
    assert_eq!(events, [json!("sent"), json!("linked")]);
    let checked = case.run(&["records", "check"]);
    assert!(
        checked.status.success(),
        "records check accepts every line: {}",
        describe(&checked)
    );

    let month = fs::read_dir(case.records().join("dispatches"))
        .expect("the dispatch log")
        .map(|entry| entry.expect("an entry").path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .expect("a month file");
    let mut text = fs::read_to_string(&month).expect("read the month file");
    text.push_str(
        &json!({"at": AT, "event": "linked", "repo": "alpha", "id": dispatch,
                "links": ["https://forge.example.org/x"]})
        .to_string(),
    );
    text.push('\n');
    fs::write(&month, text).expect("append a line with an unknown key");
    let output = case.run(&["records", "check"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.code() == Some(1)
            && stdout.contains("UnknownKeySet")
            && stdout.contains("links"),
        "records check names the unknown key: {}",
        describe(&output)
    );
}
