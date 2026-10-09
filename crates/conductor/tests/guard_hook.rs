//! `story:guard-hook`: the built `conductor guard record-guard-decision --from-pre-tool-use` as
//! Claude Code runs it, and the settings files that wire it.
//!
//! Claude Code reads a PreToolUse hook's answer from its exit status: 0 lets the call proceed, 2
//! blocks it and hands standard error to the session, and any other status is a non-blocking
//! error that lets the call proceed. So the hook answers a denial with exit 2 and its reason on
//! standard error, an allowance with exit 0 and no output, and never any other status.
//!
//! Only denials are recorded (correction 1, option A): an allowance is answered without opening
//! the store, which verifies its whole history each time it opens.
//!
//! Each case runs the binary with `HOME` set to a directory of its own under this test target's
//! temporary directory, a recorded payload (`tests/fixtures/guard/`) localised under it on
//! standard input, and `--state-dir` under the same case directory.

mod common;

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use clap::Parser as _;
use conductor_cli::cli::Cli;
use conductor_cli::config::{self, CONFIG_VARIABLE};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The hook command both settings files wire, as the coordinator decided it (DEC 3 of the brief).
/// Correction 2 appends `|| exit 2`: any failure to run the guard (`conductor` not on `PATH` is
/// 127) is a denial, since Claude Code lets a call proceed on any status but 2.
const GUARD_COMMAND: &str = "conductor guard record-guard-decision --from-pre-tool-use || exit 2";

/// The suffix that turns every failure of the wired command into a denial.
const OR_DENY: &str = " || exit 2";

/// The tools the guard is wired on, sorted.
const MATCHERS: [&str; 5] = ["Bash", "Edit", "NotebookEdit", "SendMessage", "Write"];

/// The repository the probe session ran in.
const PROBE: &str = "~/example-org/.guard-probe-w04";

/// A directory of its own for one case, holding `home/` and `state/`.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_hook")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    dir
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/guard")
        .join(format!("{name}.json"));
    serde_json::from_str(&fs::read_to_string(path).expect("read the recorded payload"))
        .expect("the recorded payload is JSON")
}

fn under(home: &Path, value: &str) -> String {
    match value.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home.display()),
        None => value.to_owned(),
    }
}

fn localise(value: Value, home: &Path) -> Value {
    match value {
        Value::String(text) => Value::String(under(home, &text)),
        Value::Array(items) => Value::Array(items.into_iter().map(|v| localise(v, home)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, v)| (key, localise(v, home)))
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

/// The recorded payload `name` under the case's home, with `set` replacing `tool_input` fields.
fn payload(name: &str, case: &Path, set: &[(&str, &str)]) -> Vec<u8> {
    let home = case.join("home");
    let mut payload = localise(fixture(name), &home);
    payload["cwd"] = Value::String(under(&home, PROBE));
    for (field, value) in set {
        payload["tool_input"][*field] = Value::String(under(&home, value));
    }
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Runs the hook as Claude Code does: the payload on standard input, `HOME` the case's home, and
/// no config file named.
fn hook(case: &Path, state: &Path, stdin: &[u8]) -> Output {
    hook_with(case, state, None, stdin)
}

/// Runs the hook as [`hook`] does, with `CONDUCTOR_CONFIG` naming `config` when one is given.
fn hook_with(case: &Path, state: &Path, config: Option<&Path>, stdin: &[u8]) -> Output {
    let mut command = common::conductor(case.join("home"));
    command
        .args(["--state-dir"])
        .arg(state)
        .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
        .current_dir(case)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(config) = config {
        command.env(CONFIG_VARIABLE, config);
    }
    let mut child = command.spawn().expect("the conductor binary runs");
    child
        .stdin
        .take()
        .expect("the hook's standard input")
        .write_all(stdin)
        .expect("write the payload");
    child.wait_with_output().expect("the hook ends")
}

/// Runs the hook as the shipped settings do: no `--state-dir`, the working directory `cwd`, `HOME`
/// the case's home, and `CONDUCTOR_CONFIG` naming `config` when one is given.
fn hook_unflagged(case: &Path, cwd: &Path, config: Option<&Path>, stdin: &[u8]) -> Output {
    let mut command = common::conductor(case.join("home"));
    command
        .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(config) = config {
        command.env(CONFIG_VARIABLE, config);
    }
    let mut child = command.spawn().expect("the conductor binary runs");
    child
        .stdin
        .take()
        .expect("the hook's standard input")
        .write_all(stdin)
        .expect("write the payload");
    child.wait_with_output().expect("the hook ends")
}

/// Without `--state-dir` and without a config file, a denial is answered and recorded nowhere: the
/// hook runs in a session's repository, and `state/` there would plant a store in it.
#[test]
fn without_a_state_dir_or_a_config_file_a_denial_is_recorded_nowhere() {
    let case = case_dir("unflagged-no-config");
    let repository = case.join("repository");
    fs::create_dir_all(&repository).expect("create the session's repository");
    let output = hook_unflagged(
        &case,
        &repository,
        None,
        &payload_at("send-message", &case, &repository, &[("to", "nobody")]),
    );
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    let said = stderr(&output);
    assert!(
        said.contains("the verdict was not recorded: no --state-dir is given"),
        "{said:?}"
    );
    assert!(
        !repository.join("state").exists() && !case.join("state").exists(),
        "a store was planted"
    );
}

/// Without `--state-dir`, a denial is recorded in the state of the instance the config file names.
#[test]
fn without_a_state_dir_a_denial_is_recorded_in_the_instance_s_state() {
    let case = case_dir("unflagged-config");
    let config = config_file(&case, "conductor.yaml", "");
    let repository = case.join("root/alpha");
    fs::create_dir_all(&repository).expect("create the checkout");
    let output = hook_unflagged(
        &case,
        &repository,
        Some(&config),
        &payload_at("send-message", &case, &repository, &[("to", "nobody")]),
    );
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    let state = case.join("home/.b10x/conductor/fixture/state");
    assert_eq!(
        decisions(&case, &state).len(),
        1,
        "stderr: {}",
        stderr(&output)
    );
    assert!(!repository.join("state").exists(), "a store was planted");
}

/// The recorded verdicts under `state`, as `guard guard-decisions --format json` lists them.
fn decisions(case: &Path, state: &Path) -> Vec<Value> {
    let output = common::conductor(case.join("home"))
        .arg("--state-dir")
        .arg(state)
        .args(["guard", "guard-decisions", "--format", "json"])
        .current_dir(case)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(
        output.status.success(),
        "`guard guard-decisions` exited {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    match serde_json::from_slice(&output.stdout).expect("the view's output is JSON") {
        Value::Array(rows) => rows,
        other => panic!("the view's output is not an array: {other}"),
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn an_allowance_is_answered_with_exit_0_and_never_opens_the_store() {
    let case = case_dir("allow-only");
    let state = case.join("state");
    for name in ["write", "edit", "notebook-edit"] {
        let allowed = hook(&case, &state, &payload(name, &case, &[]));
        assert_eq!(
            allowed.status.code(),
            Some(0),
            "{name}: {}",
            stderr(&allowed)
        );
        assert!(allowed.stdout.is_empty(), "{name}: nothing on stdout");
        assert!(allowed.stderr.is_empty(), "{name}: {}", stderr(&allowed));
    }
    assert!(
        !state.exists(),
        "an allowance creates no store: {:?}",
        fs::read_dir(&case).map(|dir| dir
            .flatten()
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>())
    );
}

#[test]
fn the_hook_denies_with_exit_2_and_allows_with_exit_0_and_records_the_denial() {
    let case = case_dir("deny-and-allow");
    let state = case.join("state");

    let denied = hook(&case, &state, &payload("send-message", &case, &[]));
    assert_eq!(denied.status.code(), Some(2), "stderr: {}", stderr(&denied));
    assert!(
        denied.stdout.is_empty(),
        "a denial writes nothing to stdout"
    );
    let reason = stderr(&denied);
    assert_eq!(reason.lines().count(), 1, "one line: {reason:?}");
    assert!(
        reason.starts_with("conductor guard: denied: ") && reason.contains("only conductor"),
        "{reason:?}"
    );

    let allowed = hook(&case, &state, &payload("write", &case, &[]));
    assert_eq!(
        allowed.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&allowed)
    );
    assert!(
        allowed.stdout.is_empty(),
        "an allowance writes nothing to stdout"
    );
    assert!(allowed.stderr.is_empty(), "stderr: {}", stderr(&allowed));

    let bash = hook(&case, &state, &payload("bash", &case, &[]));
    assert_eq!(bash.status.code(), Some(2), "stderr: {}", stderr(&bash));

    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), 2, "the two denials and no allowance: {rows:#?}");
    let session = "00000000-0000-4000-8000-0000000000a1";
    for (row, tool, target) in [
        (&rows[0], "SendMessage", "guard-probe-nobody-w04"),
        (&rows[1], "Bash", "git -C ../conductor status"),
    ] {
        assert_eq!(row["repository"], ".guard-probe-w04", "{row:#}");
        assert_eq!(row["session_ref"], session, "{row:#}");
        assert_eq!(row["tool"], tool, "{row:#}");
        assert_eq!(row["target"], target, "{row:#}");
        assert_eq!(row["verdict"], "Deny", "{row:#}");
        assert!(
            !row["reason"].as_str().expect("a reason").is_empty(),
            "{row:#}"
        );
        let decided_at = row["decided_at"].as_str().expect("a timestamp");
        OffsetDateTime::parse(decided_at, &Rfc3339).expect("decided_at is RFC 3339");
        let id = row["guard_decision_id"].as_str().expect("an id");
        assert_eq!(id.len(), 36, "a canonical UUID: {id}");
    }
    assert!(
        rows[0]["reason"]
            .as_str()
            .is_some_and(|recorded| reason.contains(recorded)),
        "the recorded reason is the one the session was given: {reason:?} vs {:#}",
        rows[0]
    );
}

#[test]
fn an_unreadable_payload_is_denied_and_recorded() {
    let case = case_dir("unreadable");
    let state = case.join("state");
    for stdin in [&b"not json"[..], b"", b"[]"] {
        let output = hook(&case, &state, stdin);
        assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
        assert!(stderr(&output).starts_with("conductor guard: denied: "));
    }
    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), 3, "{rows:#?}");
    assert!(rows.iter().all(|row| row["verdict"] == "Deny"), "{rows:#?}");
}

#[test]
fn the_verdict_is_answered_when_the_store_cannot_record_it() {
    let case = case_dir("unwritable");
    let state = case.join("state");
    fs::write(&state, "a file where the state directory should be").expect("plant a file");

    let denied = hook(&case, &state, &payload("bash", &case, &[]));
    assert_eq!(denied.status.code(), Some(2), "stderr: {}", stderr(&denied));
    let lines: Vec<String> = stderr(&denied).lines().map(str::to_owned).collect();
    assert_eq!(
        lines.len(),
        2,
        "the reason and one line on the store: {lines:#?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("conductor guard: denied: "))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("conductor guard: the verdict was not recorded: ")),
        "{lines:#?}"
    );

    // An allowance does not touch the store, so the store's state says nothing to it.
    let allowed = hook(&case, &state, &payload("write", &case, &[]));
    assert_eq!(
        allowed.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&allowed)
    );
    assert!(allowed.stderr.is_empty(), "stderr: {}", stderr(&allowed));
}

#[test]
fn a_held_writer_lock_delays_a_denial_by_at_most_two_seconds_and_an_allowance_not_at_all() {
    let case = case_dir("held-lock");
    let state = case.join("state");
    let first = hook(&case, &state, &payload("bash", &case, &[]));
    assert_eq!(first.status.code(), Some(2), "stderr: {}", stderr(&first));

    let lock = File::options()
        .read(true)
        .write(true)
        .open(state.join("tree/.lock"))
        .expect("open the store's writer lock");
    lock.lock().expect("hold the writer lock");

    let started = Instant::now();
    let held = hook(&case, &state, &payload("send-message", &case, &[]));
    let waited = started.elapsed();
    let started = Instant::now();
    let allowed = hook(&case, &state, &payload("write", &case, &[]));
    let answered = started.elapsed();
    lock.unlock().expect("release the writer lock");

    assert_eq!(
        allowed.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&allowed)
    );
    assert!(allowed.stderr.is_empty(), "stderr: {}", stderr(&allowed));
    assert!(
        answered < Duration::from_millis(1000),
        "an allowance waited {answered:?} on a held lock"
    );

    assert_eq!(held.status.code(), Some(2), "stderr: {}", stderr(&held));
    assert!(
        waited >= Duration::from_millis(1900) && waited < Duration::from_secs(5),
        "the hook waited {waited:?} on a held lock"
    );
    let lines: Vec<String> = stderr(&held).lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("conductor guard: the verdict was not recorded: ")),
        "{lines:#?}"
    );
    assert_eq!(
        decisions(&case, &state).len(),
        1,
        "the delayed verdict was not kept"
    );
}

#[test]
fn hooks_running_at_once_each_answer_and_each_denial_is_recorded() {
    let case = case_dir("concurrent");
    let state = case.join("state");
    let calls: Vec<(Vec<u8>, i32)> = (0..8)
        .map(|n| {
            if n % 2 == 0 {
                (payload("send-message", &case, &[]), 2)
            } else {
                (payload("edit", &case, &[]), 0)
            }
        })
        .collect();
    let answers: Vec<(Output, i32)> = std::thread::scope(|scope| {
        let running: Vec<_> = calls
            .iter()
            .map(|(stdin, expected)| {
                let (case, state) = (&case, &state);
                scope.spawn(move || (hook(case, state, stdin), *expected))
            })
            .collect();
        running
            .into_iter()
            .map(|handle| handle.join().expect("the hook's thread"))
            .collect()
    });
    for (output, expected) in &answers {
        assert_eq!(
            output.status.code(),
            Some(*expected),
            "stderr: {}",
            stderr(output)
        );
        assert!(
            !stderr(output).contains("not recorded"),
            "stderr: {}",
            stderr(output)
        );
    }
    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), 4, "the four denials: {rows:#?}");
    assert!(rows.iter().all(|row| row["verdict"] == "Deny"), "{rows:#?}");
}

/// Runs the hook as [`hook`] does, with `PATH` replaced by `path`.
fn hook_on(case: &Path, state: &Path, path: &Path, stdin: &[u8]) -> Output {
    let mut child = common::conductor(case.join("home"))
        .args(["--state-dir"])
        .arg(state)
        .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
        .current_dir(case)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the conductor binary runs");
    child
        .stdin
        .take()
        .expect("the hook's standard input")
        .write_all(stdin)
        .expect("write the payload");
    child.wait_with_output().expect("the hook ends")
}

/// `story:guard-conductor-ref`: the hook reads the session list a socket address needs from
/// `claude agents --json` on `PATH`. A `claude` that prints `tests/fixtures/guard/session-list.json`
/// when given exactly `agents --json` stands in for it; with none on `PATH` the list cannot be
/// read, and the socket is denied naming why.
#[test]
fn the_hook_reads_claude_agents_json_for_a_socket_address() {
    use std::os::unix::fs::PermissionsExt as _;

    let case = case_dir("socket");
    let state = case.join("state");
    let list: Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/guard/session-list.json"),
        )
        .expect("read the session list"),
    )
    .expect("the session list is JSON");
    let list = serde_json::to_string(&list).expect("serialise the session list");
    assert!(!list.contains('\''), "the list fits in single quotes");
    let bin = case.join("bin");
    fs::create_dir_all(&bin).expect("create the PATH directory");
    let claude = bin.join("claude");
    fs::write(
        &claude,
        format!("#!/bin/sh\n[ \"$*\" = 'agents --json' ] || exit 9\nprintf '%s\\n' '{list}'\n"),
    )
    .expect("write a fake claude");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it executable");
    let empty = case.join("empty-path");
    fs::create_dir_all(&empty).expect("create an empty PATH directory");

    let conductor = "uds:/run/user/4242/cc-socks/4100001.sock";
    let connectors = "uds:/run/user/4242/cc-socks/4100011.sock";
    let send = |to: &str| payload("send-message", &case, &[("to", to), ("recipient", to)]);

    let allowed = hook_on(&case, &state, &bin, &send(conductor));
    assert_eq!(
        allowed.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&allowed)
    );
    assert!(allowed.stderr.is_empty(), "stderr: {}", stderr(&allowed));

    let denied = hook_on(&case, &state, &bin, &send(connectors));
    assert_eq!(denied.status.code(), Some(2), "stderr: {}", stderr(&denied));
    assert!(
        stderr(&denied).contains(connectors) && stderr(&denied).contains("only conductor"),
        "{}",
        stderr(&denied)
    );

    let unread = hook_on(&case, &state, &empty, &send(conductor));
    assert_eq!(unread.status.code(), Some(2), "stderr: {}", stderr(&unread));
    assert!(
        stderr(&unread).contains("session list") && stderr(&unread).contains("`claude`"),
        "{}",
        stderr(&unread)
    );

    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), 2, "the two denials: {rows:#?}");
    assert_eq!(rows[0]["target"], connectors, "{:#}", rows[0]);
    assert_eq!(rows[1]["target"], conductor, "{:#}", rows[1]);
}

// -------------------------------------------------------------------------------------------------
// The instance's places. The hook decides by the instance the process runs: with a config file,
// checkouts under its `checkouts` and conductor's session in its `records`; without one, or with
// one that does not load, the built-in instance's, today's.
// -------------------------------------------------------------------------------------------------

/// A config file of one instance whose checkouts are under the case's `root/`, managed worktrees
/// under `trees/` and records in `records/`, with `extra` appended to the instance.
fn config_text(case: &Path, extra: &str) -> String {
    format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: fixture\n\
         \x20   sources: [{{github: fixture}}]\n\
         \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
         \x20   records: {records}\n\
         {extra}",
        root = case.join("root").display(),
        trees = case.join("trees").display(),
        records = case.join("records").display(),
    )
}

/// Writes [`config_text`] to `file` under the case directory, and answers its path.
fn config_file(case: &Path, file: &str, extra: &str) -> PathBuf {
    let path = case.join(file);
    fs::create_dir_all(path.parent().expect("the file's directory"))
        .expect("create the file's directory");
    fs::write(&path, config_text(case, extra)).expect("write the config file");
    path
}

/// The recorded payload `name` from a session whose cwd is `cwd`, with `set` replacing
/// `tool_input` fields; a SendMessage's `recipient` is its `to`.
fn payload_at(name: &str, case: &Path, cwd: &Path, set: &[(&str, &str)]) -> Vec<u8> {
    let mut payload = localise(fixture(name), &case.join("home"));
    payload["cwd"] = Value::String(cwd.display().to_string());
    for (field, value) in set {
        payload["tool_input"][*field] = Value::String((*value).to_owned());
        if *field == "to" {
            payload["tool_input"]["recipient"] = Value::String((*value).to_owned());
        }
    }
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Asserts `output` answers `code`, and for a denial that its reason names `because`.
#[track_caller]
fn assert_answer(output: &Output, code: i32, because: &str, what: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{what}: stderr: {}",
        stderr(output)
    );
    if code == 0 {
        assert!(output.stderr.is_empty(), "{what}: {}", stderr(output));
    } else {
        assert!(
            stderr(output).contains(because),
            "{what}: the reason does not name {because:?}: {}",
            stderr(output)
        );
    }
}

/// Acceptance: with a config file naming checkouts and records under the case directory, a session
/// whose cwd is in `records/` runs conductor's rules: it writes its records and nothing else,
/// messages anyone, runs every leaf and makes no GitHub write, and its denials are recorded as
/// conductor's. The file is read where `CONDUCTOR_CONFIG` names it and at the default path; without
/// it the same session is in no place a rule set applies to.
#[test]
fn with_a_config_file_a_session_in_records_runs_conductor_s_rules() {
    let case = case_dir("config-conductor");
    let named = config_file(&case, "conductor.yaml", "");
    let default = config_file(&case, &format!("home/{}", config::DEFAULT_FILE), "");
    let records = case.join("records");
    let at = |rest: &str| case.join(rest).display().to_string();
    for (how, config, state) in [
        ("named", Some(named.as_path()), case.join("state-named")),
        ("default", None, case.join("state-default")),
    ] {
        for target in [
            "records/decisions/2026-10.jsonl",
            "records/STATUS.md",
            "records/docs/handoff/2026-10-08.md",
        ] {
            let output = hook_with(
                &case,
                &state,
                config,
                &payload_at("write", &case, &records, &[("file_path", &at(target))]),
            );
            assert_answer(&output, 0, "", &format!("{how}: write {target}"));
        }
        for (name, field, value) in [
            ("send-message", "to", "nobody"),
            (
                "bash",
                "command",
                "conductor decision record-operator-decision --input-json -",
            ),
        ] {
            let output = hook_with(
                &case,
                &state,
                config,
                &payload_at(name, &case, &records, &[(field, value)]),
            );
            assert_answer(&output, 0, "", &format!("{how}: {name} {value}"));
        }
        for (name, field, value, because) in [
            (
                "write",
                "file_path",
                at("records/crates/x"),
                "conductor writes only its records",
            ),
            (
                "edit",
                "file_path",
                at("root/repo/x"),
                "conductor writes only its records",
            ),
            (
                "bash",
                "command",
                "gh pr merge 3".to_owned(),
                "GitHub write",
            ),
        ] {
            let output = hook_with(
                &case,
                &state,
                config,
                &payload_at(name, &case, &records, &[(field, &value)]),
            );
            assert_answer(&output, 2, because, &format!("{how}: {name} {value}"));
        }
        let rows = decisions(&case, &state);
        assert_eq!(rows.len(), 3, "{how}: the three denials: {rows:#?}");
        assert!(
            rows.iter().all(|row| row["repository"] == "conductor"),
            "{how}: {rows:#?}"
        );
    }
    fs::remove_file(&default).expect("remove the default file");
    let output = hook(
        &case,
        &case.join("state-none"),
        &payload_at(
            "write",
            &case,
            &records,
            &[("file_path", &at("records/decisions/2026-10.jsonl"))],
        ),
    );
    assert_answer(&output, 2, "where every rule set applies", "no file");
}

/// Acceptance: with the same config file, a session in `<root>/<repo>`, or in one of the repo's
/// managed worktrees under `<trees>/<repo>/`, runs a controller's rules.
#[test]
fn with_a_config_file_a_session_in_a_checkout_runs_a_controller_s_rules() {
    let case = case_dir("config-controller");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let at = |rest: &str| case.join(rest).display().to_string();
    for cwd in [case.join("root/repo"), case.join("trees/repo/t")] {
        let shown = cwd.display();
        for (name, field, value) in [
            ("write", "file_path", at("root/repo/src/lib.rs")),
            ("notebook-edit", "notebook_path", at("trees/repo/t/n.ipynb")),
            ("send-message", "to", "conductor".to_owned()),
            ("bash", "command", "git status".to_owned()),
        ] {
            let output = hook_with(
                &case,
                &state,
                Some(&config),
                &payload_at(name, &case, &cwd, &[(field, &value)]),
            );
            assert_answer(&output, 0, "", &format!("{shown}: {name} {value}"));
        }
        for (name, field, value, because) in [
            (
                "write",
                "file_path",
                at("root/other/x"),
                "outside the repository's checkout",
            ),
            ("send-message", "to", "nobody".to_owned(), "only conductor"),
            (
                "bash",
                "command",
                "conductor decision record-operator-decision --input-json -".to_owned(),
                "only conductor runs it",
            ),
            (
                "bash",
                "command",
                format!("git -C {} status", at("root/other")),
                "reaches another repository, other",
            ),
        ] {
            let output = hook_with(
                &case,
                &state,
                Some(&config),
                &payload_at(name, &case, &cwd, &[(field, &value)]),
            );
            assert_answer(&output, 2, because, &format!("{shown}: {name} {value}"));
        }
    }
    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), 8, "the eight denials: {rows:#?}");
    assert!(
        rows.iter().all(|row| row["repository"] == "repo"),
        "{rows:#?}"
    );
}

/// Acceptance: with the same config file, a controller writing into `records/decisions/` (or any
/// of conductor's records) is denied, with every file tool, from its checkout and from its managed
/// worktrees; and a Bash `cd` into the records is denied as a `cd` into another repository is.
#[test]
fn with_a_config_file_a_controller_writing_into_records_is_denied() {
    let case = case_dir("config-controller-records");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let at = |rest: &str| case.join(rest).display().to_string();
    for cwd in [case.join("root/repo"), case.join("trees/repo/t")] {
        let shown = cwd.display();
        for (name, field) in [
            ("write", "file_path"),
            ("edit", "file_path"),
            ("notebook-edit", "notebook_path"),
        ] {
            for target in [
                "records/decisions/2026-10.jsonl",
                "records/dispatches/2026-10.jsonl",
                "records/STATUS.md",
            ] {
                let output = hook_with(
                    &case,
                    &state,
                    Some(&config),
                    &payload_at(name, &case, &cwd, &[(field, &at(target))]),
                );
                assert_answer(
                    &output,
                    2,
                    "outside the repository's checkout",
                    &format!("{shown}: {name} {target}"),
                );
            }
        }
        let command = format!("cd {} && echo x >> 2026-10.jsonl", at("records/decisions"));
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &payload_at("bash", &case, &cwd, &[("command", &command)]),
        );
        assert_answer(
            &output,
            2,
            "reaches conductor's records",
            &format!("{shown}: {command}"),
        );
    }
}

/// The guard is a security boundary: a config file that does not load (a key the specification
/// does not declare, two instances and none chosen, a named file that is not there) leaves the hook
/// on the built-in places, so every call denied without a file is still denied, and the file's
/// places give nothing.
#[test]
fn a_config_file_that_does_not_load_leaves_the_hook_on_the_built_in_places() {
    let case = case_dir("config-unloadable");
    let home = case.join("home");
    let root = PathBuf::from(config::built_in(&home, &home).checkouts.root);
    let unknown = config_file(&case, "unknown.yaml", "    colour: blue\n");
    let two = config_file(
        &case,
        "two.yaml",
        "  - name: second\n    sources: [{github: second}]\n    checkouts: {root: ~/a, trees: ~/b}\n",
    );
    let missing = case.join("missing.yaml");
    let conductor = root.join("conductor");
    let probe = root.join("probe");
    let records = case.join("records");
    for (how, config) in [
        ("unknown key", &unknown),
        ("two instances", &two),
        ("missing", &missing),
    ] {
        let state = case.join(format!("state-{}", how.replace(' ', "-")));
        let decide = |cwd: &Path, name: &str, field: &str, value: &Path| {
            let value = value.display().to_string();
            hook_with(
                &case,
                &state,
                Some(config),
                &payload_at(name, &case, cwd, &[(field, &value)]),
            )
        };
        assert_answer(
            &decide(
                &conductor,
                "write",
                "file_path",
                &conductor.join("decisions/2026-10.jsonl"),
            ),
            0,
            "",
            &format!("{how}: conductor writes its records"),
        );
        assert_answer(
            &decide(&conductor, "write", "file_path", &conductor.join("src/x")),
            2,
            "conductor writes only its records",
            &format!("{how}: conductor writes no source"),
        );
        assert_answer(
            &decide(&probe, "write", "file_path", &conductor.join("decisions/x")),
            2,
            "outside the repository's checkout",
            &format!("{how}: a controller writes no record"),
        );
        assert_answer(
            &decide(&probe, "send-message", "to", Path::new("nobody")),
            2,
            "only conductor",
            &format!("{how}: a controller messages only conductor"),
        );
        assert_answer(
            &decide(
                &records,
                "write",
                "file_path",
                &records.join("decisions/2026-10.jsonl"),
            ),
            2,
            "where every rule set applies",
            &format!("{how}: the file's records are no place"),
        );
        assert_answer(
            &decide(
                &case.join("root/repo"),
                "write",
                "file_path",
                &case.join("root/repo/x"),
            ),
            2,
            "where every rule set applies",
            &format!("{how}: the file's checkouts are no place"),
        );
    }
}

// -------------------------------------------------------------------------------------------------
// Wave 08 W5: a controller does not write conductor's config, the file the hook process resolves
// (`CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml`), the default file, and the
// instance's state directory (`story:guard-write-forms-and-config-scope`). Conductor's own session
// is decided as before.
// -------------------------------------------------------------------------------------------------

/// What every denial of a write to conductor's config says.
const NOT_CONFIG: &str = "a controller does not write conductor's config";

/// A Read of `file` from a session whose cwd is `cwd`: a tool the guard has no rule for.
fn read_at(case: &Path, cwd: &Path, file: &str) -> Vec<u8> {
    let mut payload = localise(fixture("write"), &case.join("home"));
    payload["cwd"] = Value::String(cwd.display().to_string());
    payload["tool_name"] = Value::String("Read".to_owned());
    payload["tool_input"] = serde_json::json!({ "file_path": file });
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Acceptance: from a controller's cwd (its checkout, and a worker's managed worktree), a Write to
/// the config file the hook resolves, an Edit of a file in the instance's state directory
/// (`~/.b10x/conductor/fixture/state/` by default), `sed -i` and
/// `cp` onto `~/.b10x/conductor/conductor.yaml`, and an `echo … >` to `"$CONDUCTOR_CONFIG"` or to
/// the resolved path are each denied, naming the file, and recorded; reading it, by `cat` or Read,
/// is allowed. The file is named beside the checkouts, named inside the controller's own checkout,
/// and is the default file.
#[test]
fn a_controller_does_not_write_the_config_file_the_hook_resolves() {
    let case = case_dir("config-controller-config");
    let home = case.join("home");
    let beside = config_file(&case, "conductor.yaml", "");
    let inside = config_file(&case, "root/repo/conductor.yaml", "");
    let default = config_file(&case, &format!("home/{}", config::DEFAULT_FILE), "");
    let in_dir = home
        .join(".b10x/conductor/fixture/state/tree/x")
        .display()
        .to_string();
    for (how, named, file, shown) in [
        (
            "beside",
            Some(beside.as_path()),
            &beside,
            beside.display().to_string(),
        ),
        (
            "inside",
            Some(inside.as_path()),
            &inside,
            inside.display().to_string(),
        ),
        (
            "default",
            None,
            &default,
            "~/.b10x/conductor/conductor.yaml".to_owned(),
        ),
    ] {
        let state = case.join(format!("state-{how}"));
        let file = file.display().to_string();
        for cwd in [case.join("root/repo"), case.join("trees/repo/t")] {
            let what =
                |name: &str, value: &str| format!("{how}: {}: {name} {value}", cwd.display());
            for (name, field, value) in [
                ("write", "file_path", file.clone()),
                ("edit", "file_path", in_dir.clone()),
                ("notebook-edit", "notebook_path", file.clone()),
                (
                    "bash",
                    "command",
                    "sed -i 's/a/b/' ~/.b10x/conductor/conductor.yaml".to_owned(),
                ),
                (
                    "bash",
                    "command",
                    "cp x ~/.b10x/conductor/conductor.yaml".to_owned(),
                ),
                (
                    "bash",
                    "command",
                    "echo 'checkouts: {root: /}' > \"$CONDUCTOR_CONFIG\"".to_owned(),
                ),
                ("bash", "command", format!("echo x > {file}")),
            ] {
                let output = hook_with(
                    &case,
                    &state,
                    named,
                    &payload_at(name, &case, &cwd, &[(field, &value)]),
                );
                assert_answer(&output, 2, NOT_CONFIG, &what(name, &value));
                assert_answer(&output, 2, &shown, &what(name, &value));
            }
            for (name, value) in [
                ("bash", "cat ~/.b10x/conductor/conductor.yaml".to_owned()),
                ("bash", "cat \"$CONDUCTOR_CONFIG\"".to_owned()),
                ("bash", format!("cat {file}")),
            ] {
                let output = hook_with(
                    &case,
                    &state,
                    named,
                    &payload_at(name, &case, &cwd, &[("command", &value)]),
                );
                assert_answer(&output, 0, "", &what(name, &value));
            }
            let output = hook_with(&case, &state, named, &read_at(&case, &cwd, &file));
            assert_answer(&output, 0, "", &what("read", &file));
        }
        let rows = decisions(&case, &state);
        assert_eq!(rows.len(), 14, "{how}: the fourteen denials: {rows:#?}");
        assert!(
            rows.iter().all(|row| row["repository"] == "repo"
                && row["reason"]
                    .as_str()
                    .is_some_and(|r| r.contains(NOT_CONFIG))),
            "{how}: {rows:#?}"
        );
    }
}

/// A relative `CONDUCTOR_CONFIG` names the file from the hook's working directory, where the
/// process reads it, and that file is the one a controller does not write.
#[test]
fn a_relative_conductor_config_is_taken_from_the_hook_s_working_directory() {
    let case = case_dir("config-relative");
    let state = case.join("state");
    let file = config_file(&case, "conductor.yaml", "");
    let target = file.display().to_string();
    for cwd in [case.join("root/repo"), case.join("trees/repo/t")] {
        let output = hook_with(
            &case,
            &state,
            Some(Path::new("conductor.yaml")),
            &payload_at("write", &case, &cwd, &[("file_path", &target)]),
        );
        assert_answer(&output, 2, NOT_CONFIG, &format!("{}: write", cwd.display()));
    }
}

/// A config file that does not load leaves the hook on the built-in places, and the file
/// `CONDUCTOR_CONFIG` names is still conductor's: a controller cannot mend it into one that loads.
#[test]
fn a_config_file_that_does_not_load_is_not_a_controller_s_to_mend() {
    let case = case_dir("config-unloadable-controller");
    let state = case.join("state");
    let home = case.join("home");
    let broken = config_file(&case, "broken.yaml", "    colour: blue\n");
    let probe = PathBuf::from(config::built_in(&home, &home).checkouts.root).join("probe");
    let file = broken.display().to_string();
    for (name, field, value) in [
        ("write", "file_path", file.clone()),
        ("bash", "command", format!("sed -i /colour/d {file}")),
        (
            "bash",
            "command",
            "echo x > \"$CONDUCTOR_CONFIG\"".to_owned(),
        ),
    ] {
        let output = hook_with(
            &case,
            &state,
            Some(&broken),
            &payload_at(name, &case, &probe, &[(field, &value)]),
        );
        assert_answer(&output, 2, NOT_CONFIG, &format!("{name} {value}"));
    }
}

/// Without a config file, the default file is still not a controller's to write, so it cannot
/// create the file that would choose the instance; conductor's Bash may.
#[test]
fn without_a_config_file_a_controller_does_not_create_one() {
    let case = case_dir("config-none");
    let state = case.join("state");
    let home = case.join("home");
    let root = PathBuf::from(config::built_in(&home, &home).checkouts.root);
    let file = home.join(config::DEFAULT_FILE).display().to_string();
    for (name, field, value) in [
        ("write", "file_path", file.clone()),
        (
            "bash",
            "command",
            "cp x ~/.b10x/conductor/conductor.yaml".to_owned(),
        ),
        (
            "bash",
            "command",
            "mkdir -p ~/.b10x/conductor && echo x > ~/.b10x/conductor/conductor.yaml".to_owned(),
        ),
    ] {
        let output = hook(
            &case,
            &state,
            &payload_at(name, &case, &root.join("probe"), &[(field, &value)]),
        );
        assert_answer(&output, 2, NOT_CONFIG, &format!("{name} {value}"));
    }
    let output = hook(
        &case,
        &state,
        &payload_at(
            "bash",
            &case,
            &root.join("conductor"),
            &[("command", "cp x ~/.b10x/conductor/conductor.yaml")],
        ),
    );
    assert_answer(&output, 0, "", "conductor's Bash writes the config");
}

/// Acceptance, decision 3: conductor's session is not affected. Its Bash writes the config file the
/// hook resolves, by every form a controller is denied; its file tools write its records and not
/// the config, as before; and with a config file that leaves `records` to its default, under
/// `~/.b10x/conductor/`, it writes its records there, while a controller does not: a controller's
/// file tool is denied it as outside its checkout.
#[test]
fn conductor_s_session_writes_the_config_as_before() {
    let case = case_dir("config-conductor-config");
    let home = case.join("home");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let file = config.display().to_string();
    let records = case.join("records");
    for command in [
        "sed -i 's/a/b/' ~/.b10x/conductor/conductor.yaml".to_owned(),
        "cp x ~/.b10x/conductor/conductor.yaml".to_owned(),
        "echo x > \"$CONDUCTOR_CONFIG\"".to_owned(),
        format!("echo x > {file}"),
    ] {
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &payload_at("bash", &case, &records, &[("command", &command)]),
        );
        assert_answer(&output, 0, "", &format!("conductor: {command}"));
    }
    let record = records
        .join("decisions/2026-10.jsonl")
        .display()
        .to_string();
    for (target, code, because) in [
        (&record, 0, ""),
        (&file, 2, "conductor writes only its records"),
    ] {
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &payload_at("write", &case, &records, &[("file_path", target)]),
        );
        assert_answer(
            &output,
            code,
            because,
            &format!("conductor: write {target}"),
        );
    }

    let defaulted = case.join("records-defaulted.yaml");
    fs::write(
        &defaulted,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: fixture\n\
             \x20   sources: [{{github: fixture}}]\n\
             \x20   checkouts: {{root: {}, trees: {}}}\n",
            case.join("root").display(),
            case.join("trees").display(),
        ),
    )
    .expect("write the config file");
    let records = home.join(".b10x/conductor/fixture/records");
    let record = records
        .join("decisions/2026-10.jsonl")
        .display()
        .to_string();
    for (cwd, code, because) in [
        (records.clone(), 0, ""),
        (case.join("root/repo"), 2, "outside"),
    ] {
        let output = hook_with(
            &case,
            &case.join("state-defaulted"),
            Some(&defaulted),
            &payload_at("write", &case, &cwd, &[("file_path", &record)]),
        );
        assert_answer(
            &output,
            code,
            because,
            &format!("{}: write {record}", cwd.display()),
        );
    }
}

// -------------------------------------------------------------------------------------------------
// The settings files.
// -------------------------------------------------------------------------------------------------

fn settings(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.claude")
        .join(name);
    serde_json::from_str(&fs::read_to_string(&path).expect("read the settings file"))
        .expect("the settings file is JSON")
}

/// The matchers of `settings`' PreToolUse hooks, sorted, after asserting each runs the guard
/// command and nothing else, and that no other hook event is wired.
#[track_caller]
fn guarded_matchers(settings: &Value) -> Vec<String> {
    let hooks = settings["hooks"].as_object().expect("a hooks object");
    assert_eq!(
        hooks.keys().collect::<Vec<_>>(),
        ["PreToolUse"],
        "only PreToolUse is wired"
    );
    let mut matchers: Vec<String> = hooks["PreToolUse"]
        .as_array()
        .expect("a PreToolUse list")
        .iter()
        .map(|entry| {
            let commands = entry["hooks"].as_array().expect("a hooks list");
            assert_eq!(commands.len(), 1, "{entry:#}");
            assert_eq!(commands[0]["type"], "command", "{entry:#}");
            assert_eq!(commands[0]["command"], GUARD_COMMAND, "{entry:#}");
            entry["matcher"].as_str().expect("a matcher").to_owned()
        })
        .collect();
    matchers.sort();
    matchers
}

#[test]
fn the_controller_settings_wire_the_guard_on_the_five_tools() {
    let controller = settings("controller-settings.json");
    assert_eq!(guarded_matchers(&controller), MATCHERS);
    assert_eq!(
        controller["disableAllHooks"],
        Value::Bool(false),
        "pinned off, so no other settings source switches the guard off by merging"
    );
}

#[test]
fn conductor_settings_wire_the_guard_and_keep_background_isolation_off() {
    let conductor = settings("conductor-settings.json");
    assert_eq!(guarded_matchers(&conductor), MATCHERS);
    assert_eq!(conductor["worktree"]["bgIsolation"], "none");
    assert_eq!(conductor["disableAllHooks"], Value::Bool(false));
}

/// Both files run the guard under `sh`, as Claude Code does: with the built binary on `PATH` a
/// denial is 2, and with no `conductor` on `PATH` the shell's 127 becomes 2 as well.
#[test]
fn the_wired_command_denies_when_it_cannot_run_the_guard() {
    let case = case_dir("wired-path");
    let home = case.join("home");
    let bin = Path::new(common::conductor(&home).get_program())
        .parent()
        .expect("the binary's directory")
        .to_owned();
    let empty = case.join("empty-path");
    fs::create_dir_all(&empty).expect("create an empty PATH directory");
    let stdin = payload("send-message", &case, &[]);
    for name in ["controller-settings.json", "conductor-settings.json"] {
        let command = settings(name)["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .expect("a command")
            .to_owned();
        for (path, why) in [
            (&bin, "conductor on PATH"),
            (&empty, "no conductor on PATH"),
        ] {
            let mut child = Command::new("/bin/sh")
                .args(["-c", &command])
                .current_dir(&case)
                .env_clear()
                .env("HOME", &home)
                .env("PATH", path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("sh runs");
            let _ = child.stdin.take().expect("stdin").write_all(&stdin);
            let output = child.wait_with_output().expect("sh ends");
            assert_eq!(
                output.status.code(),
                Some(2),
                "{name}, {why}: {}",
                stderr(&output)
            );
        }
    }
}

#[test]
fn the_wired_command_parses_as_the_guard_hook() {
    let guard = GUARD_COMMAND
        .strip_suffix(OR_DENY)
        .expect("the wired command ends in `|| exit 2`");
    let argv: Vec<String> = guard
        .split(' ')
        .map(|word| word.trim_matches('"').to_owned())
        .collect();
    let cli = Cli::try_parse_from(&argv).expect("the wired command parses");
    assert_eq!(
        cli.state_dir, None,
        "the state directory is the instance's, not a path in the settings"
    );
    let debug = format!("{:?}", cli.group);
    assert!(
        debug.starts_with("Guard(RecordGuardDecision(")
            && debug.contains("from_pre_tool_use: true"),
        "{debug}"
    );
}
