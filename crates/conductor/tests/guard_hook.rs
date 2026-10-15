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
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use clap::Parser as _;
use conductor_cli::cli::Cli;
use conductor_cli::config::{self, CONFIG_VARIABLE};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The hook command the controller settings wire, as the coordinator decided it (DEC 3 of the
/// brief). Correction 2 appends `|| exit 2`: any failure to run the guard (`conductor` not on
/// `PATH` is 127) is a denial, since Claude Code lets a call proceed on any status but 2.
const GUARD_COMMAND: &str = "conductor guard record-guard-decision --from-pre-tool-use || exit 2";

/// The hook command conductor's settings wire: the guard command that also passes the calling
/// session's process, the hook shell's parent, as the SessionStart claim does, so the lease check
/// tells the holder from another process carrying its session id (`story:lease-by-process`).
const CONDUCTOR_GUARD_COMMAND: &str =
    "conductor guard record-guard-decision --from-pre-tool-use --pid \"$PPID\" || exit 2";

/// The suffix that turns every failure of the wired command into a denial.
const OR_DENY: &str = " || exit 2";

/// The matcher the guard is wired on: every tool (`story:guard-all-tools`).
const MATCHERS: [&str; 1] = ["*"];

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
    hook_from(case, state, config, None, stdin)
}

/// Runs the hook as [`hook_with`] does, passing `--pid` with `pid` when one is given, as
/// conductor's settings pass the hook shell's parent (`story:lease-by-process`).
fn hook_from(
    case: &Path,
    state: &Path,
    config: Option<&Path>,
    pid: Option<u32>,
    stdin: &[u8],
) -> Output {
    let mut command = common::conductor(case.join("home"));
    command
        .args(["--state-dir"])
        .arg(state)
        .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
        .current_dir(case)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(pid) = pid {
        command.arg("--pid").arg(pid.to_string());
    }
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
// checkouts under its `checkouts` and conductor's session in its `records`; without one, the
// built-in instance's, today's. One that does not load denies every call.
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

/// `story:class-h-handover`: a `gh` credential write (`gh secret`, `gh auth`, `gh ssh-key`,
/// `gh gpg-key`) is denied to conductor and to a controller as class H, the operator's hands,
/// with the exact command for the operator to run and no bot route; any other `gh` write keeps
/// its text, which names the bot route; a credential read stays allowed.
#[test]
fn a_gh_credential_write_is_handed_to_the_operator_as_class_h() {
    let case = case_dir("class-h-handover");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    for cwd in [case.join("records"), case.join("root/repo")] {
        let shown = cwd.display();
        let run = |command: &str| {
            hook_with(
                &case,
                &state,
                Some(&config),
                &payload_at("bash", &case, &cwd, &[("command", command)]),
            )
        };
        for command in [
            "gh secret set B10X_GATES_POLICY --org example-org --body x",
            "gh secret delete TOKEN -R example-org/repo",
            "gh auth login -h github.com",
            "gh auth logout",
            "gh auth refresh -s admin:org",
            "gh auth token",
            "gh auth setup-git",
            "gh ssh-key add key.pub --title t",
            "gh ssh-key delete 7",
            "gh gpg-key add key.asc",
            "gh gpg-key delete ABC",
        ] {
            let output = run(command);
            for because in ["class H", "the operator's hands", &format!("! {command}")] {
                assert_answer(&output, 2, because, &format!("{shown}: {command}"));
            }
            assert!(
                !stderr(&output).contains("bot route"),
                "{shown}: {command}: a credential write names no bot route: {}",
                stderr(&output)
            );
        }
        for command in [
            "gh pr merge 3",
            "gh release create v1.0.0",
            "gh variable set X",
        ] {
            let output = run(command);
            assert_answer(
                &output,
                2,
                "GitHub write: gh here only reads",
                &format!("{shown}: {command}"),
            );
            assert_answer(
                &output,
                2,
                "writes go through the bot route",
                &format!("{shown}: {command}"),
            );
            assert!(
                !stderr(&output).contains("class H"),
                "{shown}: {command}: {}",
                stderr(&output)
            );
        }
        for command in ["gh secret list", "gh auth status", "gh ssh-key list"] {
            assert_answer(&run(command), 0, "", &format!("{shown}: {command}"));
        }
    }
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
/// does not declare, two instances and none chosen, a named file that is not there) denies every
/// call, naming why (`story:instance-isolation`, `ConfigUnloaded`): a call the built-in places
/// would allow is denied too, every call denied without a file is still denied for its own reason
/// as well, and the file's places give nothing.
#[test]
fn a_config_file_that_does_not_load_denies_every_call() {
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
            2,
            "ConfigUnloaded: the config file does not load",
            &format!("{how}: not even conductor writes its records"),
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

/// The matchers of `settings`' PreToolUse hooks, sorted, after asserting each runs `command` and
/// nothing else, and that the hook events wired are exactly `events`.
#[track_caller]
fn guarded_matchers(settings: &Value, command: &str, events: &[&str]) -> Vec<String> {
    let hooks = settings["hooks"].as_object().expect("a hooks object");
    assert_eq!(
        hooks.keys().map(String::as_str).collect::<Vec<_>>(),
        events,
        "the hook events wired"
    );
    let mut matchers: Vec<String> = hooks["PreToolUse"]
        .as_array()
        .expect("a PreToolUse list")
        .iter()
        .map(|entry| {
            let commands = entry["hooks"].as_array().expect("a hooks list");
            assert_eq!(commands.len(), 1, "{entry:#}");
            assert_eq!(commands[0]["type"], "command", "{entry:#}");
            assert_eq!(commands[0]["command"], command, "{entry:#}");
            entry["matcher"].as_str().expect("a matcher").to_owned()
        })
        .collect();
    matchers.sort();
    matchers
}

#[test]
fn the_controller_settings_wire_the_guard_on_every_tool() {
    let controller = settings("controller-settings.json");
    assert_eq!(
        guarded_matchers(&controller, GUARD_COMMAND, &["PreToolUse"]),
        MATCHERS
    );
    assert_eq!(
        controller["disableAllHooks"],
        Value::Bool(false),
        "pinned off, so no other settings source switches the guard off by merging"
    );
}

#[test]
fn conductor_settings_wire_the_guard_and_keep_background_isolation_off() {
    let conductor = settings("conductor-settings.json");
    // Conductor's settings also claim the instance's conductor lease when a session starts
    // (`story:single-conductor-lease`, `tests/session.rs`).
    assert_eq!(
        guarded_matchers(
            &conductor,
            CONDUCTOR_GUARD_COMMAND,
            &["PreToolUse", "SessionStart"]
        ),
        MATCHERS
    );
    assert_eq!(conductor["worktree"]["bgIsolation"], "none");
    assert_eq!(conductor["disableAllHooks"], Value::Bool(false));
}

/// The tools conductor never calls, which its settings deny so their definitions stop filling
/// its first-turn context (`story:conductor-context-budget`, measured in
/// `docs/analysis/conductor-context-budget.md`).
const CONDUCTOR_DENIED_TOOLS: [&str; 7] = [
    "Artifact",
    "Workflow",
    "ScheduleWakeup",
    "EnterWorktree",
    "AskUserQuestion",
    "ReportFindings",
    "ShareOnboardingGuide",
];

#[test]
fn conductor_settings_deny_the_tools_conductor_never_calls() {
    let conductor = settings("conductor-settings.json");
    let denied: Vec<&str> = conductor["permissions"]["deny"]
        .as_array()
        .unwrap_or_else(|| panic!("conductor's settings deny a list of tools: {conductor:#}"))
        .iter()
        .map(|tool| tool.as_str().expect("a tool name"))
        .collect();
    assert_eq!(denied, CONDUCTOR_DENIED_TOOLS);
    // The controllers' settings are not conductor's to narrow.
    let controller = settings("controller-settings.json");
    assert_eq!(
        controller["permissions"]["deny"],
        Value::Null,
        "{controller:#}"
    );
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

/// Both wired commands parse as the guard hook. The shell expands conductor's `"$PPID"` to the
/// hook shell's parent, a pid, which the guard reads as the calling session's process; the
/// controller's command passes none.
#[test]
fn the_wired_command_parses_as_the_guard_hook() {
    for (wired, pid) in [
        (GUARD_COMMAND, "pid: None"),
        (CONDUCTOR_GUARD_COMMAND, "pid: Some(4242)"),
    ] {
        let guard = wired
            .strip_suffix(OR_DENY)
            .expect("the wired command ends in `|| exit 2`")
            .replace("\"$PPID\"", "4242");
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
                && debug.contains("from_pre_tool_use: true")
                && debug.contains(pid),
            "{wired}: {debug}"
        );
    }
}

// -------------------------------------------------------------------------------------------------
// story:guard-all-tools: every tool reaches the guard, and one outside its ruled set is allowed
// only from a read-only allowlist that a role's `read_only_tools` extends.
// -------------------------------------------------------------------------------------------------

/// What every denial of a tool outside the ruled set and the allowlist names.
const NOT_ALLOWLISTED: &str = "read-only allowlist";

/// A call of `tool` with `input` from a session whose cwd is `cwd`.
fn tool_at(case: &Path, cwd: &Path, tool: &str, input: Value) -> Vec<u8> {
    let mut payload = localise(fixture("write"), &case.join("home"));
    payload["cwd"] = Value::String(cwd.display().to_string());
    payload["tool_name"] = Value::String(tool.to_owned());
    payload["tool_input"] = input;
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Acceptance: from a controller's checkout and from conductor's records, a tool the guard has no
/// rule for and that is on no allowlist (a write-capable MCP tool, a tool Claude Code adds later)
/// is denied with a reason naming the allowlist, and recorded; an allowlisted read tool is
/// allowed with exit 0, and the allowance opens no store.
#[test]
fn an_unknown_tool_is_denied_and_an_allowlisted_read_tool_is_allowed() {
    let case = case_dir("all-tools");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let at = |rest: &str| case.join(rest).display().to_string();
    for cwd in [case.join("root/repo"), case.join("records")] {
        let shown = cwd.display();
        for (tool, input) in [
            (
                "Read",
                serde_json::json!({ "file_path": at("root/repo/src/lib.rs") }),
            ),
            (
                "Grep",
                serde_json::json!({ "pattern": "fn", "path": at("root/repo") }),
            ),
            ("Glob", serde_json::json!({ "pattern": "**/*.rs" })),
            (
                "WebFetch",
                serde_json::json!({ "url": "https://example.org", "prompt": "x" }),
            ),
            ("TodoWrite", serde_json::json!({ "todos": [] })),
            ("ToolSearch", serde_json::json!({ "query": "x" })),
            (
                "ScheduleWakeup",
                serde_json::json!({ "delaySeconds": 1200, "prompt": "x", "reason": "x" }),
            ),
        ] {
            let output = hook_with(
                &case,
                &state,
                Some(&config),
                &tool_at(&case, &cwd, tool, input),
            );
            assert_answer(&output, 0, "", &format!("{shown}: {tool}"));
        }
        assert!(!state.exists(), "{shown}: an allowance creates no store");
    }
    let mut denials = 0;
    for cwd in [case.join("root/repo"), case.join("records")] {
        let shown = cwd.display();
        for (tool, input) in [
            (
                "mcp__filesystem__write_file",
                serde_json::json!({ "path": at("records/STATUS.md"), "content": "x" }),
            ),
            ("FutureWriteTool", serde_json::json!({ "anything": "x" })),
        ] {
            let output = hook_with(
                &case,
                &state,
                Some(&config),
                &tool_at(&case, &cwd, tool, input),
            );
            assert_answer(&output, 2, NOT_ALLOWLISTED, &format!("{shown}: {tool}"));
            denials += 1;
        }
    }
    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), denials, "every denial is recorded: {rows:#?}");
    assert!(
        rows.iter().all(|row| row["reason"]
            .as_str()
            .is_some_and(|r| r.contains(NOT_ALLOWLISTED))),
        "{rows:#?}"
    );
}

/// Acceptance: a role's `read_only_tools` extends the allowlist for that role's sessions only: a
/// tool the controller role lists is allowed to a controller and still denied to conductor, and
/// a tool with a rule of its own stays under that rule whatever the list says.
#[test]
fn a_role_s_read_only_tools_extend_the_allowlist_for_that_role_only() {
    let case = case_dir("all-tools-role");
    let state = case.join("state");
    let config = config_file(
        &case,
        "conductor.yaml",
        "\x20   roles:\n\
         \x20     - {role: conductor, harness: claude, model: opus}\n\
         \x20     - {role: controller, harness: claude, model: opus, \
         read_only_tools: [mcp__example__read_issue, Write]}\n",
    );
    let at = |rest: &str| case.join(rest).display().to_string();
    let read = || serde_json::json!({ "issue": 3 });
    let controller = case.join("root/repo");
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &tool_at(&case, &controller, "mcp__example__read_issue", read()),
    );
    assert_answer(&output, 0, "", "controller: a listed tool");
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &tool_at(
            &case,
            &case.join("records"),
            "mcp__example__read_issue",
            read(),
        ),
    );
    assert_answer(
        &output,
        2,
        NOT_ALLOWLISTED,
        "conductor: the controller's tool",
    );
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &payload_at(
            "write",
            &case,
            &controller,
            &[("file_path", &at("root/other/x"))],
        ),
    );
    assert_answer(
        &output,
        2,
        "outside the repository's checkout",
        "controller: a listed Write is still decided by its rule",
    );
}

/// Monitor runs a shell command as Bash does, so it is decided by the Bash rule: conductor's
/// watch runs, and a controller's GitHub write is denied as it would be under Bash.
#[test]
fn monitor_is_decided_by_the_bash_rule() {
    let case = case_dir("all-tools-monitor");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let monitor = |command: &str| serde_json::json!({ "command": command, "description": "x" });
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &tool_at(
            &case,
            &case.join("records"),
            "Monitor",
            monitor("conductor watch run --follow"),
        ),
    );
    assert_answer(&output, 0, "", "conductor: the watch under Monitor");
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &tool_at(
            &case,
            &case.join("root/repo"),
            "Monitor",
            monitor("gh pr merge 3"),
        ),
    );
    assert_answer(
        &output,
        2,
        "GitHub write",
        "controller: gh pr merge under Monitor",
    );
    let output = hook_with(
        &case,
        &state,
        Some(&config),
        &tool_at(
            &case,
            &case.join("root/repo"),
            "Monitor",
            serde_json::json!({}),
        ),
    );
    assert_answer(
        &output,
        2,
        "names no command",
        "controller: Monitor with no command",
    );
}

/// Acceptance (taken over from `story:guard-bash-write-targets`): conductor writes
/// `<records>/docs/analysis/**` with the file tools, and still not `<records>/other/`.
#[test]
fn conductor_writes_its_analysis_records() {
    let case = case_dir("all-tools-analysis");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    let at = |rest: &str| case.join(rest).display().to_string();
    for name in ["write", "edit"] {
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &payload_at(
                name,
                &case,
                &records,
                &[(
                    "file_path",
                    &at("records/docs/analysis/2026-10-12-guard.md"),
                )],
            ),
        );
        assert_answer(&output, 0, "", &format!("{name} docs/analysis"));
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &payload_at(
                name,
                &case,
                &records,
                &[("file_path", &at("records/other/x.md"))],
            ),
        );
        assert_answer(
            &output,
            2,
            "conductor writes only its records",
            &format!("{name} other/"),
        );
    }
}

// -------------------------------------------------------------------------------------------------
// story:lease-guard: while a live process holds the instance's conductor lease, a conductor-role
// session that is not the holder may read and may not write.
// -------------------------------------------------------------------------------------------------

/// The session that holds the instance's conductor lease in these cases.
const HOLDER: &str = "00000000-0000-4000-8000-0000000000c1";

/// A second conductor-role session of the same instance.
const SECOND: &str = "00000000-0000-4000-8000-0000000000c2";

/// What every denial of a second conductor's write says of it.
const SECOND_CONDUCTOR: &str = "second conductor";

/// A process standing for the holder's session: a `sleep` child of this test, ended when dropped.
struct HolderProcess(Child);

impl HolderProcess {
    fn start() -> Self {
        Self(
            Command::new("sleep")
                .arg("600")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("sleep runs"),
        )
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }

    /// Ends the process and reaps it, so no process has its pid any more.
    fn end(&mut self) {
        self.0.kill().expect("end the holder's process");
        self.0.wait().expect("reap the holder's process");
    }
}

impl Drop for HolderProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Records the conductor lease of `config`'s instance under `state` for `session_id` and `pid`, as
/// the SessionStart hook of conductor's settings claims it.
fn claim_lease(case: &Path, state: &Path, config: &Path, session_id: &str, pid: u32) {
    let output = common::conductor(case.join("home"))
        .env(CONFIG_VARIABLE, config)
        .arg("--state-dir")
        .arg(state)
        .args(["session", "claim", "--session-id", session_id, "--pid"])
        .arg(pid.to_string())
        .current_dir(case)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(
        output.status.success(),
        "`session claim` exited {:?}; stderr: {}",
        output.status.code(),
        stderr(&output)
    );
    let copy = lease_file(state);
    assert_eq!(
        copy["session_id"], session_id,
        "the claim writes the guard's copy of the lease: {copy:#}"
    );
    assert_eq!(copy["pid"], u64::from(pid), "{copy:#}");
    assert_eq!(copy["instance"], "fixture", "{copy:#}");
}

/// The guard's copy of the lease under `state`, as `session claim` writes it.
fn lease_file(state: &Path) -> Value {
    let text = fs::read_to_string(state.join(LEASE_FILE)).expect("read the lease file");
    serde_json::from_str(&text).expect("the lease file is JSON")
}

/// The file the guard reads the lease from, under the state directory.
const LEASE_FILE: &str = "conductor.lease";

/// Writes the guard's copy of the lease under `state` by hand, naming `session_id` and `pid` for
/// the instance of [`config_text`], with no store beside it.
fn write_lease_file(state: &Path, session_id: &str, pid: u32) {
    fs::create_dir_all(state).expect("create the state directory");
    let body = serde_json::json!({ "instance": "fixture", "session_id": session_id, "pid": pid });
    fs::write(state.join(LEASE_FILE), format!("{body}\n")).expect("write the lease file");
}

/// `payload` as the session `session_id` sends it.
fn from_session(payload: &[u8], session_id: &str) -> Vec<u8> {
    let mut payload: Value = serde_json::from_slice(payload).expect("the payload is JSON");
    payload["session_id"] = Value::String(session_id.to_owned());
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Acceptance: while the lease's holder lives, the holder writes its records; a second session
/// in conductor's records is denied a records write, a SendMessage and a Bash write, each naming
/// the holder's session id and pid, and each denial is recorded as its own; it still reads, with
/// Read and with a Bash read. Once no process has the holder's pid, the second session writes as
/// before.
#[test]
fn a_conductor_session_that_does_not_hold_a_live_lease_reads_and_does_not_write() {
    let case = case_dir("lease-guard");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    let record = case
        .join("records/decisions/2026-10.jsonl")
        .display()
        .to_string();
    let mut holder = HolderProcess::start();
    let pid = holder.pid();
    claim_lease(&case, &state, &config, HOLDER, pid);

    let write = payload_at("write", &case, &records, &[("file_path", &record)]);
    let output = hook_with(&case, &state, Some(&config), &from_session(&write, HOLDER));
    assert_answer(&output, 0, "", "the holder writes its records");

    let holder_named = format!("session {HOLDER} (pid {pid})");
    let mut denials = 0;
    for (what, payload) in [
        ("a records write", write.clone()),
        (
            "a SendMessage",
            payload_at("send-message", &case, &records, &[("to", "nobody")]),
        ),
        (
            "a Bash write",
            payload_at(
                "bash",
                &case,
                &records,
                &[("command", &format!("echo x >> {record}"))],
            ),
        ),
        (
            "a conductor leaf that writes",
            payload_at(
                "bash",
                &case,
                &records,
                &[(
                    "command",
                    "conductor decision record-operator-decision --input-json -",
                )],
            ),
        ),
    ] {
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &from_session(&payload, SECOND),
        );
        assert_answer(&output, 2, SECOND_CONDUCTOR, &format!("second: {what}"));
        assert!(
            stderr(&output).contains(&holder_named),
            "second: {what}: the denial does not name {holder_named:?}: {}",
            stderr(&output)
        );
        denials += 1;
    }
    for (what, payload) in [
        (
            "Read",
            tool_at(
                &case,
                &records,
                "Read",
                serde_json::json!({ "file_path": record }),
            ),
        ),
        (
            "a Bash read",
            payload_at(
                "bash",
                &case,
                &records,
                &[("command", "conductor session leases")],
            ),
        ),
    ] {
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &from_session(&payload, SECOND),
        );
        assert_answer(&output, 0, "", &format!("second: {what}"));
    }
    let rows = decisions(&case, &state);
    assert_eq!(rows.len(), denials, "every denial is recorded: {rows:#?}");
    assert!(
        rows.iter().all(|row| row["session_ref"] == SECOND
            && row["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(&holder_named))),
        "{rows:#?}"
    );

    holder.end();
    let output = hook_with(&case, &state, Some(&config), &from_session(&write, SECOND));
    assert_answer(
        &output,
        0,
        "",
        "second: a records write once the holder's process is gone",
    );
}

/// Without a lease, a conductor-role session writes its records as before, and reading the lease
/// creates no store.
#[test]
fn without_a_lease_a_conductor_session_writes_as_before() {
    let case = case_dir("lease-guard-none");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    let record = case
        .join("records/decisions/2026-10.jsonl")
        .display()
        .to_string();
    let write = payload_at("write", &case, &records, &[("file_path", &record)]);
    let output = hook_with(&case, &state, Some(&config), &from_session(&write, SECOND));
    assert_answer(&output, 0, "", "a records write without a lease");
    assert!(!state.exists(), "reading the lease created a store");
}

/// The guard reads the lease from its copy alone, with no store beside it: a copy naming a live
/// holder denies a second session's write, and one naming a pid no process has any more lets it
/// through, opening no store either way.
#[test]
fn the_guard_reads_the_lease_from_its_copy_and_a_dead_holder_lets_the_write_through() {
    let case = case_dir("lease-guard-file");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    let record = case
        .join("records/decisions/2026-10.jsonl")
        .display()
        .to_string();
    let write = from_session(
        &payload_at("write", &case, &records, &[("file_path", &record)]),
        SECOND,
    );

    let mut holder = HolderProcess::start();
    let pid = holder.pid();
    write_lease_file(&state, HOLDER, pid);
    let output = hook_with(&case, &state, Some(&config), &write);
    assert_answer(&output, 2, SECOND_CONDUCTOR, "a live holder in the copy");
    assert!(
        stderr(&output).contains(&format!("session {HOLDER} (pid {pid})")),
        "{}",
        stderr(&output)
    );

    holder.end();
    let state = case.join("state-dead");
    write_lease_file(&state, HOLDER, pid);
    let output = hook_with(&case, &state, Some(&config), &write);
    assert_answer(&output, 0, "", "a dead holder in the copy");
    assert!(
        !state.join("tree").exists(),
        "reading the lease opened a store"
    );
}

// -------------------------------------------------------------------------------------------------
// Security review of story:lease-guard (wave 09): conformance cases for the lease's own contract.
// -------------------------------------------------------------------------------------------------

/// Review finding: while a live holder exists, a second conductor's Bash command that disables the
/// guard's copy of the lease is not decided as a write, so it is allowed; once the file is gone or
/// unreadable the guard lets every write of that session through (`guard.rs` `leased`: "no such
/// regular file, one that does not read ... leaves `decision` as it is"), Edit, Write and
/// SendMessage included. A command that names the lease file is a write to the lease, whatever
/// program it runs.
#[test]
fn a_second_conductor_is_denied_a_bash_command_that_disables_the_lease_file() {
    let case = case_dir("lease-guard-disable");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    let holder = HolderProcess::start();
    claim_lease(&case, &state, &config, HOLDER, holder.pid());
    let lease = state.join(LEASE_FILE).display().to_string();

    let mut allowed = Vec::new();
    for command in [
        format!("chmod 000 {lease}"),
        format!("unlink {lease}"),
        format!("find {} -name {LEASE_FILE} -delete", state.display()),
        format!("python3 -c \"import os; os.remove('{lease}')\""),
    ] {
        let payload = payload_at("bash", &case, &records, &[("command", &command)]);
        let output = hook_with(
            &case,
            &state,
            Some(&config),
            &from_session(&payload, SECOND),
        );
        if output.status.code() != Some(2) || !stderr(&output).contains(SECOND_CONDUCTOR) {
            allowed.push(format!(
                "{command:?} answered {:?}: {}",
                output.status.code(),
                stderr(&output)
            ));
        }
    }
    assert!(
        allowed.is_empty(),
        "a second conductor may disable the lease file while the holder lives:\n{}",
        allowed.join("\n")
    );
    assert_eq!(
        lease_file(&state)["session_id"],
        HOLDER,
        "the lease file still names the holder"
    );
}

/// A Claude Code session standing in a process of its own: a shell that runs the SessionStart
/// claim and then one PreToolUse guard call, each under `sh -c` as Claude Code runs a hook, and
/// then becomes `sleep`, keeping its pid. The claim and the guard each read its pid from the inner
/// shell's `$PPID`, as the shipped settings do.
struct SessionProcess {
    process: Child,
    claim_status: Option<i32>,
    claim_output: String,
    guard_status: Option<i32>,
    guard_stderr: String,
}

impl SessionProcess {
    fn start(
        case: &Path,
        state: &Path,
        config: &Path,
        name: &str,
        session_id: &str,
        tool_call: &[u8],
    ) -> Self {
        let dir = case.join(name);
        fs::create_dir_all(&dir).expect("create the session's directory");
        let start = serde_json::json!({
            "session_id": session_id,
            "hook_event_name": "SessionStart",
            "source": "resume",
            "cwd": case.join("records"),
        });
        fs::write(dir.join("start.json"), start.to_string()).expect("write the start payload");
        fs::write(dir.join("call.json"), tool_call).expect("write the tool call payload");
        let home = case.join("home");
        let binary = common::conductor(&home).get_program().to_owned();
        let script = r#"sh -c '"$0" --state-dir "$1" session claim --from-session-start --pid "$PPID"' "$1" "$2" <"$3/start.json" >"$3/claim.out" 2>&1; echo $? >"$3/claim.status"; sh -c '"$0" --state-dir "$1" guard record-guard-decision --from-pre-tool-use --pid "$PPID"' "$1" "$2" <"$3/call.json" >/dev/null 2>"$3/guard.err"; echo $? >"$3/guard.next" && mv "$3/guard.next" "$3/guard.status"; exec sleep 600"#;
        let mut process = Command::new("/bin/sh")
            .args(["-c", script, name])
            .arg(&binary)
            .arg(state)
            .arg(&dir)
            .current_dir(case.join("records"))
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin")
            .env(CONFIG_VARIABLE, config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("sh runs");
        let done = dir.join("guard.status");
        let started = Instant::now();
        while !done.exists() {
            if started.elapsed() >= Duration::from_secs(60) {
                let _ = process.kill();
                let _ = process.wait();
                panic!("the hooks of {name} did not end within 60 s");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let read = |file: &str| fs::read_to_string(dir.join(file)).unwrap_or_default();
        Self {
            claim_status: read("claim.status").trim().parse().ok(),
            claim_output: read("claim.out"),
            guard_status: read("guard.status").trim().parse().ok(),
            guard_stderr: read("guard.err"),
            process,
        }
    }

    fn pid(&self) -> u32 {
        self.process.id()
    }
}

impl Drop for SessionProcess {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Review finding (`story:lease-by-process`): the guard knew the holder by the payload's
/// `session_id` alone. A second live process that carries the holder's session id (`claude
/// --resume <id>` by hand, which keeps the id, while the holder's process still runs) has its
/// SessionStart claim refused, since its pid is not the holder's, and is denied its writes, since
/// the guard learns the calling process from `--pid "$PPID"`. The holder's own write stays allowed.
#[test]
fn a_second_process_resuming_the_holder_s_session_is_denied_its_writes() {
    let case = case_dir("lease-guard-same-id");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    fs::create_dir_all(&records).expect("create the records directory");
    let record = records
        .join("decisions/2026-10.jsonl")
        .display()
        .to_string();
    let write = from_session(
        &payload_at("write", &case, &records, &[("file_path", &record)]),
        HOLDER,
    );

    let holder = SessionProcess::start(&case, &state, &config, "holder", HOLDER, &write);
    assert_eq!(holder.claim_status, Some(0), "{}", holder.claim_output);
    assert_eq!(lease_file(&state)["pid"], u64::from(holder.pid()));
    assert_eq!(
        holder.guard_status,
        Some(0),
        "the holder writes its records: {}",
        holder.guard_stderr
    );

    let resumed = SessionProcess::start(&case, &state, &config, "resumed", HOLDER, &write);
    assert_ne!(
        resumed.claim_status,
        Some(0),
        "the resumed process's claim is refused while the holder lives: {}",
        resumed.claim_output
    );
    assert_eq!(
        resumed.guard_status,
        Some(2),
        "a second live process of the holder's session id writes while the holder's process lives \
         (lease pid {}, this process {}): {}",
        holder.pid(),
        resumed.pid(),
        resumed.guard_stderr
    );
}

/// `story:lease-by-process`: with `--pid` the guard knows the calling session's process. A call of
/// the holder's session id from the holder's process (the holder, its sub-agents and its Monitor
/// commands, whose hooks the same Claude Code process runs) is allowed; the same call from
/// another live process is denied, naming the holder; another session id is denied from the
/// holder's process as well. Without `--pid` (the controller settings, an older settings file) the
/// holder's session id is allowed as before.
#[test]
fn the_guard_tells_the_holder_s_process_from_another_carrying_its_session_id() {
    let case = case_dir("lease-guard-process");
    let state = case.join("state");
    let config = config_file(&case, "conductor.yaml", "");
    let records = case.join("records");
    fs::create_dir_all(&records).expect("create the records directory");
    let record = records
        .join("decisions/2026-10.jsonl")
        .display()
        .to_string();
    let write = payload_at("write", &case, &records, &[("file_path", &record)]);
    let lease = state.join(LEASE_FILE).display().to_string();
    let monitor = tool_at(
        &case,
        &records,
        "Monitor",
        serde_json::json!({ "command": format!("tail -f {lease}"), "description": "x" }),
    );

    let holder = HolderProcess::start();
    let pid = holder.pid();
    write_lease_file(&state, HOLDER, pid);
    let another = std::process::id();
    let holder_named = format!("session {HOLDER} (pid {pid})");

    for (payload, what) in [
        (&write, "a records write"),
        (&monitor, "a Monitor naming the lease"),
    ] {
        let stdin = from_session(payload, HOLDER);
        let output = hook_from(&case, &state, Some(&config), Some(pid), &stdin);
        assert_answer(&output, 0, "", &format!("{what} from the holder's process"));
        let output = hook_from(&case, &state, Some(&config), None, &stdin);
        assert_answer(
            &output,
            0,
            "",
            &format!("{what} of the holder without --pid"),
        );
        let output = hook_from(&case, &state, Some(&config), Some(another), &stdin);
        assert_answer(
            &output,
            2,
            SECOND_CONDUCTOR,
            &format!("{what} of the holder's session id from another process"),
        );
        assert!(
            stderr(&output).contains(&holder_named),
            "{what}: {}",
            stderr(&output)
        );
        let output = hook_from(
            &case,
            &state,
            Some(&config),
            Some(pid),
            &from_session(payload, SECOND),
        );
        assert_answer(
            &output,
            2,
            SECOND_CONDUCTOR,
            &format!("{what} of another session id from the holder's process"),
        );
    }
}
