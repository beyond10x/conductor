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
        "[REPORT cortex DSP-20261017-01] started on the brief\n\nWave 1 runs in tree t1.\n",
        &Text::File,
        "Report",
        "cortex",
    );
}

#[test]
fn a_need_is_left_in_the_mailbox_and_recorded() {
    sends(
        "need",
        "[NEED cortex N-20261017-01] atlas a release with the new parser\n",
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

/// The format lists the hand-over form `message::read` parses.
#[test]
fn the_format_names_the_handover_form() {
    assert!(
        conductor_cli::message::FORMAT.contains("[HANDOVER <repo>] context <n>k"),
        "{}",
        conductor_cli::message::FORMAT
    );
}
