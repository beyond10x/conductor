//! `story:mailbox-send-and-deliver`, acceptance 2 (docs/design/conductor-run.md § 3): `conductor
//! message send --to conductor --from <session> --text <file|->` parses the message's first line
//! with the § 6 grammar. A line that parses is left whole as `<state>/mailbox/conductor/<ulid>.md`,
//! recorded `Received` with transport `Mailbox` and that file, relative to the state directory, as
//! its `body_path`, and appended to the records' dispatch log as one `message-received` line with
//! the same `body_path`. A line that does not parse prints the format, exits 1 and writes nothing.
//!
//! Each case is a [`Scratch`] directory: `home/`, the `HOME` the binary runs with; `records/`,
//! where it runs; `state/`; and the config file `conductor.yaml` naming them, under the session
//! prefix `ex`.

mod common;
mod scratch;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use scratch::Scratch;
use serde_json::Value;

/// The controller's session that sends every message.
const FROM: &str = "ex-cortex";

/// Crockford's base 32, the alphabet of a ULID.
const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

struct Case {
    scratch: Scratch,
    dir: PathBuf,
}

/// Where `--text` reads the message from.
enum Text {
    File,
    Stdin,
}

impl Case {
    fn new(name: &str) -> Self {
        let scratch = Scratch::new(Path::new("mailbox").join(name));
        for sub in ["home", "records", "state"] {
            fs::create_dir_all(scratch.path().join(sub)).expect("create a case directory");
        }
        let dir = scratch.path().canonicalize().expect("the case's directory");
        fs::write(
            dir.join("conductor.yaml"),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n",
                root = dir.join("home/example-org").display(),
                trees = dir.join("trees").display(),
                records = dir.join("records").display(),
                state = dir.join("state").display(),
            ),
        )
        .expect("write the config file");
        Self { scratch, dir }
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn mailbox(&self) -> PathBuf {
        self.state().join("mailbox/conductor")
    }

    /// `conductor <args>` from the records directory, with the case's home and config file, and
    /// `stdin` on standard input.
    fn run(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = common::conductor(self.dir.join("home"))
            .args(args)
            .current_dir(self.dir.join("records"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("conductor runs");
        child
            .stdin
            .take()
            .expect("its standard input")
            .write_all(stdin.as_bytes())
            .expect("write its standard input");
        child.wait_with_output().expect("conductor ends")
    }

    /// `conductor message send --to conductor --from <FROM> --text …` with `text` in a file the
    /// case writes, or on standard input.
    fn send(&self, text: &str, how: &Text) -> Output {
        match how {
            Text::File => {
                let file = self.scratch.path().join("message.md");
                fs::write(&file, text).expect("write the message");
                let file = file.display().to_string();
                self.run(
                    &[
                        "message",
                        "send",
                        "--to",
                        "conductor",
                        "--from",
                        FROM,
                        "--text",
                        &file,
                    ],
                    "",
                )
            }
            Text::Stdin => self.run(
                &[
                    "message",
                    "send",
                    "--to",
                    "conductor",
                    "--from",
                    FROM,
                    "--text",
                    "-",
                ],
                text,
            ),
        }
    }

    /// The files of the conductor's mailbox; none when the directory is not there.
    fn mailbox_files(&self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(self.mailbox()) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .map(|entry| entry.expect("read the mailbox").path())
            .collect();
        files.sort();
        files
    }

    /// The rows of `conductor message messages --format json`.
    fn messages(&self) -> Vec<Value> {
        let output = self.run(&["message", "messages", "--format", "json"], "");
        assert!(output.status.success(), "{}", describe(&output));
        let rows: Value = serde_json::from_slice(&output.stdout).expect("the view is JSON");
        rows.as_array().expect("an array of rows").clone()
    }

    /// Every line of the records' dispatch log, in file and line order.
    fn dispatch_lines(&self) -> Vec<Value> {
        let Ok(entries) = fs::read_dir(self.dir.join("records/dispatches")) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .map(|entry| entry.expect("read the log").path())
            .collect();
        files.sort();
        files
            .iter()
            .flat_map(|file| {
                fs::read_to_string(file)
                    .expect("read a log file")
                    .lines()
                    .map(|line| serde_json::from_str(line).expect("a line is JSON"))
                    .collect::<Vec<Value>>()
            })
            .collect()
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Sends `text`, whose first line is `first_line`, of `kind`, naming `repo`, and checks the
/// mailbox file, the message the store keeps and the line the dispatch log holds.
fn sends(name: &str, text: &str, how: &Text, kind: &str, repo: &str) {
    let case = Case::new(name);
    let first_line = text.lines().next().expect("a first line");
    let output = case.send(text, how);
    assert!(output.status.success(), "{}", describe(&output));
    let id = String::from_utf8(output.stdout.clone())
        .expect("stdout is UTF-8")
        .trim()
        .to_owned();
    assert_eq!(
        id.len(),
        36,
        "it prints the message's id: {}",
        describe(&output)
    );

    let files = case.mailbox_files();
    assert_eq!(files.len(), 1, "one file in the mailbox: {files:?}");
    let file = &files[0];
    let file_name = file.file_name().expect("a name").to_string_lossy();
    let ulid = file_name
        .strip_suffix(".md")
        .unwrap_or_else(|| panic!("the file is <ulid>.md: {file_name}"));
    assert!(
        ulid.len() == 26 && ulid.chars().all(|c| CROCKFORD.contains(c)) && ulid < "8",
        "the file is named by a ULID: {file_name}"
    );
    assert_eq!(
        fs::read_to_string(file).expect("read the mailbox file"),
        text,
        "the file holds the whole text"
    );
    let body_path = format!("mailbox/conductor/{file_name}");

    let rows = case.messages();
    assert_eq!(rows.len(), 1, "one message: {rows:#?}");
    let row = &rows[0];
    for (key, value) in [
        ("message_id", id.as_str()),
        ("state", "Received"),
        ("sender", FROM),
        ("recipient", "conductor"),
        ("kind", kind),
        ("first_line", first_line),
        ("transport", "Mailbox"),
        ("body_path", body_path.as_str()),
    ] {
        assert_eq!(row[key], value, "the message's {key}: {row}");
    }

    let lines = case.dispatch_lines();
    assert_eq!(lines.len(), 1, "one dispatch line: {lines:#?}");
    let line = &lines[0];
    for (key, value) in [
        ("event", "message-received"),
        ("repo", repo),
        ("id", id.as_str()),
        ("from", FROM),
        ("to", "conductor"),
        ("kind", kind),
        ("first_line", first_line),
        ("body_path", body_path.as_str()),
    ] {
        assert_eq!(line[key], value, "the line's {key}: {line}");
    }
}

#[test]
fn a_report_is_left_in_the_mailbox_and_recorded() {
    sends(
        "report",
        "[REPORT cortex DSP-20991231-01] started on the brief\n\nWave 1 runs in tree t1.\n",
        &Text::File,
        "Report",
        "cortex",
    );
}

/// A `review <url>` report, the one a controller under `merge: human` sends for its pull request.
#[test]
fn a_review_report_is_left_in_the_mailbox_and_recorded() {
    sends(
        "review",
        "[REPORT cortex DSP-20991231-01] review https://forge.example.org/example-org/cortex/pull/12\n",
        &Text::Stdin,
        "Report",
        "cortex",
    );
}

#[test]
fn a_need_is_left_in_the_mailbox_and_recorded() {
    sends(
        "need",
        "[NEED cortex N-20991231-01] atlas a release with the new parser\n",
        &Text::File,
        "Need",
        "cortex",
    );
}

#[test]
fn a_decision_request_is_left_in_the_mailbox_and_recorded() {
    sends(
        "decision-request",
        "[DECISION-REQUEST cortex R-7] merge the parser now or after the release?\n\
         A: now\nB: after\n",
        &Text::Stdin,
        "DecisionRequest",
        "cortex",
    );
}

#[test]
fn a_resource_request_is_left_in_the_mailbox_and_recorded() {
    sends(
        "resource",
        "[RESOURCE cortex] disk 20G",
        &Text::File,
        "Resource",
        "cortex",
    );
}

#[test]
fn a_handover_is_left_in_the_mailbox_and_recorded() {
    sends(
        "handover",
        "[HANDOVER cortex] context shutdown\nThe hand-over is in .git/handoff/1.md.\n",
        &Text::Stdin,
        "Handover",
        "cortex",
    );
}

/// An unparsed first line prints the format on standard output, names the refusal on one line of
/// standard error, exits 1, and leaves no file, no message and no line.
#[test]
fn a_first_line_that_does_not_parse_is_answered_with_the_format_and_writes_nothing() {
    let case = Case::new("unparsed");
    let output = case.send("hello conductor\n[REPORT cortex -] started\n", &Text::File);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        conductor_cli::message::FORMAT,
        "the format is on standard output"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("message send") && stderr.contains("Unparsed"),
        "the refusal names the command and Unparsed: {stderr}"
    );
    assert!(
        case.mailbox_files().is_empty(),
        "no file: {:?}",
        case.mailbox_files()
    );
    let state: Vec<_> = fs::read_dir(case.state())
        .expect("read the state directory")
        .collect();
    assert!(
        state.is_empty(),
        "nothing under the state directory: {state:?}"
    );
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
}

/// The recipient names a directory under `mailbox/`: one that would leave it is refused before
/// anything is written.
#[test]
fn a_recipient_that_leaves_the_mailbox_is_refused() {
    let case = Case::new("recipient");
    for to in ["../conductor", "a/b", "", ".."] {
        let output = case.run(
            &["message", "send", "--to", to, "--from", FROM, "--text", "-"],
            "[REPORT cortex -] started\n",
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "--to {to:?}: {}",
            describe(&output)
        );
        let state: Vec<_> = fs::read_dir(case.state())
            .expect("read the state directory")
            .collect();
        assert!(state.is_empty(), "--to {to:?}: nothing written: {state:?}");
    }
}

/// A report whose first line is as long as the bound allows: 300 characters.
fn report_of(length: usize) -> String {
    let head = "[REPORT cortex DSP-20991231-01] progress ";
    format!("{head}{}", "x".repeat(length - head.len()))
}

/// `story:conductor-context-budget`: a controller's REPORT first line is at most 300 characters.
/// One of 300 is left in the mailbox and recorded.
#[test]
fn a_report_whose_first_line_is_300_characters_is_left_in_the_mailbox() {
    let text = format!("{}\nThe detail is in the body.\n", report_of(300));
    assert_eq!(
        text.lines().next().map(|line| line.chars().count()),
        Some(300)
    );
    sends("report-300", &text, &Text::Stdin, "Report", "cortex");
}

/// One of 301 is answered with the format, which names the bound, and writes nothing: no file,
/// no message and no line, so nothing of it is typed into a session or kept in a log.
#[test]
fn a_report_whose_first_line_is_over_300_characters_is_refused() {
    let case = Case::new("report-301");
    let output = case.send(
        &format!("{}\nThe detail is in the body.\n", report_of(301)),
        &Text::Stdin,
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        conductor_cli::message::FORMAT,
        "the format is on standard output"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("Unparsed") && stderr.contains("301 characters, over the 300"),
        "the refusal names the length and the bound: {stderr}"
    );
    assert!(
        stderr.len() < 400,
        "the refusal does not repeat the line: {stderr}"
    );
    assert!(case.mailbox_files().is_empty(), "no file");
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
}

/// The format names the bound of a report's first line.
#[test]
fn the_format_names_the_bound_of_a_report_s_first_line() {
    assert!(
        conductor_cli::message::FORMAT
            .contains("A [REPORT …] first line is at most 300 characters"),
        "{}",
        conductor_cli::message::FORMAT
    );
}

/// `story:conductor-context-budget`: a received message is logged by the command that received
/// it, never re-typed by a session: `message send` answers one line, the message's id, and not
/// the body.
#[test]
fn a_sent_message_is_answered_with_its_id_alone() {
    let case = Case::new("answer-one-line");
    let output = case.send(
        "[REPORT cortex -] progress wave 2 merged\n\nA body of several lines.\nSecond line.\n",
        &Text::Stdin,
    );
    assert!(output.status.success(), "{}", describe(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 1, "one line: {stdout}");
    assert_eq!(stdout.trim().len(), 36, "the message's id: {stdout}");
    let lines = case.dispatch_lines();
    assert_eq!(lines.len(), 1, "the command logged it: {lines:?}");
    assert_eq!(
        lines[0]["first_line"], "[REPORT cortex -] progress wave 2 merged",
        "{lines:?}"
    );
}

/// The format lists the hand-over form `message::read` parses.
#[test]
fn the_format_names_the_handover_form() {
    assert!(
        conductor_cli::message::FORMAT.contains("[HANDOVER <repo>] context <n>k"),
        "{}",
        conductor_cli::message::FORMAT
    );
}

/// Adversary, wave 49 U8. The format bounds a `[REPORT …]` first line only. A first line of
/// another kind that breaks its form (a `[NEED …]` without its request id) and happens to be
/// over 300 characters is refused for its form: the refusal must not tell the sender that a
/// first line holds 300 characters, which is true of no kind but a report, and a need of that
/// length with its request id is accepted
/// (`a_first_line_of_another_kind_is_not_bounded_and_characters_are_counted`).
#[test]
fn a_long_first_line_of_another_kind_that_breaks_its_form_is_not_told_the_report_bound() {
    let case = Case::new("need-long-unparsed");
    let first = format!("[NEED cortex] atlas {}", "x".repeat(330));
    let output = case.send(&format!("{first}\n"), &Text::Stdin);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unparsed"), "{stderr}");
    assert!(
        !stderr.contains("over the 300"),
        "a need has no 300-character bound; the line breaks its form: {stderr}"
    );
}

/// Adversary, wave 49 U8. The bound is in characters, not bytes, and holds a report only:
/// `message::read`, which `message send` and `message receive-message` both read the first line
/// with.
#[test]
fn a_first_line_of_another_kind_is_not_bounded_and_characters_are_counted() {
    use conductor_cli::message::read;
    let head = "[REPORT cortex DSP-20991231-01] progress ";
    let umlauts = |length: usize| format!("{head}{}", "ä".repeat(length - head.chars().count()));
    assert!(umlauts(300).len() > 300, "over 300 bytes");
    assert!(read(&umlauts(300)).parsed, "300 characters of a report");
    assert!(!read(&umlauts(301)).parsed, "301 characters of a report");
    assert!(read(&report_of(300)).parsed, "300");
    assert!(!read(&report_of(301)).parsed, "301");
    let need = format!("[NEED cortex N-20991231-01] atlas {}", "x".repeat(400));
    assert!(read(&need).parsed, "a need is not bounded");
    let request = format!("[DECISION-REQUEST cortex R-7] {}?", "x".repeat(400));
    assert!(read(&request).parsed, "a decision request is not bounded");
    sends(
        "need-long",
        &format!("{need}\n"),
        &Text::Stdin,
        "Need",
        "cortex",
    );
}

/// Adversary, wave 49 U8. The bound exists because the first line is logged, listed and typed
/// into the session it reaches (`REPORT_FIRST_LINE`). `read` counts the trimmed line and `send`
/// logs the line as written, so the first line the log holds is at most 300 characters too.
#[test]
fn the_first_line_the_log_holds_of_a_report_is_at_most_300_characters() {
    let case = Case::new("report-padded");
    let first = format!(
        "[REPORT cortex -] progress wave 2 merged{}",
        " ".repeat(400)
    );
    let output = case.send(&format!("{first}\nThe body.\n"), &Text::Stdin);
    let lines = case.dispatch_lines();
    let logged: Vec<usize> = lines
        .iter()
        .filter_map(|line| line["first_line"].as_str())
        .map(|line| line.chars().count())
        .collect();
    assert!(
        logged.iter().all(|length| *length <= 300),
        "a report was accepted and logged with a first line of {logged:?} characters: {}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// The loop's mailbox pass (story:need-starts-unit-of-work)
// ---------------------------------------------------------------------------------------------

/// The variable that holds a child run's config file: the child runs one watch tick of the loop
/// over no watch line, which reads the mailbox ([`child`]).
const CHILD: &str = "CONDUCTOR_TEST_MAILBOX_CHILD";

/// The deliverer of a child run: no controller runs in these cases, so nothing is delivered.
struct Nobody;

impl conductor_cli::session::Deliverer for Nobody {
    fn deliver(&self, name: &str, _: &str) -> anyhow::Result<conductor_cli::session::Delivery> {
        anyhow::bail!("no session {name} runs in this case")
    }
}

/// The child run of this test binary ([`Case::tick`]); a plain run of the suite passes at once.
/// It fixes the case's config file as the process's instance and runs one watch tick, which reads
/// conductor's mailbox, then exits 0, or names the error and exits 1.
#[test]
fn child() {
    let Some(file) = std::env::var_os(CHILD) else {
        return;
    };
    let file = PathBuf::from(file);
    conductor_cli::config::set_active(Some(&file)).expect("the case's config loads");
    let host = conductor_cli::confine::Host;
    let seams = conductor_cli::run::Seams {
        providers: &host,
        deliverer: &Nobody,
        handle_request: None,
        program: PathBuf::from(std::env::var_os("CONDUCTOR_TEST_PROGRAM").expect("the binary")),
        gc: conductor_cli::gc::gc,
    };
    match conductor_cli::run::handle_lines(None, None, &[], &seams) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

impl Case {
    /// A checkout of `repo` under the checkouts root, which makes it a repository the instance
    /// knows.
    fn checkout(&self, repo: &str) {
        let git = self.dir.join("home/example-org").join(repo).join(".git");
        fs::create_dir_all(&git).expect("create a checkout");
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("write its HEAD");
    }

    /// One watch tick of the loop in a child of this test binary ([`child`]).
    fn tick(&self) {
        let program = common::conductor(self.dir.join("home"))
            .get_program()
            .to_owned();
        let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
            .args([
                "--exact",
                "child",
                "--nocapture",
                "--test-threads",
                "1",
                "-q",
            ])
            .env("HOME", self.dir.join("home"))
            .env_remove("CONDUCTOR_INSTANCE")
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env(CHILD, self.dir.join("conductor.yaml"))
            .env("CONDUCTOR_TEST_PROGRAM", program)
            .current_dir(self.dir.join("records"))
            .stdin(Stdio::null())
            .output()
            .expect("the child runs");
        assert!(output.status.success(), "the tick: {}", describe(&output));
    }

    /// The rows of `conductor dispatch dispatches --format json`; none before a store exists.
    fn dispatches(&self) -> Vec<Value> {
        let output = self.run(&["dispatch", "dispatches", "--format", "json"], "");
        if !output.status.success() {
            return Vec::new();
        }
        let rows: Value = serde_json::from_slice(&output.stdout).expect("the view is JSON");
        rows.as_array().expect("an array of rows").clone()
    }

    /// What the loop keeps for a controller's next start, `<state>/run/pending.json`.
    fn kept(&self) -> Vec<Value> {
        fs::read_to_string(self.state().join("run/pending.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|kept| kept.as_array().cloned())
            .unwrap_or_default()
    }

    /// Sends `text` from [`FROM`], which must be taken; the message's id.
    fn sent(&self, text: &str) -> String {
        let output = self.send(text, &Text::Stdin);
        assert!(output.status.success(), "{}", describe(&output));
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}

/// `story:need-starts-unit-of-work`: a `[NEED]` for a repository the instance knows that has no
/// charter, no running controller and no open dispatch becomes one unit of work for it: a
/// dispatch with the reference `need-<repo>-<request-id>` and the need's whole text as its brief.
/// The message reads `Handled` with that dispatch, nothing is kept for the other repository, and
/// the requester is told (kept for its next start, as no controller of it runs). Sent again, the
/// need opens nothing more: its message is routed to the same dispatch.
#[test]
fn a_need_to_a_chartless_repository_opens_one_unit_of_work() {
    let case = Case::new("need-unit");
    case.checkout("cortex");
    case.checkout("atlas");
    let need = "[NEED cortex N-20991231-01] atlas a release with the new parser\n\nIt reads it.\n";
    let first = case.sent(need);
    case.tick();

    let dispatches = case.dispatches();
    assert_eq!(dispatches.len(), 1, "one dispatch: {dispatches:#?}");
    let dispatch = &dispatches[0];
    assert_eq!(dispatch["repository"], "atlas", "{dispatch}");
    assert_eq!(
        dispatch["reference"], "need-cortex-N-20991231-01",
        "{dispatch}"
    );
    assert_eq!(dispatch["brief"], need, "{dispatch}");
    let id = dispatch["dispatch_id"].as_str().expect("its id").to_owned();

    let message = |message_id: &str| {
        case.messages()
            .into_iter()
            .find(|row| row["message_id"] == message_id)
            .expect("the message")
    };
    let row = message(&first);
    assert_eq!(row["state"], "Handled", "{row}");
    assert_eq!(row["dispatch_id"], id.as_str(), "{row}");

    let kept = case.kept();
    assert!(
        kept.iter().all(|item| item["repo"] != "atlas"),
        "the need is not kept for atlas: {kept:#?}"
    );
    let told = format!("[REPORT cortex -] need N-20991231-01 opened {id} for atlas");
    assert!(
        kept.iter().any(|item| item["repo"] == "cortex"
            && item["text"]
                .as_str()
                .is_some_and(|text| text.starts_with(&told))),
        "the requester is told {told:?}: {kept:#?}"
    );

    let again = case.sent(need);
    case.tick();
    assert_eq!(case.dispatches().len(), 1, "sent twice, one dispatch");
    let row = message(&again);
    assert_eq!(row["state"], "Handled", "{row}");
    assert_eq!(row["dispatch_id"], id.as_str(), "{row}");
}

/// `story:need-starts-unit-of-work`: a `[NEED]` for a repository with a charter, and one whose
/// `<other-repo>` names the requester itself, keep today's behaviour: kept for the repository's
/// next start, no dispatch opened.
#[test]
fn a_need_to_a_chartered_repository_or_to_its_sender_opens_nothing() {
    let case = Case::new("need-chartered");
    case.checkout("cortex");
    case.checkout("atlas");
    let charter = case.dir.join("records/charters/atlas.md");
    fs::create_dir_all(charter.parent().expect("a parent")).expect("create the charters");
    fs::write(&charter, "# Charter: atlas\n\nKeep main green.\n").expect("write the charter");
    case.sent("[NEED cortex N-20991231-01] atlas a release with the new parser\n");
    case.sent("[NEED cortex N-20991231-02] cortex a release of itself\n");
    case.tick();
    assert_eq!(case.dispatches(), Vec::<Value>::new(), "no dispatch");
    let kept: Vec<String> = case
        .kept()
        .iter()
        .filter_map(|item| item["repo"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(kept, ["atlas", "cortex"], "each need is kept as before");
}

/// `story:need-starts-unit-of-work`: a `[NEED]` whose `<other-repo>` names no repository of the
/// instance raises one alert, `need-unroutable`, its subject `<repo> <request-id>`; the next tick
/// raises none again.
#[test]
fn a_need_naming_no_repository_raises_one_alert() {
    let case = Case::new("need-unroutable");
    case.checkout("cortex");
    case.sent("[NEED cortex N-20991231-01] delta export the store id\n");
    case.tick();
    case.tick();
    let alerts: Vec<Value> = case
        .dispatch_lines()
        .into_iter()
        .filter(|line| line["event"] == "alert")
        .collect();
    assert_eq!(alerts.len(), 1, "one alert line: {alerts:#?}");
    assert_eq!(alerts[0]["condition"], "need-unroutable", "{}", alerts[0]);
    assert_eq!(alerts[0]["repo"], "cortex", "{}", alerts[0]);
    assert_eq!(alerts[0]["id"], "cortex N-20991231-01", "{}", alerts[0]);
    assert_eq!(case.dispatches(), Vec::<Value>::new(), "no dispatch");
}
