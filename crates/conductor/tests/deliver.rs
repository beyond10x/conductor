//! `story:mailbox-send-and-deliver`, acceptance 4 (docs/design/conductor-run.md § 3): `conductor
//! session deliver --name <session> --text <file|->` types the text into the session's tmux
//! server on `<state>/sessions/<name>.tmux` through the launcher's `type_text` (one bracketed
//! paste, then Enter pasted unbracketed), records the delivery (`MessageDelivered`) and appends
//! one `delivered` line to the records' dispatch log. Text read from standard input is kept as
//! `<state>/mailbox/delivered/<ulid>.md`, so every delivery names its body. A session with no
//! socket is refused with `SessionNotReachable`, exit 1, nothing typed or recorded; a tmux that
//! does not answer is refused the same way after one try. `conductor session deliveries` lists
//! what was delivered.
//!
//! Each case is a [`Scratch`] directory: `home/`, the `HOME` the binary runs with, whose `bin/`
//! holds a fake `tmux` this file writes, first on `PATH`, that records each call's working
//! directory and arguments and touches no real server; `records/`, where the binary runs;
//! `state/`; and the config file `conductor.yaml` naming them, under the session prefix `ex`.

mod common;
mod scratch;

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use scratch::Scratch;
use serde_json::Value;

/// The controller's session every case delivers to.
const SESSION: &str = "ex-cortex";

/// Crockford's base 32, the alphabet of a ULID.
const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Ends each argument of a recorded call.
const ARG: char = '\u{1f}';

/// Ends each recorded call.
const CALL: char = '\u{1e}';

struct Case {
    scratch: Scratch,
    dir: PathBuf,
}

impl Case {
    /// A case whose fake `tmux` exits `status`, printing `stderr` when it fails.
    fn new(name: &str, status: u8) -> Self {
        let scratch = Scratch::new(Path::new("deliver").join(name));
        for sub in ["home/bin", "records", "state/sessions"] {
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
        let fake = dir.join("home/bin/tmux");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\n\
                 {{ printf '%s\\037' \"$(pwd)\"; for a in \"$@\"; do printf '%s\\037' \"$a\"; \
                 done; printf '\\036'; }} >> '{calls}'\n\
                 [ {status} -eq 0 ] || echo 'no server running on {SESSION}.tmux' >&2\n\
                 exit {status}\n",
                calls = dir.join("tmux.calls").display(),
            ),
        )
        .expect("write the fake tmux");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        Self { scratch, dir }
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    /// Plants the session's launcher socket, as the provider's unit leaves it while it runs.
    fn socket(&self) {
        fs::write(self.state().join(format!("sessions/{SESSION}.tmux")), "")
            .expect("plant the socket");
    }

    /// `conductor <args>` from the records directory, with the case's home, config file and fake
    /// `tmux` first on `PATH`, and `stdin` on standard input.
    fn run(&self, args: &[&str], stdin: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.join("home/bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut child = common::conductor(self.dir.join("home"))
            .args(args)
            .current_dir(self.dir.join("records"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env("PATH", path)
            .env_remove("TMUX")
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

    /// `conductor session deliver --name <SESSION> --text <file>`, the file holding `text`.
    fn deliver_file(&self, text: &str) -> (Output, PathBuf) {
        let file = self.scratch.path().join("text.md");
        fs::write(&file, text).expect("write the text");
        let file = file.canonicalize().expect("the text's file");
        let shown = file.display().to_string();
        let output = self.run(
            &["session", "deliver", "--name", SESSION, "--text", &shown],
            "",
        );
        (output, file)
    }

    /// `conductor session deliver --name <SESSION> --text -`, `text` on standard input.
    fn deliver_stdin(&self, text: &str) -> Output {
        self.run(
            &["session", "deliver", "--name", SESSION, "--text", "-"],
            text,
        )
    }

    /// The calls the fake `tmux` recorded: each its working directory, then its arguments.
    fn calls(&self) -> Vec<Vec<String>> {
        let Ok(text) = fs::read_to_string(self.dir.join("tmux.calls")) else {
            return Vec::new();
        };
        text.split(CALL)
            .filter(|call| !call.is_empty())
            .map(|call| {
                call.strip_suffix(ARG)
                    .unwrap_or(call)
                    .split(ARG)
                    .map(str::to_owned)
                    .collect()
            })
            .collect()
    }

    /// The rows of `conductor session deliveries --format json`.
    fn deliveries(&self) -> Vec<Value> {
        let output = self.run(&["session", "deliveries", "--format", "json"], "");
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

/// The four calls `type_text` makes on the session's socket from `<state>/sessions`: the text
/// as one bracketed paste, then a carriage return pasted unbracketed.
fn typed(case: &Case, text: &str) -> Vec<Vec<String>> {
    let sessions = case.state().join("sessions").display().to_string();
    let socket = format!("{SESSION}.tmux");
    let pane = format!("={SESSION}:");
    let call = |args: &[&str]| -> Vec<String> {
        [sessions.as_str(), "-S", socket.as_str()]
            .iter()
            .chain(args)
            .map(|word| (*word).to_owned())
            .collect()
    };
    vec![
        call(&["set-buffer", "-b", "conductor", "--", text]),
        call(&["paste-buffer", "-d", "-p", "-b", "conductor", "-t", &pane]),
        call(&["set-buffer", "-b", "conductor", "--", "\r"]),
        call(&["paste-buffer", "-d", "-b", "conductor", "-t", &pane]),
    ]
}

/// Checks the one delivery and the one `delivered` line a case recorded.
fn recorded(case: &Case, first_line: &str, body_path: &str, repo: &str) {
    let rows = case.deliveries();
    assert_eq!(rows.len(), 1, "one delivery: {rows:#?}");
    let row = &rows[0];
    for (key, value) in [
        ("state", "Delivered"),
        ("session_name", SESSION),
        ("first_line", first_line),
        ("body_path", body_path),
    ] {
        assert_eq!(row[key], value, "the delivery's {key}: {row}");
    }
    let delivered_at = row["delivered_at"].as_str().expect("a time");
    assert!(
        delivered_at.len() >= 20 && delivered_at.ends_with('Z'),
        "an RFC 3339 instant in UTC: {row}"
    );
    let lines = case.dispatch_lines();
    assert_eq!(lines.len(), 1, "one dispatch line: {lines:#?}");
    let line = &lines[0];
    for (key, value) in [
        ("event", "delivered"),
        ("repo", repo),
        ("to", SESSION),
        ("first_line", first_line),
        ("body_path", body_path),
    ] {
        assert_eq!(line[key], value, "the line's {key}: {line}");
    }
    let mut keys: Vec<&str> = line
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["at", "body_path", "event", "first_line", "repo", "to"],
        "the keys of a delivered line: {line}"
    );
}

#[test]
fn a_file_is_typed_into_the_session_and_recorded() {
    let case = Case::new("file", 0);
    case.socket();
    let text = "[HANDOVER cortex] context 210k\nWrite the hand-over file, then stop.\n";
    let (output, file) = case.deliver_file(text);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(
        case.calls(),
        typed(&case, text.trim_end_matches('\n')),
        "the text is typed once, then Enter"
    );
    recorded(
        &case,
        "[HANDOVER cortex] context 210k",
        &file.display().to_string(),
        "cortex",
    );
}

#[test]
fn standard_input_is_kept_in_the_mailbox_typed_and_recorded() {
    let case = Case::new("stdin", 0);
    case.socket();
    let text = "[DECISION R-7] A merge now (DEC-20261017-01)\nThe release waits.\n";
    let output = case.deliver_stdin(text);
    assert!(output.status.success(), "{}", describe(&output));
    let kept = case.state().join("mailbox/delivered");
    let files: Vec<PathBuf> = fs::read_dir(&kept)
        .expect("the delivered mailbox is there")
        .map(|entry| entry.expect("read it").path())
        .collect();
    assert_eq!(files.len(), 1, "one file: {files:?}");
    let name = files[0]
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    let ulid = name.strip_suffix(".md").expect("<ulid>.md");
    assert!(
        ulid.len() == 26 && ulid.chars().all(|c| CROCKFORD.contains(c)),
        "named by a ULID: {name}"
    );
    assert_eq!(
        fs::read_to_string(&files[0]).expect("read it"),
        text,
        "the file holds the text"
    );
    assert_eq!(case.calls(), typed(&case, text.trim_end_matches('\n')));
    // `[DECISION …]` names no repository, so the line's repo is the session.
    recorded(
        &case,
        "[DECISION R-7] A merge now (DEC-20261017-01)",
        &format!("mailbox/delivered/{name}"),
        SESSION,
    );
}

/// No socket: nothing typed, nothing kept, nothing recorded, one line naming the session.
#[test]
fn a_session_without_a_socket_is_not_reachable() {
    let case = Case::new("no-socket", 0);
    let output = case.deliver_stdin("[HANDOVER cortex] context 210k\n");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("SessionNotReachable") && stderr.contains(SESSION),
        "the refusal names the error and the session: {stderr}"
    );
    assert!(case.calls().is_empty(), "tmux is never called");
    assert!(
        !case.state().join("mailbox").exists(),
        "no body is kept for a session that is not reached"
    );
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
    assert!(
        !case.state().join("tree").exists(),
        "no store is created for a delivery that was not made"
    );
}

/// A socket whose tmux does not answer: refused after one call, never retried.
#[test]
fn a_tmux_that_does_not_answer_is_tried_once_and_refused() {
    let case = Case::new("no-server", 1);
    case.socket();
    let (output, _) = case.deliver_file("[REPORT cortex -] watch: CI red on main\n");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("SessionNotReachable") && stderr.contains(SESSION),
        "the refusal names the error and the session: {stderr}"
    );
    assert_eq!(
        case.calls().len(),
        1,
        "one call, no retry: {:?}",
        case.calls()
    );
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
}

/// `session deliveries` lists every delivery, in the order they were made, and renders text.
#[test]
fn the_deliveries_view_lists_each_delivery() {
    let case = Case::new("view", 0);
    case.socket();
    for text in [
        "[HANDOVER cortex] context 210k\n",
        "[REPORT cortex -] check-in: report your state or blocked\n",
    ] {
        let output = case.deliver_stdin(text);
        assert!(output.status.success(), "{}", describe(&output));
    }
    let rows = case.deliveries();
    let first: Vec<&str> = rows
        .iter()
        .map(|row| row["first_line"].as_str().expect("a first line"))
        .collect();
    assert_eq!(
        first,
        [
            "[HANDOVER cortex] context 210k",
            "[REPORT cortex -] check-in: report your state or blocked"
        ]
    );
    let text = case.run(&["session", "deliveries"], "");
    assert!(text.status.success(), "{}", describe(&text));
    let shown = String::from_utf8_lossy(&text.stdout);
    for column in [
        "delivery_id",
        "state",
        "session_name",
        "first_line",
        "body_path",
        "delivered_at",
    ] {
        assert!(shown.contains(column), "the text names {column}: {shown}");
    }
    assert_eq!(
        shown.matches(SESSION).count(),
        2,
        "one row per delivery: {shown}"
    );
}
