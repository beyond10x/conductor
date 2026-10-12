//! `story:single-conductor-lease`: the conductor lease, claimed by the `SessionStart` hook the
//! shipped `.claude/conductor-settings.json` wires, as Claude Code runs it.
//!
//! Claude Code runs a hook command under `sh -c`, from the session's own process, with the
//! SessionStart payload on standard input. So a case runs the settings file's command under
//! `/bin/sh -c` from the process that stands for the session: this test's own process, or a child
//! of it that runs the command and then lives on as `sleep`, keeping its pid. The command reads the
//! session's id from the payload and its process from the shell's `$PPID`.
//!
//! Each case runs with `HOME` set to a directory of its own under this test target's temporary
//! directory, no config file and that directory as the working directory, so the lease is kept in
//! `state/` under it; the view is read with `--state-dir` naming that directory.

mod common;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long a session standing in a child process is given to run its hook.
const HOOK_DEADLINE: Duration = Duration::from_secs(60);

/// A directory of its own for one case, holding `home/`; the lease goes to `state/` beside it.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("session")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    dir
}

/// The shipped settings conductor starts with.
fn conductor_settings() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/conductor-settings.json");
    serde_json::from_str(&fs::read_to_string(&path).expect("read the settings file"))
        .expect("the settings file is JSON")
}

/// The one command the shipped settings run at the start of a session.
fn session_start_command() -> String {
    let settings = conductor_settings();
    let entries = settings["hooks"]["SessionStart"]
        .as_array()
        .expect("the settings wire a SessionStart hook");
    assert_eq!(entries.len(), 1, "{entries:#?}");
    let hooks = entries[0]["hooks"].as_array().expect("a hooks list");
    assert_eq!(hooks.len(), 1, "{hooks:#?}");
    assert_eq!(hooks[0]["type"], "command", "{hooks:#?}");
    hooks[0]["command"].as_str().expect("a command").to_owned()
}

/// `PATH` for the hook: the built binary's directory first, then the system's, for `sleep`.
fn path(home: &Path) -> String {
    let bin = Path::new(common::conductor(home).get_program())
        .parent()
        .expect("the binary's directory")
        .to_owned();
    format!("{}:/usr/bin:/bin", bin.display())
}

/// A SessionStart payload as Claude Code sends it, for the session `session_id`.
fn payload(case: &Path, session_id: &str) -> Vec<u8> {
    json!({
        "session_id": session_id,
        "transcript_path": case.join(format!("home/.claude/projects/{session_id}.jsonl")),
        "cwd": case,
        "hook_event_name": "SessionStart",
        "source": "startup",
    })
    .to_string()
    .into_bytes()
}

/// A `/bin/sh` for the case: its directory, `HOME`, `PATH`, and nothing else of this process's
/// environment, so no config file or instance variable of the shell the tests run in decides it.
fn sh(case: &Path) -> Command {
    let home = case.join("home");
    let mut command = Command::new("/bin/sh");
    command
        .current_dir(case)
        .env_clear()
        .env("HOME", &home)
        .env("PATH", path(&home))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Runs the SessionStart command as a session whose process is this test's own.
fn claim_from_this_process(case: &Path, session_id: &str) -> Output {
    let mut child = sh(case)
        .args(["-c", &session_start_command()])
        .spawn()
        .expect("sh runs");
    let _ = child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&payload(case, session_id));
    child.wait_with_output().expect("sh ends")
}

/// A session whose process is a child of this test: a shell that runs the SessionStart command in
/// a shell of its own, as Claude Code does, and then becomes `sleep`, so its pid lives on.
struct ChildSession {
    process: Child,
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl ChildSession {
    fn start(case: &Path, name: &str, session_id: &str) -> Self {
        let out = case.join(format!("{name}.out"));
        let err = case.join(format!("{name}.err"));
        let status = case.join(format!("{name}.status"));
        let mut process = sh(case)
            .args([
                "-c",
                "sh -c \"$1\" >\"$2\" 2>\"$3\"; echo $? >\"$4.next\" && mv \"$4.next\" \"$4\"; \
                 exec sleep 600",
                "session",
                &session_start_command(),
            ])
            .arg(&out)
            .arg(&err)
            .arg(&status)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("sh runs");
        let _ = process
            .stdin
            .take()
            .expect("stdin")
            .write_all(&payload(case, session_id));
        let started = Instant::now();
        while !status.exists() {
            if started.elapsed() >= HOOK_DEADLINE {
                let _ = process.kill();
                let _ = process.wait();
                panic!(
                    "the hook of {name} did not end within {} s",
                    HOOK_DEADLINE.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
        let read = |path: &Path| fs::read_to_string(path).expect("read what the hook wrote");
        Self {
            status: read(&status).trim().parse().ok(),
            stdout: read(&out),
            stderr: read(&err),
            process,
        }
    }

    fn pid(&self) -> u32 {
        self.process.id()
    }

    /// Ends the session's process and reaps it, so its pid names no process any more.
    fn end(&mut self) {
        self.process.kill().expect("end the session's process");
        self.process.wait().expect("reap the session's process");
    }
}

impl Drop for ChildSession {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// The rows of `conductor session leases --format json` over the case's state directory.
fn leases(case: &Path) -> Vec<Value> {
    let output = common::conductor(case.join("home"))
        .current_dir(case)
        .args(["session", "leases", "--format", "json", "--state-dir"])
        .arg(case.join("state"))
        .output()
        .expect("conductor runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    match serde_json::from_slice(&output.stdout).expect("the view prints JSON") {
        Value::Array(rows) => rows,
        other => panic!("the view prints an array: {other}"),
    }
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The acceptance of `story:single-conductor-lease`: of two sessions whose processes live, the
/// first to run the shipped SessionStart command takes the lease and the second is refused,
/// naming the holder's session id and pid; once the holder's process is gone, the next claim
/// takes the lease over and the lease records whom from.
#[test]
fn a_second_live_conductor_is_refused_and_a_gone_one_is_taken_over() {
    let case = case_dir("second-conductor");
    let this = std::process::id();

    let mut first = ChildSession::start(&case, "first", "session-one");
    assert_eq!(
        first.status,
        Some(0),
        "the first session takes the lease\nstdout: {}\nstderr: {}",
        first.stdout,
        first.stderr
    );
    let holder = first.pid();
    assert_ne!(holder, this);

    let second = claim_from_this_process(&case, "session-two");
    assert_ne!(
        second.status.code(),
        Some(0),
        "a second live session is refused: {}",
        shown(&second)
    );
    let refusal = String::from_utf8_lossy(&second.stderr);
    let holder_pid = holder.to_string();
    for part in [
        "LeaseHeld",
        "session-one",
        holder_pid.as_str(),
        "second conductor",
    ] {
        assert!(
            refusal.contains(part),
            "the refusal names {part}: {}",
            shown(&second)
        );
    }
    let rows = leases(&case);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["session_id"], "session-one", "{rows:#?}");
    assert_eq!(rows[0]["pid"], json!(holder), "{rows:#?}");
    assert_eq!(rows[0]["taken_over_session_id"], Value::Null, "{rows:#?}");

    first.end();
    let third = claim_from_this_process(&case, "session-three");
    assert_eq!(
        third.status.code(),
        Some(0),
        "a claim over a gone holder takes the lease: {}",
        shown(&third)
    );
    let taken = String::from_utf8_lossy(&third.stdout);
    assert!(
        taken.contains("session-one") && taken.contains(&holder.to_string()),
        "the takeover names whom it took the lease from: {}",
        shown(&third)
    );
    let rows = leases(&case);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let lease = &rows[0];
    assert_eq!(lease["state"], "Held", "{rows:#?}");
    assert_eq!(lease["session_id"], "session-three", "{rows:#?}");
    assert_eq!(lease["pid"], json!(this), "{rows:#?}");
    assert_eq!(lease["taken_over_session_id"], "session-one", "{rows:#?}");
    assert_eq!(lease["taken_over_pid"], json!(holder), "{rows:#?}");
}

/// The holder's own process claims again, as a cleared or compacted session does under a new
/// session id: the claim succeeds, the lease names the new id, and the takeover it records stays.
#[test]
fn the_holder_itself_claims_again() {
    let case = case_dir("holder-again");
    let this = std::process::id();

    let mut gone = ChildSession::start(&case, "gone", "session-gone");
    assert_eq!(gone.status, Some(0), "{}", gone.stderr);
    let gone_pid = gone.pid();
    gone.end();

    let taken = claim_from_this_process(&case, "session-one");
    assert_eq!(taken.status.code(), Some(0), "{}", shown(&taken));
    let again = claim_from_this_process(&case, "session-one-cleared");
    assert_eq!(again.status.code(), Some(0), "{}", shown(&again));

    let rows = leases(&case);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["session_id"], "session-one-cleared", "{rows:#?}");
    assert_eq!(rows[0]["pid"], json!(this), "{rows:#?}");
    assert_eq!(
        rows[0]["taken_over_session_id"], "session-gone",
        "{rows:#?}"
    );
    assert_eq!(rows[0]["taken_over_pid"], json!(gone_pid), "{rows:#?}");
}

/// The hook takes the session's id from the payload, so a payload without one is refused rather
/// than claimed under an empty id.
#[test]
fn a_payload_without_a_session_id_is_refused() {
    let case = case_dir("no-session-id");
    let mut child = sh(&case)
        .args(["-c", &session_start_command()])
        .spawn()
        .expect("sh runs");
    let _ = child
        .stdin
        .take()
        .expect("stdin")
        .write_all(br#"{"hook_event_name": "SessionStart", "source": "startup"}"#);
    let output = child.wait_with_output().expect("sh ends");
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("session_id"),
        "{}",
        shown(&output)
    );
}
