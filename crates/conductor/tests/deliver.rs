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
//! `story:deliver-reaches-bg-session`: a session started with `claude --bg` has no launcher
//! socket. When `claude agents --json` names a live session of that name, the text is typed
//! through `claude attach <id>` in a tmux pane made for the one delivery, on a socket of that
//! delivery's own under `<state>/sessions/deliveries/`, which is ended and removed afterwards,
//! and the delivery is recorded as any other. A socket file no server answers on does not hide
//! the session and is left as it is; one a server answers on is left alone. A session the list
//! does not name live, or names by no id to attach to, is refused. `tests/deliver_attach.rs`
//! holds the route against a real `tmux`.
//!
//! Each case is a [`Scratch`] directory: `home/`, the `HOME` the binary runs with, whose `bin/`
//! holds a fake `tmux` this file writes, first on `PATH`, that records each call's working
//! directory and arguments and touches no real server (it prints a drawn screen for
//! `capture-pane`, names no terminal for `display-message`, so the wait for the pane's reader
//! is skipped, and fails its first calls as `tmux.fail` counts), and a fake `claude` that
//! records its calls in `claude.calls` and prints `agents.json` as the session list, an empty
//! one when the case wrote none; `records/`, where the binary runs; `state/`; and the config
//! file `conductor.yaml` naming them, under the session prefix `ex`.

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
                 if [ \"$3\" = load-buffer ]; then set -- \"$1\" \"$2\" \"$3\" \"$4\" \"$5\" \"$(cat \"$6\")\"; fi\n\
                 {{ printf '%s\\037' \"$(pwd)\"; for a in \"$@\"; do printf '%s\\037' \"$a\"; \
                 done; printf '\\036'; }} >> '{calls}'\n\
                 left=$(cat '{fail}' 2>/dev/null || echo 0)\n\
                 if [ \"$left\" -gt 0 ]; then echo $((left - 1)) > '{fail}'; \
                 echo 'no server running on {SESSION}.tmux' >&2; exit 1; fi\n\
                 [ {status} -eq 0 ] || echo 'no server running on {SESSION}.tmux' >&2\n\
                 for a in \"$@\"; do [ \"$a\" = capture-pane ] && echo 'drawn'; done\n\
                 exit {status}\n",
                calls = dir.join("tmux.calls").display(),
                fail = dir.join("tmux.fail").display(),
            ),
        )
        .expect("write the fake tmux");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        let fake = dir.join("home/bin/claude");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$*\" >> '{calls}'\n\
                 if [ \"$1\" = agents ]; then cat '{list}' 2>/dev/null || echo '[]'; fi\n\
                 exit 0\n",
                calls = dir.join("claude.calls").display(),
                list = dir.join("agents.json").display(),
            ),
        )
        .expect("write the fake claude");
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

    /// Lists the session as `claude agents --json` lists a background one named [`SESSION`]
    /// with the id `id`: with a pid when `live`, else as the list keeps one that exited.
    fn listed(&self, id: &str, live: bool) {
        let mut entry = serde_json::json!({
            "id": id, "sessionId": format!("{id}-0000"), "name": SESSION, "kind": "background",
            "cwd": self.dir.join("home/example-org/cortex"), "status": "idle", "state": "done",
        });
        if live {
            entry["pid"] = serde_json::json!(4242);
        }
        fs::write(
            self.dir.join("agents.json"),
            Value::Array(vec![entry]).to_string(),
        )
        .expect("write the session list");
    }

    /// Makes the fake `tmux` fail its next `calls` calls, as one with no server on the socket.
    fn tmux_fails(&self, calls: u8) {
        fs::write(self.dir.join("tmux.fail"), format!("{calls}\n")).expect("write the count");
    }

    /// What `<state>/sessions` holds, by name in order, a directory with a `/` after it and what
    /// it holds after that.
    fn left_in_sessions(&self) -> Vec<String> {
        fn walk(dir: &Path, under: &str, left: &mut Vec<String>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries {
                let entry = entry.expect("read the sessions directory");
                let name = format!("{under}{}", entry.file_name().to_string_lossy());
                if entry.path().is_dir() {
                    left.push(format!("{name}/"));
                    walk(&entry.path(), &format!("{name}/"), left);
                } else {
                    left.push(name);
                }
            }
        }
        let mut left = Vec::new();
        walk(&self.state().join("sessions"), "", &mut left);
        left.sort();
        left
    }

    /// Every call of the fake `claude`, its arguments on one line, in order.
    fn claude_calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("claude.calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// `conductor <args>` from the records directory, with the case's home, config file and fake
    /// `tmux` and `claude` first on `PATH`, and `stdin` on standard input.
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
/// loaded from its paste file (the fake logs the file's content in its place) as one bracketed
/// paste, then a carriage return pasted unbracketed.
fn typed(case: &Case, text: &str) -> Vec<Vec<String>> {
    typed_on(&case.state().join("sessions"), SESSION, text)
}

/// [`typed`]'s four calls for the tmux session `name` on `<name>.tmux`, made from `dir`.
fn typed_on(dir: &Path, name: &str, text: &str) -> Vec<Vec<String>> {
    let call = called(dir, name);
    let pane = format!("={name}:");
    vec![
        call(&["load-buffer", "-b", "conductor", text]),
        call(&["paste-buffer", "-d", "-p", "-b", "conductor", "-t", &pane]),
        call(&["set-buffer", "-b", "conductor", "--", "\r"]),
        call(&["paste-buffer", "-d", "-b", "conductor", "-t", &pane]),
    ]
}

/// A recorded call of the fake `tmux` from `dir` on the socket `<name>.tmux`, by its arguments.
fn called(dir: &Path, name: &str) -> impl Fn(&[&str]) -> Vec<String> {
    let dir = dir.display().to_string();
    let socket = format!("{name}.tmux");
    move |args: &[&str]| -> Vec<String> {
        [dir.as_str(), "-S", socket.as_str()]
            .iter()
            .chain(args)
            .map(|word| (*word).to_owned())
            .collect()
    }
}

/// The name of the pane a delivery made for itself, read from `call`, the fake `tmux`'s record
/// of its `new-session`: `<SESSION>-<process id>-<count>`, on a socket of that name under
/// `<state>/sessions/deliveries`, so no two deliveries share one and none is on the session's
/// launcher socket path.
fn pane_of(case: &Case, call: &[String]) -> String {
    assert_eq!(
        call[0],
        case.state()
            .join("sessions/deliveries")
            .display()
            .to_string(),
        "the pane's socket is in the deliveries' own directory: {call:?}"
    );
    let pane = call[2]
        .strip_suffix(".tmux")
        .unwrap_or_else(|| panic!("no socket: {call:?}"))
        .to_owned();
    let numbers: Vec<&str> = pane
        .strip_prefix(&format!("{SESSION}-"))
        .unwrap_or_else(|| panic!("not named after the session: {pane}"))
        .split('-')
        .collect();
    assert!(
        numbers.len() == 2
            && numbers
                .iter()
                .all(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())),
        "the process id and a count: {pane}"
    );
    pane
}

/// The calls a delivery to a background session with the id `id` makes, the first of which is
/// `first`: the pane that runs `claude attach <id>` on a socket of the delivery's own
/// ([`pane_of`]), with no configuration file, and one look at what it drew; [`typed`]'s four on
/// it; the question which terminal the pane has, whose input the delivery waits to see taken;
/// and the end of that server.
fn typed_through_attach(case: &Case, first: &[String], id: &str, text: &str) -> Vec<Vec<String>> {
    let pane = pane_of(case, first);
    let dir = case.state().join("sessions/deliveries");
    let call = called(&dir, &pane);
    let target = format!("={pane}:");
    let mut calls = vec![
        call(&[
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            &pane,
            "-x",
            "160",
            "-y",
            "50",
            "--",
            "claude",
            "attach",
            id,
        ]),
        call(&["capture-pane", "-p", "-t", &target]),
    ];
    calls.extend(typed_on(&dir, &pane, text));
    calls.push(call(&[
        "display-message",
        "-p",
        "-t",
        &target,
        "#{pane_tty}",
    ]));
    calls.push(call(&["kill-server"]));
    calls
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
    let text = "[DECISION R-7] A merge now (DEC-20991231-01)\nThe release waits.\n";
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
        "[DECISION R-7] A merge now (DEC-20991231-01)",
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
        !case.state().join("store.sqlite").exists(),
        "no store is created for a delivery that was not made"
    );
}

/// `story:deliver-reaches-bg-session`, acceptance 1: a session started with `claude --bg` has
/// no socket; the session list names it live, so the text is typed through `claude attach` in a
/// pane of its own, the delivery is recorded, and no socket is left behind.
#[test]
fn a_background_session_without_a_socket_is_typed_through_claude_attach_and_recorded() {
    let case = Case::new("background", 0);
    case.listed("cc062faf", true);
    let text = "[DECISION R-7] A merge now (DEC-20991231-01)\nThe release waits.\n";
    let output = case.deliver_stdin(text);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(case.claude_calls(), ["agents --json"], "the list is read");
    let calls = case.calls();
    assert_eq!(
        calls,
        typed_through_attach(&case, &calls[0], "cc062faf", text.trim_end_matches('\n')),
        "a pane on `claude attach`, the text once, then Enter, then the pane's end"
    );
    let files: Vec<PathBuf> = fs::read_dir(case.state().join("mailbox/delivered"))
        .expect("the delivered mailbox is there")
        .map(|entry| entry.expect("read it").path())
        .collect();
    assert_eq!(files.len(), 1, "one file: {files:?}");
    let name = files[0].file_name().expect("a name").to_string_lossy();
    recorded(
        &case,
        "[DECISION R-7] A merge now (DEC-20991231-01)",
        &format!("mailbox/delivered/{name}"),
        SESSION,
    );
    assert_eq!(
        case.left_in_sessions(),
        ["deliveries/"],
        "no launcher socket was made, and the pane's own is gone with it"
    );
}

/// A session the list keeps without a pid has ended: refused, nothing typed or recorded.
#[test]
fn a_session_the_list_keeps_without_a_pid_is_not_reachable() {
    let case = Case::new("exited", 0);
    case.listed("cc062faf", false);
    let output = case.deliver_stdin("[HANDOVER cortex] context 210k\n");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("SessionNotReachable")
            && stderr.contains(SESSION)
            && stderr.contains("lists no live session"),
        "the refusal names the error, the session and the list: {stderr}"
    );
    assert!(case.calls().is_empty(), "tmux is never called");
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
    assert!(!case.state().join("mailbox").exists(), "no body is kept");
}

/// A socket file no server answers on, left behind, does not hide a live background session:
/// the text goes through `claude attach`, after the one refused call and one that found no
/// server. The file is not the delivery's, and stays.
#[test]
fn a_socket_no_server_answers_on_does_not_hide_a_live_background_session() {
    let case = Case::new("stale-socket", 0);
    case.socket();
    case.listed("cc062faf", true);
    case.tmux_fails(2);
    let text = "[REPORT cortex -] watch: CI red on main\n";
    let (output, file) = case.deliver_file(text);
    assert!(output.status.success(), "{}", describe(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 2 + 8, "{calls:?}");
    assert_eq!(calls[0][3], "load-buffer", "the launcher is tried first");
    assert_eq!(calls[1][3], "list-sessions", "then asked whether it runs");
    assert_eq!(
        calls[2..],
        typed_through_attach(&case, &calls[2], "cc062faf", text.trim_end_matches('\n'))
    );
    recorded(
        &case,
        "[REPORT cortex -] watch: CI red on main",
        &file.display().to_string(),
        "cortex",
    );
    assert_eq!(
        case.left_in_sessions(),
        ["deliveries/".to_owned(), format!("{SESSION}.tmux")],
        "the file left behind is as it was"
    );
}

/// A live session the list names by an id `claude attach` would read as a flag or as two words
/// (a leading dash, whitespace), or by none, is refused before any pane is made: one line that
/// says so, no tmux call, nothing recorded.
#[test]
fn a_live_session_named_by_no_id_to_attach_to_is_refused_before_tmux_is_called() {
    for (name, id) in [
        ("id-dash", Some("--dangerously-skip-permissions")),
        ("id-space", Some("cc062faf --resume")),
        ("id-tab", Some("cc062faf\tx")),
        ("id-none", None),
    ] {
        let case = Case::new(name, 0);
        let mut entry = serde_json::json!({
            "name": SESSION, "kind": "background", "status": "idle", "state": "done", "pid": 4242,
        });
        if let Some(id) = id {
            entry["id"] = serde_json::json!(id);
        }
        fs::write(
            case.dir.join("agents.json"),
            Value::Array(vec![entry]).to_string(),
        )
        .expect("write the session list");
        let output = case.deliver_stdin("[HANDOVER cortex] context 210k\n");
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: {}",
            describe(&output)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.trim().lines().count(),
            1,
            "{name}: one line: {stderr}"
        );
        assert!(
            stderr.contains("SessionNotReachable")
                && stderr.contains(SESSION)
                && stderr.contains("names its live session by no id to attach to"),
            "{name}: the refusal names the error, the session and the id: {stderr}"
        );
        assert_eq!(case.claude_calls(), ["agents --json"], "{name}: no attach");
        assert!(case.calls().is_empty(), "{name}: tmux is never called");
        assert!(case.dispatch_lines().is_empty(), "{name}: no dispatch line");
        assert!(
            !case.state().join("mailbox").exists(),
            "{name}: no body is kept"
        );
    }
}

/// A socket a tmux server answers on is a launcher's: when it does not take the text, its
/// server is neither replaced nor ended, and the delivery is refused on one line.
#[test]
fn a_launcher_that_answers_but_does_not_take_the_text_is_left_alone() {
    let case = Case::new("launcher-kept", 0);
    case.socket();
    case.listed("cc062faf", true);
    case.tmux_fails(1);
    let (output, _) = case.deliver_file("[REPORT cortex -] watch: CI red on main\n");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(stderr.contains("SessionNotReachable"), "{stderr}");
    let calls = case.calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[1][3], "list-sessions", "{calls:?}");
    assert!(
        case.state()
            .join(format!("sessions/{SESSION}.tmux"))
            .exists(),
        "the launcher's socket stays"
    );
    assert!(case.dispatch_lines().is_empty(), "no dispatch line");
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
