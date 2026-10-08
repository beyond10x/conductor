//! `story:guard-hook`: the rule table of `conductor guard`, decided by [`guard::decide`] over the
//! PreToolUse payloads a live Claude Code 2.1.292 session sent (`tests/fixtures/guard/`).
//!
//! The fixtures were recorded by a probe hook that logged each payload and denied the call; their
//! session, prompt and tool-use ids are replaced by synthetic ones of the same shape. The
//! home directory in them is written `~`, and the project directory in `transcript_path` as
//! `-HOME-`; each case puts the payload under a home of its own in this test target's temporary
//! directory, so every `~/` value names a path under it. A case changes only `cwd` and the
//! `tool_input` fields its rule reads.
//!
//! Without a config file the guard decides by the built-in places, and these cases are written in
//! them: the repository is the first path segment of the cwd under `~/example-org/` (the checkout)
//! or under `~/.local/state/worktree/trees/example-org/` (its managed worktrees, where a controller's
//! workers run). Repository `conductor` is decided by conductor's rule, any other by the controller
//! rule, and a cwd under neither is denied for every tool that has a rule. The cases at the end
//! decide the same rules in the places a config file names.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use conductor_cli::config;
use conductor_cli::guard::{self, Decision, Places, SessionList};
use conductor_model::dispatch::Verdict;
use serde_json::{Map, Value};

/// The repository the probe session ran in.
const PROBE: &str = "~/example-org/.guard-probe-w04";

/// Conductor's checkout.
const CONDUCTOR: &str = "~/example-org/conductor";

/// A home directory of its own for one case.
fn home(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_rules")
        .join(case)
        .join("home");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's home");
    }
    fs::create_dir_all(&dir).expect("create the case's home");
    dir
}

/// A recorded payload, by its file name under `tests/fixtures/guard/`.
fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/guard")
        .join(format!("{name}.json"));
    let text = fs::read_to_string(&path).expect("read the recorded payload");
    serde_json::from_str(&text).expect("the recorded payload is JSON")
}

/// `value` with a leading `~/` written as `home`.
fn under(home: &Path, value: &str) -> String {
    match value.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home.display()),
        None => value.to_owned(),
    }
}

/// Every string in `value` that starts with `~/`, written under `home`.
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

/// The recorded payload `name`, under `home`, run from `cwd`, with each `(field, value)` of `set`
/// replacing that `tool_input` field. Values starting with `~/` are written under `home`.
fn payload(name: &str, home: &Path, cwd: &str, set: &[(&str, &str)]) -> Value {
    let mut payload = localise(fixture(name), home);
    payload["cwd"] = Value::String(under(home, cwd));
    for (field, value) in set {
        payload["tool_input"][*field] = Value::String(under(home, value));
    }
    payload
}

/// The decision on the recorded payload `name` from `cwd`, with `set` applied.
fn decide(name: &str, home: &Path, cwd: &str, set: &[(&str, &str)]) -> Decision {
    guard::decide(&payload(name, home, cwd, set), home)
}

/// Asserts `decision` is `verdict`, and that its reason names `because`.
#[track_caller]
fn assert_verdict(decision: &Decision, verdict: Verdict, because: &str) {
    assert_eq!(
        decision.verdict, verdict,
        "expected {verdict:?} ({because}), got {decision:#?}"
    );
    assert!(
        decision.reason.contains(because),
        "the reason {:?} does not name {because:?}",
        decision.reason
    );
}

// -------------------------------------------------------------------------------------------------
// The recorded payloads, as recorded.
// -------------------------------------------------------------------------------------------------

#[test]
fn the_recorded_payloads_are_decided_with_the_fields_the_record_keeps() {
    let home = home("recorded");
    let session = "00000000-0000-4000-8000-0000000000a1";
    let cases = [
        (
            "send-message",
            "SendMessage",
            "guard-probe-nobody-w04".to_owned(),
            Verdict::Deny,
        ),
        (
            "write",
            "Write",
            under(&home, "~/example-org/.guard-probe-w04/probe-write.txt"),
            Verdict::Allow,
        ),
        (
            "edit",
            "Edit",
            under(&home, "~/example-org/.guard-probe-w04/a.txt"),
            Verdict::Allow,
        ),
        (
            "notebook-edit",
            "NotebookEdit",
            under(&home, "~/example-org/.guard-probe-w04/n.ipynb"),
            Verdict::Allow,
        ),
        (
            "bash",
            "Bash",
            "git -C ../conductor status".to_owned(),
            Verdict::Deny,
        ),
    ];
    for (name, tool, target, verdict) in cases {
        let decision = guard::decide(&localise(fixture(name), &home), &home);
        assert_eq!(
            decision.repository, ".guard-probe-w04",
            "{name}: {decision:#?}"
        );
        assert_eq!(decision.session_ref, session, "{name}: {decision:#?}");
        assert_eq!(decision.tool, tool, "{name}: {decision:#?}");
        assert_eq!(decision.target, target, "{name}: {decision:#?}");
        assert_eq!(decision.verdict, verdict, "{name}: {decision:#?}");
        assert!(
            !decision.reason.is_empty(),
            "{name}: every verdict has a reason"
        );
    }
}

// -------------------------------------------------------------------------------------------------
// Controller rule, row 1: Edit, Write and NotebookEdit stay in the repository's checkout and its
// managed worktrees.
// -------------------------------------------------------------------------------------------------

/// The field each file tool names its target in, and its recorded payload.
const FILE_TOOLS: [(&str, &str); 3] = [
    ("write", "file_path"),
    ("edit", "file_path"),
    ("notebook-edit", "notebook_path"),
];

#[test]
fn a_controller_writes_in_its_checkout_and_its_managed_worktrees() {
    let home = home("controller-file-allow");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/.guard-probe-w04/src/new.rs",
            "~/example-org/.guard-probe-w04/./docs/../README.md",
            "~/.local/state/worktree/trees/example-org/.guard-probe-w04/probe-w04-u1/src/lib.rs",
            "relative/inside.txt",
        ] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "");
            assert_eq!(decision.repository, ".guard-probe-w04");
        }
    }
}

#[test]
fn a_controller_writes_nowhere_else() {
    let home = home("controller-file-deny");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/conductor/x",
            "~/example-org/.guard-probe-w04/../conductor/x",
            "~/example-org/.guard-probe-w04-other/x",
            "~/example-org/x",
            "~/.cache/x",
            "~/.claude/settings.json",
            "~/.local/state/worktree/trees/example-org/conductor/conductor-w04-u1/x",
            "~/.local/state/worktree/trees/example-org/x",
            "../conductor/x",
            "/etc/x",
        ] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "outside");
        }
    }
}

#[test]
fn a_tilde_target_is_the_home_directory_not_a_directory_named_tilde() {
    let home = home("controller-file-tilde");
    let decision = decide(
        "write",
        &home,
        PROBE,
        &[("file_path", "~/example-org/conductor/x")],
    );
    assert_verdict(&decision, Verdict::Deny, "outside");
    // The fixture's `~/` is localised by the case; a literal `~/` the session sends is the home.
    let mut raw = payload("write", &home, PROBE, &[]);
    raw["tool_input"]["file_path"] = Value::String("~/example-org/conductor/x".to_owned());
    assert_verdict(&guard::decide(&raw, &home), Verdict::Deny, "outside");
}

#[test]
fn a_symlink_in_the_checkout_does_not_carry_a_write_out_of_it() {
    let home = home("controller-file-symlink");
    let checkout = home.join("example-org/.guard-probe-w04");
    let conductor = home.join("example-org/conductor");
    fs::create_dir_all(&checkout).expect("create the checkout");
    fs::create_dir_all(&conductor).expect("create conductor's checkout");
    std::os::unix::fs::symlink(&conductor, checkout.join("link")).expect("plant the link");
    let decision = decide(
        "write",
        &home,
        PROBE,
        &[("file_path", "~/example-org/.guard-probe-w04/link/x")],
    );
    assert_verdict(&decision, Verdict::Deny, "outside");
}

#[test]
fn a_file_tool_call_without_a_target_is_denied() {
    let home = home("controller-file-missing");
    for (name, field) in FILE_TOOLS {
        let mut raw = payload(name, &home, PROBE, &[]);
        raw["tool_input"]
            .as_object_mut()
            .expect("tool_input is an object")
            .remove(field);
        assert_verdict(&guard::decide(&raw, &home), Verdict::Deny, field);
    }
}

// -------------------------------------------------------------------------------------------------
// Controller rule, row 2: SendMessage goes only to conductor.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_controller_messages_conductor() {
    let home = home("controller-message-allow");
    let decision = decide(
        "send-message",
        &home,
        PROBE,
        &[("to", "conductor"), ("recipient", "conductor")],
    );
    assert_verdict(&decision, Verdict::Allow, "conductor");
    assert_eq!(decision.target, "conductor");
}

#[test]
fn a_controller_messages_an_agent_it_started_by_its_id() {
    let home = home("controller-message-own-agent");
    let decision = decide(
        "send-message",
        &home,
        PROBE,
        &[
            ("to", "a0123456789abcdef"),
            ("recipient", "a0123456789abcdef"),
        ],
    );
    assert_verdict(&decision, Verdict::Allow, "an agent this session started");
    for set in [
        vec![
            ("to", "a0123456789abcde"),
            ("recipient", "a0123456789abcde"),
        ],
        vec![
            ("to", "a0123456789abcdefx"),
            ("recipient", "a0123456789abcdefx"),
        ],
        vec![
            ("to", "A0123456789abcdef"),
            ("recipient", "A0123456789abcdef"),
        ],
        vec![
            ("to", "ag123456789abcdef"),
            ("recipient", "ag123456789abcdef"),
        ],
        vec![("to", "a0123456789abcdef"), ("recipient", "delta")],
    ] {
        let decision = decide("send-message", &home, PROBE, &set);
        assert_verdict(&decision, Verdict::Deny, "only conductor");
    }
}

#[test]
fn a_controller_messages_no_one_else() {
    let home = home("controller-message-deny");
    let recorded = decide("send-message", &home, PROBE, &[]);
    assert_verdict(&recorded, Verdict::Deny, "guard-probe-nobody-w04");
    for set in [
        vec![("to", "conductor-dev"), ("recipient", "conductor-dev")],
        vec![("to", "Conductor"), ("recipient", "Conductor")],
        vec![("to", "conductor"), ("recipient", "delta")],
        vec![("to", "*"), ("recipient", "*")],
    ] {
        let decision = decide("send-message", &home, PROBE, &set);
        assert_verdict(&decision, Verdict::Deny, "only conductor");
    }
    let mut raw = payload("send-message", &home, PROBE, &[]);
    raw["tool_input"]
        .as_object_mut()
        .expect("tool_input is an object")
        .remove("to");
    assert_verdict(&guard::decide(&raw, &home), Verdict::Deny, "only conductor");
}

// -------------------------------------------------------------------------------------------------
// `story:guard-conductor-ref`: a controller reaches conductor by its ref or socket address too.
// When two sessions are named `conductor`, SendMessage refuses the bare name as ambiguous, and a
// controller addresses one as `conductor [<ref>]` or by its socket address
// `uds:<dir>/<pid>.sock`. The harness sends `recipient` equal to `to` in each (`recipient_kind`
// `name` and `session`). The refs, pids and socket directory here are synthetic.
// -------------------------------------------------------------------------------------------------

/// The session list `tests/fixtures/guard/session-list.json`, printed by `cat`: conductor live as
/// pids 4100001 and 4100002 and stopped once (no pid), connectors as 4100011, conductor-dev as
/// 4100012, and an unnamed session as 4100013.
fn listed() -> SessionList {
    let list = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/guard/session-list.json");
    SessionList {
        command: vec![OsString::from("cat"), list.into_os_string()],
        bound: Duration::from_secs(5),
    }
}

/// A session list whose command is no program: a decision that reads it gets an error.
fn unlisted() -> SessionList {
    SessionList {
        command: vec![OsString::from("conductor-guard-test-no-such-session-list")],
        bound: Duration::from_secs(5),
    }
}

/// The decision on a SendMessage to `to` from `cwd`, with `recipient` (absent when `None`), the
/// session list read from `sessions`.
fn message(
    home: &Path,
    cwd: &str,
    sessions: &SessionList,
    to: &str,
    recipient: Option<&str>,
) -> Decision {
    let mut raw = payload("send-message", home, cwd, &[("to", to)]);
    let input = raw["tool_input"]
        .as_object_mut()
        .expect("tool_input is an object");
    match recipient {
        Some(recipient) => {
            input.insert("recipient".to_owned(), Value::String(recipient.to_owned()));
        }
        None => {
            input.remove("recipient");
        }
    }
    guard::decide_with(&raw, home, sessions)
}

/// Decision 1: `conductor [<ref>]`, the ref being one or more lower-case hexadecimal characters,
/// is allowed without a lookup, so a session list that cannot be read changes nothing.
/// `recipient`, when present, names the same session in either form.
#[test]
fn a_controller_messages_conductor_by_the_ref_the_session_list_prints() {
    let home = home("controller-message-ref-allow");
    for cwd in [PROBE, WORKER] {
        for (to, recipient) in [
            ("conductor [c0d0a1]", Some("conductor [c0d0a1]")),
            ("conductor [c0d0b2]", Some("conductor [c0d0b2]")),
            ("conductor [c0d0a1]", None),
            ("conductor [c0d0a1]", Some("conductor")),
            ("conductor", Some("conductor [c0d0a1]")),
            ("conductor [0]", Some("conductor [0]")),
            ("conductor [0123456789abcdef]", None),
            ("conductor", Some("conductor")),
            ("conductor", None),
        ] {
            let decision = message(&home, cwd, &unlisted(), to, recipient);
            assert_verdict(&decision, Verdict::Allow, "conductor");
            assert_eq!(decision.target, to);
        }
    }
}

#[test]
fn a_ref_on_another_name_or_one_that_is_not_lower_case_hexadecimal_is_denied() {
    let home = home("controller-message-ref-deny");
    for (to, recipient) in [
        ("delta [c0d0a1]", Some("delta [c0d0a1]")),
        ("conductor-dev [c0d0a1]", Some("conductor-dev [c0d0a1]")),
        ("Conductor [c0d0a1]", Some("Conductor [c0d0a1]")),
        ("conductor [zz]", Some("conductor [zz]")),
        ("conductor [C0D0A1]", Some("conductor [C0D0A1]")),
        ("conductor []", Some("conductor []")),
        ("conductor [c0d0a1", Some("conductor [c0d0a1")),
        ("conductor [c0d0a1] ", Some("conductor [c0d0a1] ")),
        ("conductor  [c0d0a1]", Some("conductor  [c0d0a1]")),
        ("conductor[c0d0a1]", Some("conductor[c0d0a1]")),
        ("conductor [c0 0a1]", Some("conductor [c0 0a1]")),
        ("conductor [c0d0a1] [c0d0b2]", None),
        ("conductor [c0d0a1]", Some("conductor [c0d0b2]")),
        ("conductor [c0d0a1]", Some("delta")),
        ("conductor [c0d0a1]", Some("delta [c0d0a1]")),
        ("conductor", Some("conductor [zz]")),
        ("delta", Some("conductor [c0d0a1]")),
    ] {
        let decision = message(&home, PROBE, &unlisted(), to, recipient);
        assert_verdict(&decision, Verdict::Deny, "only conductor");
        assert!(
            !decision.reason.contains("session list"),
            "{to:?}: a name is decided without the session list: {}",
            decision.reason
        );
    }
}

/// Decision 2: a socket address `uds:<path>/<pid>.sock` is allowed when the session list holds a
/// live entry (`pid` present) named `conductor` with that pid; a pid of another session, of a
/// stopped session or of none is denied, naming the address.
#[test]
fn a_controller_messages_conductor_s_socket_by_the_pid_the_session_list_gives_it() {
    let home = home("controller-message-socket");
    for cwd in [PROBE, WORKER] {
        for (to, recipient) in [
            (
                "uds:/run/user/4242/cc-socks/4100001.sock",
                Some("uds:/run/user/4242/cc-socks/4100001.sock"),
            ),
            ("uds:/run/user/4242/cc-socks/4100002.sock", None),
        ] {
            let decision = message(&home, cwd, &listed(), to, recipient);
            assert_verdict(&decision, Verdict::Allow, "named conductor");
            assert_eq!(decision.target, to);
        }
        for (to, whose) in [
            ("uds:/run/user/4242/cc-socks/4100011.sock", "connectors"),
            ("uds:/run/user/4242/cc-socks/4100012.sock", "conductor-dev"),
            (
                "uds:/run/user/4242/cc-socks/4100013.sock",
                "an unnamed session",
            ),
            ("uds:/run/user/4242/cc-socks/4100099.sock", "no session"),
        ] {
            let decision = message(&home, cwd, &listed(), to, Some(to));
            assert_verdict(&decision, Verdict::Deny, "only conductor");
            assert!(
                decision.reason.contains(to),
                "{to:?} ({whose}): the denial names the address: {}",
                decision.reason
            );
        }
    }
    for recipient in [
        "uds:/run/user/4242/cc-socks/4100011.sock",
        "uds:/run/user/4242/cc-socks/4100002.sock",
        "delta",
    ] {
        let decision = message(
            &home,
            PROBE,
            &listed(),
            "uds:/run/user/4242/cc-socks/4100001.sock",
            Some(recipient),
        );
        assert_verdict(&decision, Verdict::Deny, "only conductor");
    }
}

/// Story, "What changes" 2: any other socket address is denied, naming it.
#[test]
fn a_socket_address_not_of_the_form_uds_path_pid_sock_is_denied_naming_it() {
    let home = home("controller-message-socket-form");
    for to in [
        "uds:/run/user/4242/cc-socks/conductor.sock",
        "uds:/run/user/4242/cc-socks/4100001",
        "uds:/run/user/4242/cc-socks/4100001.sock.bak",
        "uds:/run/user/4242/cc-socks/04100001.sock",
        "uds:/run/user/4242/cc-socks/+4100001.sock",
        "uds:/run/user/4242/cc-socks/.sock",
        "uds:4100001.sock",
        "uds:cc-socks/4100001.sock",
        "uds:",
        "UDS:/run/user/4242/cc-socks/4100001.sock",
        "tcp:127.0.0.1:4100001",
    ] {
        let decision = message(&home, PROBE, &listed(), to, Some(to));
        assert_verdict(&decision, Verdict::Deny, "only conductor");
        assert!(
            decision.reason.contains(to),
            "the denial names {to:?}: {}",
            decision.reason
        );
    }
}

/// Decision 2: a session list that does not start, fails, times out or does not parse denies the
/// socket, naming why.
#[test]
fn a_session_list_that_cannot_be_read_denies_the_socket_naming_why() {
    let home = home("controller-message-socket-unread");
    let socket = "uds:/run/user/4242/cc-socks/4100001.sock";
    let not_an_array =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/guard/send-message.json");
    let words = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
    for (sessions, why) in [
        (unlisted(), "conductor-guard-test-no-such-session-list"),
        (
            SessionList {
                command: words(&["sh", "-c", "exit 3"]),
                bound: Duration::from_secs(5),
            },
            "exited 3",
        ),
        (
            SessionList {
                command: words(&["echo", "not json"]),
                bound: Duration::from_secs(5),
            },
            "no JSON",
        ),
        (
            SessionList {
                command: vec![OsString::from("cat"), not_an_array.into_os_string()],
                bound: Duration::from_secs(5),
            },
            "no JSON array",
        ),
        (
            SessionList {
                command: Vec::new(),
                bound: Duration::from_secs(5),
            },
            "no session-list command",
        ),
    ] {
        let decision = message(&home, PROBE, &sessions, socket, Some(socket));
        assert_verdict(&decision, Verdict::Deny, why);
        assert!(
            decision.reason.contains("session list") && decision.reason.contains(socket),
            "{why}: {}",
            decision.reason
        );
    }
    let started = Instant::now();
    let decision = message(
        &home,
        PROBE,
        &SessionList {
            command: words(&["sleep", "30"]),
            bound: Duration::from_secs(1),
        },
        socket,
        Some(socket),
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the decision waited {:?}",
        started.elapsed()
    );
    assert_verdict(&decision, Verdict::Deny, "no answer within 1 s");
}

/// A socket address is compared where it lands, as every path the guard reads is: a link named
/// for conductor's pid that lands on another session's socket is that session's.
#[test]
fn a_socket_address_is_matched_by_where_it_lands() {
    let home = home("controller-message-socket-link");
    let socks = home.join("socks");
    fs::create_dir_all(&socks).expect("create a socket directory");
    std::os::unix::fs::symlink(socks.join("4100011.sock"), socks.join("4100001.sock"))
        .expect("link conductor's pid to connectors' socket");
    std::os::unix::fs::symlink(socks.join("4100002.sock"), socks.join("1.sock"))
        .expect("link another pid to conductor's socket");
    let to = format!("uds:{}/4100001.sock", socks.display());
    assert_verdict(
        &message(&home, PROBE, &listed(), &to, Some(&to)),
        Verdict::Deny,
        "only conductor",
    );
    let to = format!("uds:{}/1.sock", socks.display());
    assert_verdict(
        &message(&home, PROBE, &listed(), &to, Some(&to)),
        Verdict::Allow,
        "named conductor",
    );
}

/// Decision 3: conductor's own SendMessage stays unrestricted, and reads no session list.
#[test]
fn conductor_messages_any_socket_without_reading_the_session_list() {
    let home = home("conductor-message-socket");
    for to in [
        "uds:/run/user/4242/cc-socks/4100011.sock",
        "uds:/run/user/4242/cc-socks/x.sock",
        "delta [c0d0a1]",
    ] {
        assert_verdict(
            &message(&home, CONDUCTOR, &unlisted(), to, Some(to)),
            Verdict::Allow,
            "not restricted",
        );
    }
}

/// Decision 2: the guard reads `claude agents --json`, bounded at 5 s; a test replaces both.
#[test]
fn the_session_list_is_claude_agents_json_bounded_at_five_seconds() {
    let sessions = SessionList::default();
    assert_eq!(sessions.command, ["claude", "agents", "--json"]);
    assert_eq!(sessions.bound, Duration::from_secs(5));
}

// -------------------------------------------------------------------------------------------------
// Controller rule, row 3: Bash does not reach into another repository, and makes no GitHub write.
// -------------------------------------------------------------------------------------------------

fn bash(home: &Path, cwd: &str, command: &str) -> Decision {
    decide("bash", home, cwd, &[("command", command)])
}

#[test]
fn a_controller_runs_git_and_cd_in_its_own_checkout() {
    let home = home("controller-bash-allow");
    let own = format!("{}/example-org/.guard-probe-w04", home.display());
    for command in [
        "git status",
        "git -C . status",
        "git -C src log --oneline -3",
        &format!("git -C {own} status"),
        "git -C ~/example-org/.guard-probe-w04 status",
        "cd src && cargo test",
        "cd ~/example-org/.guard-probe-w04/crates && ls",
        "cd ~/.local/state/worktree/trees/example-org/.guard-probe-w04/t && git status",
        "cd ~/example-org && ls",
        "echo 'cd ~/example-org/conductor'",
        "git commit -m \"see git -C ~/example-org/conductor\"",
        "git log --format='%H' -- ../conductor",
        "rg -n 'cd ../conductor' docs",
    ] {
        assert_verdict(&bash(&home, PROBE, command), Verdict::Allow, "");
    }
}

#[test]
fn a_controller_does_not_cd_or_git_dash_c_into_another_repository() {
    let home = home("controller-bash-deny-repo");
    let other = format!("{}/example-org/conductor", home.display());
    for command in [
        "git -C ../conductor status",
        "git -C ~/example-org/conductor log",
        "git -C $HOME/example-org/conductor log",
        "git -C ${HOME}/example-org/conductor log",
        "git -C \"$HOME/example-org/conductor\" log",
        &format!("git -C {other} status"),
        &format!("git -C {other}/crates status"),
        "git -c core.pager=cat -C ../conductor log",
        "git -C .. -C conductor status",
        "git --git-dir=../conductor/.git log",
        "git --work-tree ../conductor status",
        "cd ../conductor",
        "cd ~/example-org/conductor && git status",
        "cd ~/example-org/conductor/crates",
        "cd .. && cd conductor && git push",
        "cd ..; cd conductor",
        "pushd ../conductor",
        "ls && cd ~/example-org/delta",
        "(cd ../conductor && git log)",
        "echo $(cd ../conductor; pwd)",
        "bash -c 'cd ../conductor && git status'",
        "FOO=1 git -C ../conductor status",
        "/usr/bin/git -C ../conductor status",
        "if true; then cd ../conductor; fi",
        "cd ~/.local/state/worktree/trees/example-org/conductor/conductor-w04-u1",
        "git -C ~/.local/state/worktree/trees/example-org/conductor/conductor-w04-u1 status",
    ] {
        let decision = bash(&home, PROBE, command);
        assert_verdict(&decision, Verdict::Deny, "another repository");
    }
}

/// The `gh` verbs the rule names, each denied for controllers and conductor alike.
const GH_WRITES: [&str; 12] = [
    "gh pr create --title t --body b",
    "gh pr comment 12 --body hi",
    "gh pr merge 12 --squash",
    "gh issue create --title t --body b",
    "gh release create v1.0.0",
    "gh release edit v1.0.0 --draft=false",
    "gh run rerun 123",
    "gh api repos/o/r/issues --method POST -f title=t",
    "gh api repos/o/r/pulls/1 -X PATCH",
    "gh api --method=DELETE repos/o/r/git/refs/heads/x",
    "gh api repos/o/r/issues -f title=t",
    "gh -R o/r pr merge 12",
];

/// `gh` calls that read.
const GH_READS: [&str; 10] = [
    "gh pr list --json number",
    "gh pr view 12",
    "gh issue list",
    "gh run list --branch main",
    "gh api repos/o/r/pulls",
    "gh api repos/o/r/pulls --method GET",
    "gh api -X get repos/o/r",
    "gh release list",
    "gh release download v0.1.15 -R o/gates -p *.tar.gz -D out",
    "gh run download 123 -n artifact -D out",
];

#[test]
fn a_controller_makes_no_github_write() {
    let home = home("controller-bash-deny-gh");
    for command in GH_WRITES.iter().copied().chain([
        "git push && gh pr create --fill",
        "bash -c \"gh pr merge 3\"",
        "env GH_REPO=o/r gh issue create -t x -b y",
        "cat body.md | gh pr comment 4 -F -",
        "gh api graphql -f query='mutation { addComment(input: {}) { clientMutationId } }'",
    ]) {
        assert_verdict(&bash(&home, PROBE, command), Verdict::Deny, "GitHub write");
    }
}

#[test]
fn a_controller_reads_github() {
    let home = home("controller-bash-allow-gh");
    for command in GH_READS.iter().copied().chain([
        "gh api graphql -f query='query { viewer { login } }'",
        "echo 'gh pr create'",
        "git commit -F - <<'MSG'\nNext: gh pr create, gh pr merge\nMSG",
        "cat <<EOF\ngh release create v9\nEOF\ngit status",
        "# gh pr merge 3\ngit status",
    ]) {
        assert_verdict(&bash(&home, PROBE, command), Verdict::Allow, "");
    }
}

// -------------------------------------------------------------------------------------------------
// Conductor's rule.
// -------------------------------------------------------------------------------------------------

#[test]
fn conductor_writes_its_records() {
    let home = home("conductor-file-allow");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/conductor/decisions/2026-10.jsonl",
            "~/example-org/conductor/dispatches/2026-10.jsonl",
            "~/example-org/conductor/charters/delta.md",
            "~/example-org/conductor/goals.jsonl",
            "~/example-org/conductor/STATUS.md",
            "~/example-org/conductor/NORTHSTAR.md",
            "~/example-org/conductor/docs/handoff/2026-10-07.md",
            "decisions/x",
            "docs/handoff/x.md",
        ] {
            let decision = decide(name, &home, CONDUCTOR, &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "record");
            assert_eq!(decision.repository, "conductor");
        }
    }
}

#[test]
fn conductor_writes_nothing_else() {
    let home = home("conductor-file-deny");
    for (name, field) in FILE_TOOLS {
        for target in [
            "crates/x",
            "~/example-org/conductor/crates/conductor/src/guard.rs",
            "~/example-org/conductor/.claude/conductor-settings.json",
            "~/example-org/conductor/AGENTS.md",
            "~/example-org/conductor/docs/design/conductor.md",
            "~/example-org/conductor/STATUS.md.bak",
            "~/example-org/conductor/goals.jsonl/x",
            "~/example-org/conductor/decisions/../crates/x",
            "~/example-org/conductor/state/store/events.jsonl",
            "~/example-org/delta/x",
            "~/.cache/conductor-analysis/x",
            "~/.local/state/worktree/trees/example-org/conductor/t/decisions/x",
            "~/.claude/agents/x.md",
        ] {
            let decision = decide(name, &home, CONDUCTOR, &[(field, target)]);
            assert_verdict(
                &decision,
                Verdict::Deny,
                "conductor writes only its records",
            );
        }
    }
}

#[test]
fn conductor_messages_anyone() {
    let home = home("conductor-message");
    for to in ["delta", "guard-probe-nobody-w04", "conductor-dev"] {
        let decision = decide(
            "send-message",
            &home,
            CONDUCTOR,
            &[("to", to), ("recipient", to)],
        );
        assert_verdict(&decision, Verdict::Allow, "");
    }
}

#[test]
fn conductor_reads_other_repositories_but_makes_no_github_write() {
    let home = home("conductor-bash");
    for command in [
        "git -C ~/example-org/delta log --oneline -5",
        "cd ~/example-org/delta && git status",
        "gh pr list -R example-org/delta",
    ] {
        assert_verdict(&bash(&home, CONDUCTOR, command), Verdict::Allow, "");
    }
    for command in GH_WRITES {
        assert_verdict(
            &bash(&home, CONDUCTOR, command),
            Verdict::Deny,
            "GitHub write",
        );
    }
}

// -------------------------------------------------------------------------------------------------
// A worker in a managed worktree: the tree belongs to its repository (correction 1).
// -------------------------------------------------------------------------------------------------

/// A worker of the probe repository's controller, in one of its managed trees.
const WORKER: &str =
    "~/.local/state/worktree/trees/example-org/.guard-probe-w04/guard-probe-w04-u1";

#[test]
fn a_worker_in_a_managed_tree_is_decided_by_its_repository_s_controller_rule() {
    let home = home("worker-file-allow");
    for (name, field) in FILE_TOOLS {
        for target in [
            "src/in-its-own-tree.rs",
            "~/.local/state/worktree/trees/example-org/.guard-probe-w04/guard-probe-w04-u1/src/lib.rs",
            "~/.local/state/worktree/trees/example-org/.guard-probe-w04/guard-probe-w04-u2/src/lib.rs",
            "~/example-org/.guard-probe-w04/README.md",
        ] {
            let decision = decide(name, &home, WORKER, &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "");
            assert_eq!(decision.repository, ".guard-probe-w04", "{decision:#?}");
        }
    }
}

#[test]
fn a_worker_writes_into_no_other_repository() {
    let home = home("worker-file-deny");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/conductor/x",
            "~/.local/state/worktree/trees/example-org/conductor/conductor-w04-u1/x",
            "../../conductor/conductor-w04-u1/x",
            "~/.local/state/worktree/trees/example-org/x",
            "~/.cache/x",
        ] {
            let decision = decide(name, &home, WORKER, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "outside");
        }
    }
}

#[test]
fn a_worker_messages_only_conductor_and_reaches_no_other_repository() {
    let home = home("worker-message-bash");
    assert_verdict(
        &decide(
            "send-message",
            &home,
            WORKER,
            &[("to", "conductor"), ("recipient", "conductor")],
        ),
        Verdict::Allow,
        "conductor",
    );
    assert_verdict(
        &decide("send-message", &home, WORKER, &[]),
        Verdict::Deny,
        "only conductor",
    );
    for command in [
        "cargo test -p x",
        "git -C ~/example-org/.guard-probe-w04 log",
        "cd ~/example-org/.guard-probe-w04 && git status",
        "cd ../guard-probe-w04-u2",
    ] {
        assert_verdict(&bash(&home, WORKER, command), Verdict::Allow, "");
    }
    for command in [
        "git -C ~/example-org/conductor status",
        "cd ../../conductor/conductor-w04-u1",
        "cd ~/example-org/delta",
    ] {
        assert_verdict(
            &bash(&home, WORKER, command),
            Verdict::Deny,
            "another repository",
        );
    }
    assert_verdict(
        &bash(&home, WORKER, "gh pr create --fill"),
        Verdict::Deny,
        "GitHub write",
    );
}

#[test]
fn a_session_in_a_conductor_tree_is_decided_by_conductor_s_rule() {
    let home = home("worker-conductor");
    let tree = "~/.local/state/worktree/trees/example-org/conductor/conductor-w04-u11";
    let records = decide(
        "write",
        &home,
        tree,
        &[(
            "file_path",
            "~/example-org/conductor/decisions/2026-10.jsonl",
        )],
    );
    assert_verdict(&records, Verdict::Allow, "record");
    assert_eq!(records.repository, "conductor");
    for target in [
        "crates/x",
        "decisions/x",
        "~/example-org/conductor/crates/x",
    ] {
        assert_verdict(
            &decide("write", &home, tree, &[("file_path", target)]),
            Verdict::Deny,
            "conductor writes only its records",
        );
    }
    assert_verdict(
        &decide("send-message", &home, tree, &[]),
        Verdict::Allow,
        "not restricted",
    );
}

// -------------------------------------------------------------------------------------------------
// Where the session runs.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_session_outside_the_checkouts_root_is_denied_every_tool_with_a_rule() {
    let home = home("outside");
    for cwd in [
        "~/.cache/x",
        "~/example-org",
        "~/",
        "/",
        "~/.local/state/worktree/trees/example-org",
        "~/.local/state/worktree/trees/other/x/t",
        "~/.local/state/worktree",
    ] {
        for (name, field, target) in [
            ("write", "file_path", "~/example-org/conductor/decisions/x"),
            ("edit", "file_path", "x"),
            ("notebook-edit", "notebook_path", "x.ipynb"),
            ("send-message", "to", "conductor"),
            ("bash", "command", "git status"),
        ] {
            let decision = decide(name, &home, cwd, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "outside ~/example-org/");
            assert_eq!(decision.repository, "", "{cwd}: {decision:#?}");
        }
    }
}

#[test]
fn a_tool_without_a_rule_is_allowed() {
    let home = home("no-rule");
    let mut raw = payload("write", &home, PROBE, &[]);
    raw["tool_name"] = Value::String("Read".to_owned());
    raw["tool_input"] = serde_json::json!({ "file_path": under(&home, "~/.ssh/config") });
    assert_verdict(&guard::decide(&raw, &home), Verdict::Allow, "no rule");
}

#[test]
fn a_payload_without_a_tool_or_a_cwd_is_denied() {
    let home = home("malformed");
    let mut no_tool = payload("write", &home, PROBE, &[]);
    no_tool
        .as_object_mut()
        .expect("an object")
        .remove("tool_name");
    assert_verdict(&guard::decide(&no_tool, &home), Verdict::Deny, "tool_name");

    let mut no_cwd = payload("write", &home, PROBE, &[]);
    no_cwd.as_object_mut().expect("an object").remove("cwd");
    assert_verdict(&guard::decide(&no_cwd, &home), Verdict::Deny, "cwd");

    let mut relative = payload("write", &home, PROBE, &[]);
    relative["cwd"] = Value::String("example-org/.guard-probe-w04".to_owned());
    assert_verdict(&guard::decide(&relative, &home), Verdict::Deny, "cwd");

    assert_verdict(
        &guard::decide(&Value::String("not a payload".to_owned()), &home),
        Verdict::Deny,
        "tool_name",
    );
}

// -------------------------------------------------------------------------------------------------
// Correction 2, decision 1: a controller writes no Claude Code project settings, which can carry
// `disableAllHooks` and so switch its guard off.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_controller_writes_no_project_settings_with_a_file_tool() {
    let home = home("settings-file-tools");
    let checkout = home.join("example-org/.guard-probe-w04");
    fs::create_dir_all(checkout.join("cfg")).expect("create a directory to link to");
    std::os::unix::fs::symlink(
        checkout.join(".claude/settings.json"),
        checkout.join("plain.json"),
    )
    .expect("plant a link to the settings");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/.guard-probe-w04/.claude/settings.json",
            "~/example-org/.guard-probe-w04/.claude/settings.local.json",
            "~/example-org/.guard-probe-w04/sub/dir/.claude/settings.json",
            "~/.local/state/worktree/trees/example-org/.guard-probe-w04/t/.claude/settings.local.json",
            ".claude/settings.json",
            ".claude/x/../settings.local.json",
            "plain.json",
        ] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "project settings");
        }
        for target in [
            ".claude/agents/a.md",
            ".claude/settings.json.example",
            "docs/settings.json",
        ] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "");
        }
    }
}

/// A plain substring test on the command (scope change to correction 2): it names
/// `.claude/settings` and contains a write form. No parsing, so `cd .claude && … > settings.json`
/// is not caught.
#[test]
fn a_controller_s_bash_writes_no_project_settings() {
    let home = home("settings-bash");
    for command in [
        "echo '{\"disableAllHooks\": true}' > .claude/settings.local.json",
        "cat x >> .claude/settings.json",
        "jq . x | tee .claude/settings.json",
        "sed -i 's/a/b/' .claude/settings.json",
        "cp /etc/hosts .claude/settings.local.json",
        "mv x .claude/settings.json",
        "rm .claude/settings.json",
        "ln -s /etc/hosts .claude/settings.json",
        "install -m 644 x .claude/settings.json",
        "truncate -s 0 .claude/settings.json",
        "F=.claude/settings.json; echo '{}' > $F",
        "cd ~/.local/state/worktree/trees/example-org/.guard-probe-w04/t && printf x > .claude/settings.json",
        "bash -c 'echo {} > .claude/settings.local.json'",
    ] {
        assert_verdict(
            &bash(&home, PROBE, command),
            Verdict::Deny,
            "project settings",
        );
    }
    for command in [
        "cat .claude/settings.json",
        "jq . .claude/settings.json",
        "echo x > notes.txt",
        "git diff .claude/settings.json",
    ] {
        assert_verdict(&bash(&home, PROBE, command), Verdict::Allow, "");
    }
}

// -------------------------------------------------------------------------------------------------
// Correction 2, decision 2: `gh` reads are allowed, everything else from `gh` is a GitHub write;
// the bot route is left alone.
// -------------------------------------------------------------------------------------------------

#[test]
fn gh_reads_and_the_bot_route_are_allowed() {
    let home = home("gh-allow-list-reads");
    for cwd in [PROBE, CONDUCTOR] {
        for command in [
            "gh pr checks 3",
            "gh pr diff 3",
            "gh pr status",
            "gh issue list --search x",
            "gh run watch 1",
            "gh run view 1 --log",
            "gh workflow view ci.yml",
            "gh api repos/o/r",
            "gh auth status",
            "gh --version",
            "gh help pr",
            "gh search issues --repo o/r x",
            "b10x-gates gh -- pr create --fill",
            "b10x-gates gh -- pr merge 3 --squash",
            "which gh",
            "command -v gh && gh pr list",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Allow, "");
        }
    }
}

#[test]
fn everything_else_from_gh_is_a_github_write() {
    let home = home("gh-allow-list-writes");
    for cwd in [PROBE, CONDUCTOR] {
        for command in [
            "gh",
            "gh pr checkout 3",
            "gh repo clone o/r",
            "gh workflow run ci.yml",
            "gh auth token",
            "gh api -X GET search/issues -f q=x",
            "gh api -iXDELETE repos/o/r/git/refs/heads/x",
            "gh api repos/o/r/issues --input body.json",
            "gh api graphql -F query=@mutation.graphql",
            "gh --repo o/r pr new",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Deny, "GitHub write");
        }
    }
}

/// Wave 06 U7, decision 6 (the guard used to deny `gh secret set --help`): a `gh` call carrying
/// the long flag `--help` anywhere after `gh` is a read, whatever its group and verb. The short
/// `-h` is a read only when `gh` has no group, because `gh auth login -h github.com` takes it as
/// `--hostname` and writes. Controllers and conductor alike.
#[test]
fn gh_long_help_is_a_read_anywhere_and_short_help_only_without_a_group() {
    let home = home("gh-help");
    for cwd in [PROBE, CONDUCTOR] {
        for command in [
            "gh secret set --help",
            "gh --help secret set",
            "gh pr create --help",
            "gh -R o/r pr merge 12 --help",
            "gh api -X POST repos/o/r/issues --help",
            "gh -h",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Allow, "");
        }
        for command in [
            "gh auth login -h github.com",
            "gh secret set -h",
            "gh pr merge 12 -h",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Deny, "GitHub write");
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Wave 06 U7, decision 1: a controller's session runs only conductor's read leaves.
// -------------------------------------------------------------------------------------------------

/// What a leaf of the command tree is, read from its own arguments: a command takes
/// `--input-json`, a view takes `--format` and nothing else, and a command the binding declares
/// over a `local` callable (`decision show`, `dashboard serve`) takes neither shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaf {
    Command,
    View,
    Local,
}

/// Every `(group, leaf, what it is)` of the `conductor` command tree, from the tree itself.
fn leaves() -> Vec<(String, String, Leaf)> {
    use clap::CommandFactory as _;
    let tree = conductor_cli::cli::Cli::command();
    let mut leaves = Vec::new();
    for group in tree.get_subcommands() {
        for leaf in group.get_subcommands() {
            let arguments: Vec<&str> = leaf
                .get_arguments()
                .map(|argument| argument.get_id().as_str())
                .filter(|id| !["help", "state_dir"].contains(id))
                .collect();
            let kind = if arguments.contains(&"input_json") {
                Leaf::Command
            } else if arguments == ["format"] {
                Leaf::View
            } else {
                Leaf::Local
            };
            leaves.push((
                group.get_name().to_owned(),
                leaf.get_name().to_owned(),
                kind,
            ));
        }
    }
    leaves
}

/// The ways a session writes one run of `conductor <group> <leaf>` on a Bash line: the forms
/// below, and each global option of the tree that takes a value, read from `Cli::command()`,
/// written before the group and between the group and the leaf, so that an option added later is
/// decided here without this list changing.
fn runs(group: &str, leaf: &str) -> Vec<String> {
    use clap::CommandFactory as _;
    let mut runs = vec![
        format!("conductor {group} {leaf}"),
        format!("conductor --state-dir ~/example-org/conductor/state {group} {leaf} --format json"),
        format!("cd src && FOO=1 ~/.cargo/bin/conductor {group} --state-dir=state {leaf}"),
        format!("bash -c 'conductor {group} {leaf} --input-json -' < input.json"),
        format!("ls && env timeout 5 conductor {group} {leaf} | head"),
    ];
    let tree = conductor_cli::cli::Cli::command();
    for argument in tree.get_arguments() {
        if let Some(flag) = argument.get_long()
            && argument.is_global_set()
            && argument.get_action().takes_values()
        {
            runs.push(format!("conductor --{flag} some-value {group} {leaf}"));
            runs.push(format!("conductor {group} --{flag} some-value {leaf}"));
        }
    }
    runs
}

/// Decision 1 of the wave 06 U7 brief: in a controller's session, and in a worker in a
/// controller's tree, the guard allows every view of the tree, `decision show` and the hook's own
/// `guard record-guard-decision`, and denies every other leaf, naming it. The tree is read from
/// `Cli::command()`, so a leaf added to it is decided here without this case changing: a view the
/// guard's list misses is denied, and a command it lists is allowed, and either fails.
#[test]
fn a_controller_runs_only_conductors_read_leaves() {
    let home = home("controller-conductor-leaves");
    let leaves = leaves();
    assert!(
        leaves.iter().any(|(_, _, kind)| *kind == Leaf::View)
            && leaves.iter().any(|(_, _, kind)| *kind == Leaf::Command),
        "the tree has views and commands: {leaves:?}"
    );
    let mut problems = Vec::new();
    for cwd in [
        PROBE,
        "~/.local/state/worktree/trees/example-org/.guard-probe-w04/t",
    ] {
        for (group, leaf, kind) in &leaves {
            let reads = *kind == Leaf::View
                || (group.as_str(), leaf.as_str()) == ("decision", "show")
                || (group.as_str(), leaf.as_str()) == ("guard", "record-guard-decision");
            for command in runs(group, leaf) {
                let decision = bash(&home, cwd, &command);
                let expected = if reads { Verdict::Allow } else { Verdict::Deny };
                let named = format!(
                    "`conductor {group} {leaf}` writes conductor's records; only conductor runs it"
                );
                if decision.verdict != expected || (!reads && decision.reason != named) {
                    problems.push(format!(
                        "{cwd}: {command:?} ({kind:?}): expected {expected:?}, got {:?}: {}",
                        decision.verdict, decision.reason
                    ));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} runs decided wrongly:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// Conductor's own session runs every leaf of the tree, the operator's three included (decision
/// 1: it records the operator's decisions on his word).
#[test]
fn conductor_runs_every_leaf() {
    let home = home("conductor-conductor-leaves");
    let mut denied = Vec::new();
    for (group, leaf, _) in leaves() {
        for command in runs(&group, &leaf) {
            let decision = bash(&home, CONDUCTOR, &command);
            if decision.verdict != Verdict::Allow {
                denied.push(format!("{command:?}: {}", decision.reason));
            }
        }
    }
    assert!(
        denied.is_empty(),
        "conductor was denied:\n  - {}",
        denied.join("\n  - ")
    );
}

/// A line that names `conductor` without running one of its leaves is no run of it: help, a
/// lookup, a path, a quoted string, a crate name.
#[test]
fn a_controller_names_conductor_without_running_a_leaf() {
    let home = home("controller-conductor-mentions");
    for command in [
        "conductor --help",
        "conductor decision --help",
        "conductor --state-dir state decision",
        "which conductor",
        "command -v conductor",
        "ls ~/example-org/conductor",
        "ls crates/conductor src",
        "git log -- crates/conductor",
        "rg 'conductor decision record-operator-decision' docs",
        "cargo test -p conductor-cli",
        "echo conductor",
    ] {
        assert_verdict(&bash(&home, PROBE, command), Verdict::Allow, "");
    }
}

// -------------------------------------------------------------------------------------------------
// The instance's places. The guard reads where checkouts, managed worktrees and conductor's records
// are from the instance the process runs (`config::active`): without a config file the built-in
// instance's, today's, and conductor is the session in the repository named `conductor`; with one,
// the file's `checkouts` and `records`, and conductor is the session whose cwd is in `records`.
// -------------------------------------------------------------------------------------------------

/// Where one case's sessions run, and the guard's places for them.
struct Layout {
    /// Names the layout in a failure.
    name: &'static str,
    home: PathBuf,
    /// The checkouts root.
    root: PathBuf,
    /// The managed-worktree root.
    trees: PathBuf,
    /// Conductor's records.
    records: PathBuf,
    /// Conductor's own cwd.
    conductor: PathBuf,
    /// Cwds in no place a rule set applies to.
    outside: Vec<PathBuf>,
    /// The config file the process resolves.
    config: PathBuf,
    places: Places,
}

/// The built-in instance's places under `home`: what the guard decides by without a config file.
fn built_in_layout(home: &Path) -> Layout {
    let instance = config::built_in(home, home);
    let root = PathBuf::from(&instance.checkouts.root);
    let trees = PathBuf::from(&instance.checkouts.trees);
    let conductor = root.join("conductor");
    Layout {
        name: "without a config file",
        home: home.to_owned(),
        config: home.join(config::DEFAULT_FILE),
        outside: vec![
            home.to_owned(),
            root.clone(),
            trees.clone(),
            home.join(".cache/x"),
        ],
        records: conductor.clone(),
        conductor,
        root,
        trees,
        places: Places::built_in(home),
    }
}

/// A config file naming one instance, with `checkouts` and `records` as given.
fn config_text(root: &Path, trees: &Path, records: &Path) -> String {
    format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: fixture\n\
         \x20   sources: [{{github: fixture}}]\n\
         \x20   checkouts: {{root: {}, trees: {}}}\n\
         \x20   records: {}\n",
        root.display(),
        trees.display(),
        records.display()
    )
}

/// The config file [`places_from_file`] writes beside `home`.
fn file_beside(home: &Path) -> PathBuf {
    home.parent()
        .expect("the case's directory")
        .join("conductor.yaml")
}

/// Writes `text` as the config file beside `home`, and answers the guard's places for the instance
/// it names, as the process resolves them when `CONDUCTOR_CONFIG` names the file.
fn places_from_file(home: &Path, text: &str) -> Places {
    places_from(home, &file_beside(home), text)
}

/// Writes `text` as the config file `file`, and answers the guard's places as the hook resolves
/// them when `CONDUCTOR_CONFIG` names `file`: the instance it names, and `file` as the config file.
fn places_from(home: &Path, file: &Path, text: &str) -> Places {
    fs::create_dir_all(file.parent().expect("the file's directory"))
        .expect("create the file's directory");
    fs::write(file, text).expect("write the config file");
    let environment = config::Environment {
        home: Some(home.to_owned()),
        cwd: home.to_owned(),
        config: Some(file.as_os_str().to_owned()),
        instance: None,
    };
    let active = config::resolve(None, &environment).expect("the config file loads");
    assert!(active.from_file, "the instance is the file's");
    Places::active(home, &active).with_config_file(file)
}

/// A config file's places in the case directory beside `home`: checkouts under `root/`, managed
/// worktrees under `trees/`, records in `records/`, and conductor's session in `records/`.
fn file_layout(home: &Path) -> Layout {
    let case = home.parent().expect("the case's directory");
    let (root, trees, records) = (case.join("root"), case.join("trees"), case.join("records"));
    let built_in = built_in_layout(home);
    Layout {
        name: "with a config file",
        home: home.to_owned(),
        outside: vec![
            home.to_owned(),
            case.to_owned(),
            root.clone(),
            trees.clone(),
            built_in.conductor,
            built_in.root.join("probe"),
            built_in.trees.join("probe/t"),
        ],
        places: places_from_file(home, &config_text(&root, &trees, &records)),
        config: file_beside(home),
        conductor: records.clone(),
        root,
        trees,
        records,
    }
}

impl Layout {
    /// `text` with `{home}`, `{root}`, `{trees}`, `{records}` and `{config}` written as this
    /// layout's.
    fn fill(&self, text: &str) -> String {
        text.replace("{home}", &self.home.display().to_string())
            .replace("{root}", &self.root.display().to_string())
            .replace("{trees}", &self.trees.display().to_string())
            .replace("{records}", &self.records.display().to_string())
            .replace("{config}", &self.config.display().to_string())
    }

    /// The decision on the recorded payload `name` from `cwd`, `field` set to `value` (and a
    /// SendMessage's `recipient` to its `to`), by this layout's places.
    fn decide(
        &self,
        name: &str,
        cwd: &Path,
        field: &str,
        value: &str,
        sessions: &SessionList,
    ) -> Decision {
        let mut raw = localise(fixture(name), &self.home);
        raw["cwd"] = Value::String(cwd.display().to_string());
        raw["tool_input"][field] = Value::String(value.to_owned());
        if field == "to" {
            raw["tool_input"]["recipient"] = Value::String(value.to_owned());
        }
        guard::decide_in(&raw, &self.places, sessions)
    }
}

/// Who makes a call in [`every_rule_that_denies_without_a_config_file_denies_with_one`].
#[derive(Debug, Clone, Copy)]
enum Who {
    /// A controller, in `{root}/probe`.
    Controller,
    /// A worker of that controller, in `{trees}/probe/t`.
    Worker,
    /// Conductor, in its records or (without a file) its checkout.
    Conductor,
    /// A session in each of the layout's places no rule set applies to.
    Outside,
}

/// Each rule of the table, as one call that it decides, and an allowance beside the denials of
/// each rule set so that a layout the guard does not read fails here rather than denying all.
/// `{home}`, `{root}`, `{trees}`, `{records}` and `{config}` are the layout's.
const RULES: [(Who, &str, &str, &str, Verdict, &str); 38] = [
    // Controller, Edit, Write and NotebookEdit: its checkout and managed worktrees only, no
    // project settings.
    (
        Who::Controller,
        "write",
        "file_path",
        "{root}/probe/src/lib.rs",
        Verdict::Allow,
        "inside",
    ),
    (
        Who::Controller,
        "edit",
        "file_path",
        "{trees}/probe/t/src/lib.rs",
        Verdict::Allow,
        "inside",
    ),
    (
        Who::Controller,
        "write",
        "file_path",
        "{root}/other/x",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    (
        Who::Controller,
        "notebook-edit",
        "notebook_path",
        "{trees}/other/t/x.ipynb",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    (
        Who::Controller,
        "write",
        "file_path",
        "{home}/.cache/x",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    (
        Who::Controller,
        "write",
        "file_path",
        "{records}/decisions/2026-10.jsonl",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    (
        Who::Controller,
        "edit",
        "file_path",
        "{root}/probe/.claude/settings.json",
        Verdict::Deny,
        "project settings",
    ),
    (
        Who::Worker,
        "write",
        "file_path",
        "{trees}/probe/t/src/lib.rs",
        Verdict::Allow,
        "inside",
    ),
    (
        Who::Worker,
        "write",
        "file_path",
        "{root}/other/x",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    (
        Who::Worker,
        "write",
        "file_path",
        "{records}/STATUS.md",
        Verdict::Deny,
        "outside the repository's checkout",
    ),
    // Controller, SendMessage: conductor only.
    (
        Who::Controller,
        "send-message",
        "to",
        "conductor",
        Verdict::Allow,
        "a session named conductor",
    ),
    (
        Who::Controller,
        "send-message",
        "to",
        "nobody",
        Verdict::Deny,
        "only conductor",
    ),
    (
        Who::Controller,
        "send-message",
        "to",
        "uds:/run/user/4242/cc-socks/4100011.sock",
        Verdict::Deny,
        "only conductor",
    ),
    // Controller, Bash.
    (
        Who::Controller,
        "bash",
        "command",
        "git status",
        Verdict::Allow,
        "",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "cd ../other",
        Verdict::Deny,
        "reaches another repository",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "git -C {trees}/other/t status",
        Verdict::Deny,
        "reaches another repository",
    ),
    (
        Who::Worker,
        "bash",
        "command",
        "cd ../../other/t",
        Verdict::Deny,
        "reaches another repository",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "cd {records}/decisions",
        Verdict::Deny,
        "reaches",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "gh pr merge 3",
        Verdict::Deny,
        "GitHub write",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "echo {} > .claude/settings.json",
        Verdict::Deny,
        "project settings",
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "conductor decision record-operator-decision --input-json -",
        Verdict::Deny,
        "only conductor runs it",
    ),
    // Controller, conductor's config (wave 08 W5): the file the process resolves, by a file tool
    // or a Bash write form; reading it is allowed.
    (
        Who::Controller,
        "write",
        "file_path",
        "{config}",
        Verdict::Deny,
        NOT_CONFIG,
    ),
    (
        Who::Worker,
        "edit",
        "file_path",
        "{config}",
        Verdict::Deny,
        NOT_CONFIG,
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "cp x {config}",
        Verdict::Deny,
        NOT_CONFIG,
    ),
    (
        Who::Worker,
        "bash",
        "command",
        "sed -i s/a/b/ ~/.b10x/conductor/conductor.yaml",
        Verdict::Deny,
        NOT_CONFIG,
    ),
    (
        Who::Controller,
        "bash",
        "command",
        "cat {config}",
        Verdict::Allow,
        "",
    ),
    // Conductor: its records only, any recipient, every leaf, no GitHub write; its Bash writes the
    // config, and its file tools do not, as before (wave 08 W5, decision 3).
    (
        Who::Conductor,
        "bash",
        "command",
        "cp x {config}",
        Verdict::Allow,
        "",
    ),
    (
        Who::Conductor,
        "write",
        "file_path",
        "{config}",
        Verdict::Deny,
        "conductor writes only its records",
    ),
    (
        Who::Conductor,
        "write",
        "file_path",
        "{records}/decisions/2026-10.jsonl",
        Verdict::Allow,
        "one of conductor's records",
    ),
    (
        Who::Conductor,
        "edit",
        "file_path",
        "{records}/STATUS.md",
        Verdict::Allow,
        "one of conductor's records",
    ),
    (
        Who::Conductor,
        "write",
        "file_path",
        "{records}/crates/x",
        Verdict::Deny,
        "conductor writes only its records",
    ),
    (
        Who::Conductor,
        "write",
        "file_path",
        "{root}/other/x",
        Verdict::Deny,
        "conductor writes only its records",
    ),
    (
        Who::Conductor,
        "send-message",
        "to",
        "nobody",
        Verdict::Allow,
        "not restricted",
    ),
    (
        Who::Conductor,
        "bash",
        "command",
        "conductor decision record-operator-decision --input-json -",
        Verdict::Allow,
        "",
    ),
    (
        Who::Conductor,
        "bash",
        "command",
        "gh pr merge 3",
        Verdict::Deny,
        "GitHub write",
    ),
    // A session in no place a rule set applies to.
    (
        Who::Outside,
        "write",
        "file_path",
        "{records}/decisions/x",
        Verdict::Deny,
        "where every rule set applies",
    ),
    (
        Who::Outside,
        "send-message",
        "to",
        "conductor",
        Verdict::Deny,
        "where every rule set applies",
    ),
    (
        Who::Outside,
        "bash",
        "command",
        "git status",
        Verdict::Deny,
        "where every rule set applies",
    ),
];

/// The guard is a security boundary: each rule that denies a call without a config file denies it
/// with one, in the places the file names, and conductor's session is the one in `records`.
#[test]
fn every_rule_that_denies_without_a_config_file_denies_with_one() {
    let without = built_in_layout(&home("places-without-file"));
    let with = file_layout(&home("places-with-file"));
    let mut problems = Vec::new();
    for layout in [&without, &with] {
        for (who, name, field, value, verdict, because) in RULES {
            let cwds = match who {
                Who::Controller => vec![layout.root.join("probe")],
                Who::Worker => vec![layout.trees.join("probe/t")],
                Who::Conductor => vec![layout.conductor.clone()],
                Who::Outside => layout.outside.clone(),
            };
            let value = layout.fill(value);
            for cwd in cwds {
                let decision = layout.decide(name, &cwd, field, &value, &listed());
                if decision.verdict != verdict || !decision.reason.contains(because) {
                    problems.push(format!(
                        "{}: {who:?} in {}: {name} {value:?}: expected {verdict:?} ({because}), \
                         got {:?}: {}",
                        layout.name,
                        cwd.display(),
                        decision.verdict,
                        decision.reason
                    ));
                }
            }
        }
        for field in ["tool_name", "cwd"] {
            let mut raw = localise(fixture("write"), &layout.home);
            raw.as_object_mut().expect("an object").remove(field);
            let decision = guard::decide_in(&raw, &layout.places, &unlisted());
            if decision.verdict != Verdict::Deny || !decision.reason.contains(field) {
                problems.push(format!("{}: no {field}: {decision:?}", layout.name));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} calls decided wrongly:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

/// With a config file, the session in `records` is conductor's and is recorded as `conductor`; a
/// session in a checkout or a managed worktree is its repository's controller.
#[test]
fn with_a_config_file_conductor_is_the_session_in_records() {
    let layout = file_layout(&home("places-conductor-session"));
    let conductor = layout.decide(
        "write",
        &layout.records.join("docs"),
        "file_path",
        &layout.fill("{records}/decisions/2026-10.jsonl"),
        &unlisted(),
    );
    assert_verdict(&conductor, Verdict::Allow, "one of conductor's records");
    assert_eq!(conductor.repository, "conductor", "{conductor:#?}");
    for cwd in [layout.root.join("repo"), layout.trees.join("repo/t")] {
        let controller = layout.decide("send-message", &cwd, "to", "nobody", &unlisted());
        assert_verdict(&controller, Verdict::Deny, "only conductor");
        assert_eq!(controller.repository, "repo", "{controller:#?}");
    }
}

/// Records a config file puts in a checkout are conductor's: a session in them runs conductor's
/// rule, and a controller of that repository, in its managed worktrees, neither writes them nor
/// reaches them.
#[test]
fn records_inside_a_checkout_are_conductor_s_and_no_controller_writes_or_reaches_them() {
    let home = home("places-records-in-a-checkout");
    let case = home.parent().expect("the case's directory");
    let (root, trees) = (case.join("root"), case.join("trees"));
    let records = root.join("conductor");
    let layout = Layout {
        name: "records in a checkout",
        places: places_from_file(&home, &config_text(&root, &trees, &records)),
        config: file_beside(&home),
        home: home.clone(),
        outside: Vec::new(),
        conductor: records.clone(),
        root,
        trees,
        records,
    };
    let write = |cwd: &Path, target: &str| {
        layout.decide("write", cwd, "file_path", &layout.fill(target), &unlisted())
    };
    assert_verdict(
        &write(&layout.records, "{records}/decisions/2026-10.jsonl"),
        Verdict::Allow,
        "one of conductor's records",
    );
    assert_verdict(
        &write(&layout.records, "{records}/src/lib.rs"),
        Verdict::Deny,
        "conductor writes only its records",
    );
    let tree = layout.trees.join("conductor/t");
    assert_verdict(
        &write(&tree, "{trees}/conductor/t/src/lib.rs"),
        Verdict::Allow,
        "inside",
    );
    for target in [
        "{records}/decisions/2026-10.jsonl",
        "{records}/STATUS.md",
        "{records}/src/lib.rs",
    ] {
        let decision = write(&tree, target);
        assert_verdict(&decision, Verdict::Deny, "conductor's records");
        assert_eq!(decision.repository, "conductor", "{decision:#?}");
    }
    for command in ["cd {records}/decisions", "git -C {records} log"] {
        let decision = layout.decide("bash", &tree, "command", &layout.fill(command), &unlisted());
        assert_verdict(&decision, Verdict::Deny, "conductor's records");
    }
}

/// A config file may put the managed worktrees inside the checkouts root: a worktree belongs to
/// its repository, not to a repository named after the worktree root.
#[test]
fn managed_worktrees_inside_the_checkouts_root_belong_to_their_repository() {
    let home = home("places-trees-in-the-root");
    let case = home.parent().expect("the case's directory");
    let root = case.join("work");
    let trees = root.join(".trees");
    let records = case.join("records");
    let layout = Layout {
        name: "trees in the root",
        places: places_from_file(&home, &config_text(&root, &trees, &records)),
        config: file_beside(&home),
        home: home.clone(),
        outside: Vec::new(),
        conductor: records.clone(),
        root,
        trees,
        records,
    };
    let worker = layout.trees.join("probe/t");
    let decide = |field: &str, value: &str| {
        let name = if field == "command" { "bash" } else { "write" };
        layout.decide(name, &worker, field, &layout.fill(value), &unlisted())
    };
    for target in ["{trees}/probe/t/src/lib.rs", "{root}/probe/README.md"] {
        let decision = decide("file_path", target);
        assert_verdict(&decision, Verdict::Allow, "inside");
        assert_eq!(decision.repository, "probe", "{decision:#?}");
    }
    for target in ["{trees}/other/t/src/lib.rs", "{root}/other/x"] {
        assert_verdict(
            &decide("file_path", target),
            Verdict::Deny,
            "outside the repository's checkout",
        );
    }
    assert_verdict(
        &decide("command", "cd {trees}/other/t"),
        Verdict::Deny,
        "reaches another repository",
    );
}

/// Without a config file, the places of the instance the process runs are the built-in ones, so
/// the hook decides as [`guard::decide`] does.
#[test]
fn without_a_config_file_the_active_places_are_the_built_in_ones() {
    let home = home("places-active-without-file");
    let environment = config::Environment {
        home: Some(home.clone()),
        cwd: home.join("somewhere"),
        config: None,
        instance: None,
    };
    let active = config::resolve(None, &environment).expect("the built-in instance");
    assert!(!active.from_file);
    assert_eq!(Places::active(&home, &active), Places::built_in(&home));
}

// -------------------------------------------------------------------------------------------------
// Wave 08 W5: a controller does not write conductor's config. A controller that rewrote it could
// make its own cwd conductor's `records`, and so get conductor's rules and write leaves, or set
// `checkouts.root` to `/` and so write anywhere. The config is the file the process resolves
// (`config::locate`: `CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml`), the default
// file, the directory `~/.b10x/conductor/` itself, and with a config file the instance's state
// directory (`story:guard-write-forms-and-config-scope`), where links land. Reading it is allowed,
// and conductor's own session is decided as before.
// -------------------------------------------------------------------------------------------------

/// What every denial of a write to conductor's config says.
const NOT_CONFIG: &str = "a controller does not write conductor's config";

/// The default config file, as a session writes it.
const CONFIG: &str = "~/.b10x/conductor/conductor.yaml";

/// Acceptance: a Write to the config file is denied from a controller's checkout and from its
/// worker's tree; the reason names the target and the config file. Another file under
/// `~/.b10x/conductor/` is not the config (`story:guard-write-forms-and-config-scope`): it is
/// denied as any place outside the checkout is.
#[test]
fn a_controller_writes_no_config_with_a_file_tool() {
    let home = home("config-file-tools");
    for cwd in [PROBE, WORKER] {
        for (name, field) in FILE_TOOLS {
            for target in [
                CONFIG,
                "~/.b10x/conductor/x/../conductor.yaml",
                "~/.b10x/./conductor/conductor.yaml",
            ] {
                let decision = decide(name, &home, cwd, &[(field, target)]);
                assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
                assert_verdict(&decision, Verdict::Deny, &under(&home, target));
                assert_verdict(&decision, Verdict::Deny, CONFIG);
            }
            for target in [
                "~/.b10x/conductor/fixture/records/decisions/2026-10.jsonl",
                "~/.b10x/conductor/fixture/state/tree/x",
                "~/.b10x/conductor/new.yaml",
            ] {
                let decision = decide(name, &home, cwd, &[(field, target)]);
                assert_verdict(&decision, Verdict::Deny, "outside");
                assert!(!decision.reason.contains(NOT_CONFIG), "{decision:#?}");
            }
            // A literal `~/` the session sends is the home directory.
            let mut raw = payload(name, &home, cwd, &[]);
            raw["tool_input"][field] = Value::String(CONFIG.to_owned());
            assert_verdict(&guard::decide(&raw, &home), Verdict::Deny, NOT_CONFIG);
        }
    }
}

/// Only the home directory's `~/.b10x/conductor/` is the config's: a sibling of it is denied as any
/// place outside the checkout is, and a directory of that name inside the checkout is the
/// checkout's. (A bound, not an acceptance line: it holds on the base too.)
#[test]
fn the_config_directory_is_the_home_directory_s_and_no_other() {
    let home = home("config-file-bounds");
    for (name, field) in FILE_TOOLS {
        for target in ["~/.b10x/conductor-old/x", "~/.b10x/x"] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "outside");
            assert!(!decision.reason.contains(NOT_CONFIG), "{decision:#?}");
        }
        let decision = decide(
            name,
            &home,
            PROBE,
            &[(
                field,
                "~/example-org/.guard-probe-w04/.b10x/conductor/conductor.yaml",
            )],
        );
        assert_verdict(&decision, Verdict::Allow, "inside");
    }
}

/// The guard follows links here as it does elsewhere: a link in the checkout to the config does
/// not carry a write into it, and when `~/.b10x` or `~/.b10x/conductor` is a link into the
/// checkout, a write that lands on the config, through the link or through the checkout, is denied.
/// Another file under the linked directory is not the config: outside the checkout it is denied as
/// outside, inside it is the checkout's.
#[test]
fn a_link_does_not_carry_a_controller_s_write_into_the_config() {
    let link = |target: &Path, at: &Path| {
        std::os::unix::fs::symlink(target, at).expect("plant the link");
    };
    let denied = |home: &Path, targets: &[&str]| {
        for (name, field) in FILE_TOOLS {
            for target in targets {
                let decision = decide(name, home, PROBE, &[(field, target)]);
                assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
            }
        }
    };

    // A link in the checkout to the config directory, and one to the config file.
    let home_out = home("config-link-out");
    let checkout = home_out.join("example-org/.guard-probe-w04");
    let dir = home_out.join(".b10x/conductor");
    fs::create_dir_all(&checkout).expect("create the checkout");
    fs::create_dir_all(&dir).expect("create the config directory");
    link(&dir, &checkout.join("cfg"));
    link(&dir.join("conductor.yaml"), &checkout.join("plain.yaml"));
    denied(&home_out, &["cfg/conductor.yaml", "plain.yaml"]);
    for (name, field) in FILE_TOOLS {
        let decision = decide(name, &home_out, PROBE, &[(field, "cfg/new.yaml")]);
        assert_verdict(&decision, Verdict::Deny, "outside");
    }

    // `~/.b10x` a link into the checkout: the config lands in the checkout.
    let home_in = home("config-link-in");
    let checkout = home_in.join("example-org/.guard-probe-w04");
    fs::create_dir_all(checkout.join("dot/conductor")).expect("create the linked directory");
    link(&checkout.join("dot"), &home_in.join(".b10x"));
    denied(
        &home_in,
        &[
            CONFIG,
            "~/example-org/.guard-probe-w04/dot/conductor/conductor.yaml",
            "dot/conductor/conductor.yaml",
        ],
    );
    for (name, field) in FILE_TOOLS {
        for target in ["dot/other.txt", "dot/conductor/fixture/records/x"] {
            let decision = decide(name, &home_in, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "inside");
        }
    }

    // `~/.b10x/conductor` itself a link into the checkout.
    let home_dir = home("config-dir-link-in");
    let checkout = home_dir.join("example-org/.guard-probe-w04");
    fs::create_dir_all(checkout.join("cfgdir")).expect("create the linked directory");
    fs::create_dir_all(home_dir.join(".b10x")).expect("create ~/.b10x");
    link(&checkout.join("cfgdir"), &home_dir.join(".b10x/conductor"));
    denied(&home_dir, &[CONFIG, "cfgdir/conductor.yaml"]);
    for (name, field) in FILE_TOOLS {
        let decision = decide(name, &home_dir, PROBE, &[(field, "cfgdir/x")]);
        assert_verdict(&decision, Verdict::Allow, "inside");
    }
}

/// Acceptance: a controller's Bash that names the config file or the directory
/// `~/.b10x/conductor` itself and holds a write form (the word list of the project settings rule)
/// is denied, `sed -i`, `cp` and a write to `"$CONDUCTOR_CONFIG"` among them; reading it is
/// allowed. A plain substring test, as for the project settings, so `cd ~/.b10x/conductor && cp x
/// conductor.yaml` is not caught. A path elsewhere under `~/.b10x/conductor/` is not the config
/// (`story:guard-write-forms-and-config-scope`), and without a config file nothing else is there.
#[test]
fn a_controller_s_bash_writes_no_config() {
    let home = home("config-bash");
    let config = under(&home, CONFIG);
    for cwd in [PROBE, WORKER] {
        for command in [
            "sed -i 's/^    records: .*/    records: ./' ~/.b10x/conductor/conductor.yaml",
            "cp x ~/.b10x/conductor/conductor.yaml",
            "echo 'checkouts: {root: /}' > \"$CONDUCTOR_CONFIG\"",
            "echo x >> ${CONDUCTOR_CONFIG}",
            "printf x > $HOME/.b10x/conductor/conductor.yaml",
            "printf x > \"${HOME}/.b10x/conductor/conductor.yaml\"",
            "printf x > \"$HOME\"/.b10x/conductor/conductor.yaml",
            &format!("printf x | tee {config}"),
            "mv x ~/.b10x/conductor/conductor.yaml",
            "rm -rf ~/.b10x/conductor",
            "ln -sf ~/x.yaml ~/.b10x/conductor/conductor.yaml",
            "install -m 600 x ~/.b10x/conductor/conductor.yaml",
            "truncate -s 0 ~/.b10x/conductor/conductor.yaml",
            "bash -c 'echo x > ~/.b10x/conductor/conductor.yaml'",
            "yq -i '.instances[0].records = \".\"' ~/.b10x/conductor/conductor.yaml",
            "perl -pi -e 's/x/y/' ~/.b10x/conductor/conductor.yaml",
            "dd if=x of=~/.b10x/conductor/conductor.yaml",
            "touch ~/.b10x/conductor/conductor.yaml",
            "rsync x ~/.b10x/conductor/conductor.yaml",
        ] {
            let decision = bash(&home, cwd, command);
            assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
            assert_verdict(&decision, Verdict::Deny, CONFIG);
        }
        for command in [
            "cat ~/.b10x/conductor/conductor.yaml",
            "cat \"$CONDUCTOR_CONFIG\"",
            &format!("grep records {config}"),
            "ls ~/.b10x/conductor",
            "echo x > notes.txt",
            "CONDUCTOR_CONFIG=x.yaml cargo test -p x > out.log",
            "cp x ~/.b10x/conductor/fixture/records/decisions/2026-10.jsonl",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Allow, "");
        }
    }
}

/// Where `~/.b10x` is a link, a Bash write naming where the config lands is denied too.
#[test]
fn a_controller_s_bash_names_the_config_where_it_lands() {
    let home = home("config-bash-link");
    let elsewhere = home.join("elsewhere/dot");
    fs::create_dir_all(elsewhere.join("conductor")).expect("create the linked directory");
    std::os::unix::fs::symlink(&elsewhere, home.join(".b10x")).expect("plant the link");
    for command in [
        format!("cp x {}/conductor/conductor.yaml", elsewhere.display()),
        "cp x ~/elsewhere/dot/conductor/conductor.yaml".to_owned(),
        "echo x > $HOME/elsewhere/dot/conductor/conductor.yaml".to_owned(),
        "rm -rf ~/elsewhere/dot/conductor".to_owned(),
    ] {
        assert_verdict(&bash(&home, PROBE, &command), Verdict::Deny, NOT_CONFIG);
    }
    assert_verdict(
        &bash(&home, PROBE, "cat ~/elsewhere/dot/conductor/conductor.yaml"),
        Verdict::Allow,
        "",
    );
}

/// The case directory's places of a config file `file` names, with checkouts under `root/`,
/// managed worktrees under `trees/` and conductor's records at `records`.
fn config_layout(home: &Path, name: &'static str, file: &Path, records: &Path) -> Layout {
    let case = home.parent().expect("the case's directory");
    let (root, trees) = (case.join("root"), case.join("trees"));
    Layout {
        name,
        places: places_from(home, file, &config_text(&root, &trees, records)),
        config: file.to_owned(),
        home: home.to_owned(),
        outside: Vec::new(),
        conductor: records.to_owned(),
        records: records.to_owned(),
        root,
        trees,
    }
}

/// A config file `CONDUCTOR_CONFIG` puts in a controller's checkout stays conductor's: neither the
/// controller nor its workers write it, by a file tool or a Bash write form, while the rest of the
/// checkout stays theirs.
#[test]
fn a_config_file_in_a_controller_s_checkout_is_not_the_controller_s() {
    let home = home("config-in-a-checkout");
    let case = home.parent().expect("the case's directory").to_owned();
    let layout = config_layout(
        &home,
        "config file in a checkout",
        &case.join("root/probe/conductor.yaml"),
        &case.join("records"),
    );
    let shown = layout.config.display().to_string();
    for cwd in [layout.root.join("probe"), layout.trees.join("probe/t")] {
        for (name, field) in FILE_TOOLS {
            let decision = layout.decide(name, &cwd, field, &shown, &unlisted());
            assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
            assert_verdict(&decision, Verdict::Deny, &shown);
            let other = layout.fill("{root}/probe/other.yaml");
            let decision = layout.decide(name, &cwd, field, &other, &unlisted());
            assert_verdict(&decision, Verdict::Allow, "inside");
        }
        for command in [
            "cp x {config}",
            "echo x > {config}",
            "sed -i s/a/b/ {config}",
            "echo x > \"$CONDUCTOR_CONFIG\"",
        ] {
            let command = layout.fill(command);
            let decision = layout.decide("bash", &cwd, "command", &command, &unlisted());
            assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
        }
        let command = layout.fill("cat {config}");
        let decision = layout.decide("bash", &cwd, "command", &command, &unlisted());
        assert_verdict(&decision, Verdict::Allow, "");
    }
}

/// Acceptance, decision 3: conductor's own session is not affected. Its Bash writes the config by
/// every form a controller is denied, and its file tools write its records and nothing else, as
/// before; with records under `~/.b10x/conductor/` (the default a config file leaves) it writes
/// them there, while a controller does not, by the records' own rule rather than the config's.
#[test]
fn conductor_s_session_is_not_affected_by_the_config_rule() {
    let home = home("config-conductor");
    for command in [
        "sed -i 's/a/b/' ~/.b10x/conductor/conductor.yaml",
        "cp x ~/.b10x/conductor/conductor.yaml",
        "echo x > \"$CONDUCTOR_CONFIG\"",
        "rm -rf ~/.b10x/conductor/old",
    ] {
        assert_verdict(&bash(&home, CONDUCTOR, command), Verdict::Allow, "");
    }
    for (name, field) in FILE_TOOLS {
        let decision = decide(name, &home, CONDUCTOR, &[(field, CONFIG)]);
        assert_verdict(
            &decision,
            Verdict::Deny,
            "conductor writes only its records",
        );
    }

    let records = home.join(".b10x/conductor/fixture/records");
    let layout = config_layout(
        &home,
        "records under the config directory",
        &file_beside(&home),
        &records,
    );
    let target = layout.fill("{records}/decisions/2026-10.jsonl");
    let conductor = layout.decide("write", &layout.records, "file_path", &target, &unlisted());
    assert_verdict(&conductor, Verdict::Allow, "one of conductor's records");
    assert_eq!(conductor.repository, "conductor", "{conductor:#?}");
    let controller = layout.root.join("probe");
    assert_verdict(
        &layout.decide("write", &controller, "file_path", &target, &unlisted()),
        Verdict::Deny,
        "outside",
    );
    let command = layout.fill("cp x {records}/decisions/2026-10.jsonl");
    assert_verdict(
        &layout.decide("bash", &controller, "command", &command, &unlisted()),
        Verdict::Deny,
        "a controller does not write them",
    );
    let command = layout.fill("cd {records}/decisions");
    assert_verdict(
        &layout.decide("bash", &controller, "command", &command, &unlisted()),
        Verdict::Deny,
        "reaches conductor's records",
    );
}

// -------------------------------------------------------------------------------------------------
// `story:guard-write-forms-and-config-scope`: a redirection writes only outside quotes and only to
// a file, and the config rule covers the config file and the instance's state directory, not the
// rest of `~/.b10x/conductor/`, where an instance's records are by default and keep their own rule.
// -------------------------------------------------------------------------------------------------

/// Acceptance: a descriptor duplication (`2>&1`, `>&2`, `1>&2`), a redirection to `/dev/null`, and
/// a `>` or `>>` inside quotes are no write forms, so a controller's read of the config that
/// carries one is allowed.
#[test]
fn a_redirection_to_a_descriptor_or_dev_null_or_inside_quotes_is_no_write() {
    let home = home("write-forms-read-only");
    for cwd in [PROBE, WORKER] {
        for command in [
            "cat ~/.b10x/conductor/conductor.yaml 2>&1",
            "cat ~/.b10x/conductor/conductor.yaml >&2",
            "cat ~/.b10x/conductor/conductor.yaml 1>&2",
            "cat ~/.b10x/conductor/conductor.yaml >/dev/null",
            "cat ~/.b10x/conductor/conductor.yaml 2>/dev/null",
            "cat ~/.b10x/conductor/conductor.yaml &>/dev/null",
            "cat ~/.b10x/conductor/conductor.yaml > /dev/null 2>&1",
            "awk 'NR>=490' ~/.b10x/conductor/conductor.yaml",
            "grep -c \"a > b\" ~/.b10x/conductor/conductor.yaml",
            "grep '>>' ~/.b10x/conductor/conductor.yaml | head -n 3",
            "bash -c \"awk 'NR>=490' ~/.b10x/conductor/conductor.yaml 2>&1\"",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Allow, "");
        }
    }
}

/// Acceptance, the earlier denials kept: each real write form (`>`, `>>`, `tee`, `cp`, `mv`,
/// `sed -i`) denies a controller's Bash that names the config, whether it writes the config or
/// writes elsewhere in the same command.
#[test]
fn a_real_write_form_in_a_command_naming_the_config_is_still_denied() {
    let home = home("write-forms-real");
    for cwd in [PROBE, WORKER] {
        for command in [
            "echo x > ~/.b10x/conductor/conductor.yaml",
            "echo x >> ~/.b10x/conductor/conductor.yaml",
            "echo x | tee ~/.b10x/conductor/conductor.yaml",
            "cp x ~/.b10x/conductor/conductor.yaml",
            "mv x ~/.b10x/conductor/conductor.yaml",
            "sed -i s/a/b/ ~/.b10x/conductor/conductor.yaml",
            "echo x 2>&1 >~/.b10x/conductor/conductor.yaml",
            "echo x &>> ~/.b10x/conductor/conductor.yaml",
            "echo x >| ~/.b10x/conductor/conductor.yaml",
            "echo \"$(echo x > ~/.b10x/conductor/conductor.yaml)\"",
            "cat ~/.b10x/conductor/conductor.yaml > copy.yaml",
            "cat ~/.b10x/conductor/conductor.yaml >> notes.txt",
            "cat ~/.b10x/conductor/conductor.yaml | tee copy.yaml",
            "cp ~/.b10x/conductor/conductor.yaml copy.yaml",
            "mv a b && cat ~/.b10x/conductor/conductor.yaml",
            "sed -i s/a/b/ notes.txt; cat ~/.b10x/conductor/conductor.yaml 2>&1",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Deny, NOT_CONFIG);
        }
    }
}

/// A config file beside `home` that leaves `records` and `state` to their defaults, under
/// `~/.b10x/conductor/fixture/`, with the checkout `root/probe/` and conductor's role settings
/// file `~/.config/fixture-roles/conductor.json`.
fn defaulted_places(home: &Path) -> Places {
    let case = home.parent().expect("the case's directory");
    fs::create_dir_all(case.join("root/probe/.git")).expect("create the probe checkout");
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: fixture\n\
         \x20   sources: [{{github: fixture}}]\n\
         \x20   checkouts: {{root: {}, trees: {}}}\n\
         \x20   roles:\n\
         \x20     - {{role: conductor, harness: claude, model: opus, settings: \
         ~/.config/fixture-roles/conductor.json}}\n",
        case.join("root").display(),
        case.join("trees").display(),
    );
    places_from_file(home, &text)
}

/// Acceptance: the config rule covers the config file and the instance's state directory, not the
/// rest of `~/.b10x/conductor/`. A controller's Bash reads the records there, with `2>&1` or a
/// quoted `>`, and writes beside a path that is none of them; it writes neither the config file
/// nor the state directory, and its Bash writes no record (the records' own rule).
#[test]
fn the_config_rule_covers_the_config_file_and_the_state_directory_only() {
    let home = home("config-scope");
    let places = defaulted_places(&home);
    let case = home.parent().expect("the case's directory").to_owned();
    let records = home.join(".b10x/conductor/fixture/records");
    let state = home.join(".b10x/conductor/fixture/state");
    let config = file_beside(&home);
    let at = |name: &str, cwd: &Path, field: &str, value: &str| {
        decide_in_places(&places, &home, name, cwd, field, value)
    };
    for cwd in [case.join("root/probe"), case.join("trees/probe/t")] {
        for command in [
            format!("ls {} 2>&1", records.display()),
            "ls ~/.b10x/conductor/fixture/records 2>&1".to_owned(),
            format!("awk 'NR>=490' {}/rules.md", records.display()),
            "cat ~/.b10x/conductor/other.txt > notes.txt".to_owned(),
            "cat ~/.b10x/conductor/fixture/state-old/x > notes.txt".to_owned(),
            "cat ~/.b10x/conductor/fixture/records-old/x > notes.txt".to_owned(),
            format!("ls {} 2>&1", state.display()),
        ] {
            let decision = at("bash", &cwd, "command", &command);
            assert_verdict(&decision, Verdict::Allow, "");
        }
        for command in [
            "cp x ~/.b10x/conductor/fixture/state/tree/x".to_owned(),
            format!("echo x > {}/x", state.display()),
            "rm -rf ~/.b10x/conductor/fixture/state".to_owned(),
            "rm -rf \"$HOME\"/.b10x/conductor/fixture/state/".to_owned(),
            "rm -rf ~/.b10x/conductor".to_owned(),
            "mv ~/.b10x/conductor/ x".to_owned(),
            format!("cp x {}", config.display()),
            "echo x > ~/.b10x/conductor/conductor.yaml".to_owned(),
        ] {
            let decision = at("bash", &cwd, "command", &command);
            assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
            assert_verdict(&decision, Verdict::Deny, "~/.b10x/conductor/fixture/state/");
        }
        for command in [
            format!("cp x {}/decisions/2026-10.jsonl", records.display()),
            "echo x >> ~/.b10x/conductor/fixture/records/STATUS.md".to_owned(),
            format!("cat {}/rules.md > notes.txt", records.display()),
        ] {
            let decision = at("bash", &cwd, "command", &command);
            assert_verdict(&decision, Verdict::Deny, "a controller does not write them");
            assert_verdict(
                &decision,
                Verdict::Deny,
                "~/.b10x/conductor/fixture/records/",
            );
        }
        for (name, field) in FILE_TOOLS {
            let decision = at(name, &cwd, field, &state.join("x").display().to_string());
            assert_verdict(&decision, Verdict::Deny, NOT_CONFIG);
            for target in [
                "~/.b10x/conductor/other.yaml",
                "~/.b10x/conductor/fixture/records/decisions/2026-10.jsonl",
            ] {
                let decision = at(name, &cwd, field, &under(&home, target));
                assert_verdict(&decision, Verdict::Deny, "outside");
                assert!(!decision.reason.contains(NOT_CONFIG), "{decision:#?}");
            }
        }
    }
}

/// Acceptance: conductor's own Bash that names a role's settings file runs with `2>&1` or a
/// redirection to `/dev/null`; a real write form beside that path still denies it (decision 3).
#[test]
fn conductor_s_bash_naming_a_role_s_settings_file_writes_only_by_a_real_write_form() {
    let home = home("write-forms-conductor");
    let places = defaulted_places(&home);
    let records = home.join(".b10x/conductor/fixture/records");
    fs::create_dir_all(&records).expect("create the records");
    let shown = records.display();
    let at = |command: &str| decide_in_places(&places, &home, "bash", &records, "command", command);
    for command in [
        "claude --bg --settings ~/.config/fixture-roles/conductor.json --model opus 'go on' 2>&1"
            .to_owned(),
        "claude --bg --settings ~/.config/fixture-roles/conductor.json >/dev/null 2>&1".to_owned(),
        format!("echo x >> {shown}/charters/alpha.md"),
    ] {
        let decision = at(&command);
        assert_eq!(decision.repository, "conductor", "{decision:#?}");
        assert_verdict(&decision, Verdict::Allow, "");
    }
    for command in [
        format!(
            "cat >> {shown}/charters/alpha.md < x; claude --bg --settings \
             ~/.config/fixture-roles/conductor.json"
        ),
        "echo '{}' > ~/.config/fixture-roles/conductor.json".to_owned(),
    ] {
        assert_verdict(&at(&command), Verdict::Deny, "settings");
    }
}

// -------------------------------------------------------------------------------------------------
// What the guard's files carry.
// -------------------------------------------------------------------------------------------------

/// The files of the guard this test holds to the two checks below: its code, its tests and its
/// recorded payloads.
fn guard_files() -> Vec<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<PathBuf> = [
        "src/guard.rs",
        "src/guard/rules.rs",
        "tests/guard_rules.rs",
        "tests/guard_hook.rs",
    ]
    .iter()
    .map(|file| crate_dir.join(file))
    .collect();
    let mut fixtures: Vec<PathBuf> = fs::read_dir(crate_dir.join("tests/fixtures/guard"))
        .expect("read the guard's fixtures")
        .map(|entry| entry.expect("a fixture").path())
        .collect();
    fixtures.sort();
    files.extend(fixtures);
    files
}

/// The characters a recorded value is made of: a scan hashes each window of its length within a
/// run of them.
#[derive(Debug, Clone, Copy)]
enum Charset {
    /// Lower-case hexadecimal digits: refs, pids, an agent id.
    Hex,
    /// Letters, digits, `_` and `-`: UUIDs and tool-use ids.
    Word,
    /// A word with `/` and `.`: a path.
    Path,
}

impl Charset {
    fn holds(self, c: char) -> bool {
        match self {
            Self::Hex => c.is_ascii_digit() || ('a'..='f').contains(&c),
            Self::Word => c.is_ascii_alphanumeric() || matches!(c, '_' | '-'),
            Self::Path => c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.'),
        }
    }
}

/// The values the guard's fixtures and tests carried from a live session: session, prompt and
/// tool-use ids, session-list refs, pids, an agent id, a transcript ref and the socket directory of
/// a user id. Each is held as what it was, its length, its characters and its SHA-256, so that this
/// file does not carry it either.
const RECORDED: [(&str, usize, Charset, &str); 21] = [
    (
        "the probe session's id",
        36,
        Charset::Word,
        "da2aba4ce5fc91c7e220c84aabed40b46055a6ac79ad61e8e585d35b545b568d",
    ),
    (
        "the probe session's prompt id",
        36,
        Charset::Word,
        "5a971a3ff9899f0f38f6a7f53f5c566618dcd3fed7ea9f3d1af867913fe45833",
    ),
    (
        "the Bash payload's tool-use id",
        30,
        Charset::Word,
        "b0c077d925daa40a72919f3096ac9383a404b94f537511f05de14e33e5d35ced",
    ),
    (
        "the Edit payload's tool-use id",
        30,
        Charset::Word,
        "e3bd2ef7baa1e04fdf3060de9c80b2832aa29cbef1bba2a6d349785e1c9f8df5",
    ),
    (
        "the NotebookEdit payload's tool-use id",
        30,
        Charset::Word,
        "8e9d0147a24ad4ae60053b627227881ef4cbc85d8fd5df1dab936a647d1034c6",
    ),
    (
        "the SendMessage payload's tool-use id",
        30,
        Charset::Word,
        "cffdf722f7a03f0d0da9398a22048329942af0d369a61d60019e6886411c0dab",
    ),
    (
        "the Write payload's tool-use id",
        30,
        Charset::Word,
        "8bc4d8c85d57be020b0bdfe1efdef00e1fe86330f142bcf8367d753f89a13ce2",
    ),
    (
        "the first live conductor's ref",
        8,
        Charset::Hex,
        "5fd87c0a13481c670863b1ba47874c5e6dffc55a8f7d170ef1c7af1dd5a54aaf",
    ),
    (
        "the second live conductor's ref",
        8,
        Charset::Hex,
        "1dde6aa548dba3107af8de75578dceaa4c6ddd800930d77795e54a36d46e3ebb",
    ),
    (
        "the stopped conductor's ref",
        8,
        Charset::Hex,
        "9c44b9a4ab84d24070295c792903cfff239cce063dd33d628744398b70cbf54b",
    ),
    (
        "the first live conductor's short ref",
        6,
        Charset::Hex,
        "97428a9f58a1e0850e5d9679489d18b3a4339458ec19d8faf8ff99ce5d03a112",
    ),
    (
        "the second live conductor's short ref",
        6,
        Charset::Hex,
        "f9745b13c33e443d8b9e810dec66de8a851d2deb08bd46529afdd0fe1d264532",
    ),
    (
        "the first live conductor's pid",
        7,
        Charset::Hex,
        "ba9f818c35082b033e3e589b5dbfb05d938587a13bae317cb895ad916bca3e62",
    ),
    (
        "the second live conductor's pid",
        7,
        Charset::Hex,
        "f4e7d99df6448304c519aa849c9ea7bd76750181aa0e6afee4ec7d6f71f2744d",
    ),
    (
        "another repository's controller's pid",
        7,
        Charset::Hex,
        "75626bd9b71bc795c138cb5665193628513941f5a5e33d2a9942825db1e9bf4f",
    ),
    (
        "conductor-dev's pid",
        7,
        Charset::Hex,
        "ecc4bc36baee3886eb8283658a2042c8c29d8127137501f58fc7e8b40dba5823",
    ),
    (
        "an unnamed session's pid",
        7,
        Charset::Hex,
        "2d6ba18e3f810026a1745f90a193937340ce9a1cf14decf55b584a033bb2ca8b",
    ),
    (
        "the pid of no session, beside them",
        7,
        Charset::Hex,
        "c7fb622477aecfc59067ef00584d82a90099f12729dad9c0ed4a49e5bfb51d9c",
    ),
    (
        "the agent id, its hexadecimal core",
        14,
        Charset::Hex,
        "780223bd2253129488786f37a661440b01ff058e76f01401efa8860e8f8694dd",
    ),
    (
        "a transcript's ref",
        8,
        Charset::Hex,
        "40b8ed9989487d730272e3e266fb0ba54fdd9c8d55cc89800025c9c65594eecd",
    ),
    (
        "the socket directory of a user id",
        23,
        Charset::Path,
        "11d8259eb05265e81fcdbc7e04fd5b85d3e14794e1b4e36d81a433502e87377e",
    ),
];

/// Each value of `recorded` that `text` holds, as what it was and where: every window of the
/// value's length within a run of its characters, hashed.
fn recorded_in(text: &str, recorded: &[(&str, usize, Charset, &str)]) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    for &(what, length, charset, digest) in recorded {
        let mut start = 0;
        while start < chars.len() {
            if !charset.holds(chars[start]) {
                start += 1;
                continue;
            }
            let end = (start..chars.len())
                .find(|&at| !charset.holds(chars[at]))
                .unwrap_or(chars.len());
            for at in start..(end + 1).saturating_sub(length) {
                let window: String = chars[at..at + length].iter().collect();
                if eventlog_core::blob_integrity_sha256(window.as_bytes()) == digest {
                    let line = chars[..at].iter().filter(|&&c| c == '\n').count() + 1;
                    found.push(format!("{what}, line {line}"));
                }
            }
            start = end;
        }
    }
    found
}

/// The scan finds a value by its digest wherever a run of its characters holds it, and nowhere
/// else.
#[test]
fn the_scan_finds_a_value_by_its_digest() {
    let digest = eventlog_core::blob_integrity_sha256(b"4100001");
    let control = [("a control", 7, Charset::Hex, digest.as_str())];
    assert_eq!(
        recorded_in("x\nuds:/s/4100001.sock and 04100001", &control),
        ["a control, line 2", "a control, line 2"]
    );
    assert_eq!(recorded_in("410000 1 41000012x", &control).len(), 1);
    assert!(recorded_in("4100002 41O0001", &control).is_empty());
}

/// The guard's files carry no id recorded from a live session (public audit, category on recorded
/// session data): none of [`RECORDED`], every UUID they hold is a synthetic one,
/// `00000000-0000-4000-8000-…`, and every tool-use id names a fixture.
#[test]
fn no_id_recorded_from_a_live_session_is_left_in_the_guard_s_files() {
    let mut problems = Vec::new();
    for file in guard_files() {
        let text = fs::read_to_string(&file).expect("read the guard's file");
        let shown = file.display();
        for found in recorded_in(&text, &RECORDED) {
            problems.push(format!("{shown}: {found}"));
        }
        let words: Vec<&str> = text
            .split(|c: char| !Charset::Word.holds(c))
            .filter(|word| !word.is_empty())
            .collect();
        for word in words {
            let uuid = word.len() == 36
                && word
                    .char_indices()
                    .all(|(at, c)| [8, 13, 18, 23].contains(&at) == (c == '-'))
                && word.chars().all(|c| c == '-' || Charset::Hex.holds(c));
            if uuid && !word.starts_with("00000000-0000-4000-8000-") {
                problems.push(format!("{shown}: a UUID that is not synthetic: {word}"));
            }
            if word.len() > "toolu_".len()
                && word.starts_with("toolu_")
                && !word.contains("Fixture")
            {
                problems.push(format!("{shown}: a tool-use id of no fixture: {word}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "recorded session data is left:\n  - {}",
        problems.join("\n  - ")
    );
}

/// The words that name the organization, its workspace or its worktree root.
const ORGANIZATION: [&str; 1] = ["example-org"];

/// The one word of the guard's code that still names the organization: the program of the bot
/// route, which a `gh` call is left to and which no field of the config names.
const STILL_NAMED: &str = "\"b10x-gates\"";

/// The guard's code reads the organization's places from the instance and names it nowhere else:
/// [`STILL_NAMED`] once, and nothing more. Its tests and recorded payloads are left out: they
/// decide the built-in places, today's, and `tests/guard_hook.rs` matches the wired settings.
#[test]
fn the_guard_s_code_names_no_organization() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut named = Vec::new();
    let mut still = 0;
    for file in ["src/guard.rs", "src/guard/rules.rs"] {
        let text = fs::read_to_string(crate_dir.join(file)).expect("read the guard's code");
        for (at, line) in text.lines().enumerate() {
            still += line.matches(STILL_NAMED).count();
            let rest = line.replace(STILL_NAMED, "");
            if ORGANIZATION.iter().any(|word| rest.contains(word)) {
                named.push(format!("{file}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    assert!(
        named.is_empty(),
        "the guard's code names the organization:\n  - {}",
        named.join("\n  - ")
    );
    assert_eq!(still, 1, "{STILL_NAMED} is named once, in GH_NOT_RUN");
}

// -------------------------------------------------------------------------------------------------
// Scratch (`story:guard-scratch`).
// -------------------------------------------------------------------------------------------------

/// The decision on the recorded payload `name` from `cwd`, `tmp` the session's `$TMPDIR`.
fn decide_with_tmp(
    name: &str,
    home: &Path,
    cwd: &str,
    tmp: Option<&Path>,
    set: &[(&str, &str)],
) -> Decision {
    let places = Places::built_in(home).with_tmp(tmp);
    guard::decide_in(
        &payload(name, home, cwd, set),
        &places,
        &SessionList::default(),
    )
}

/// A controller writes scratch under `$TMPDIR` and under `~/.cache/<repo>-<anything>/`, and not
/// at `~/.cache/<repo>-<anything>` itself, under another repository's prefix, or under `$TMPDIR`
/// when the hook has none.
#[test]
fn a_controller_writes_scratch_under_tmpdir_and_its_own_cache_prefix() {
    let home = home("controller-scratch");
    let tmp = home.join(".cache/claude-tmp");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/.cache/claude-tmp/x.py",
            "~/.cache/claude-tmp/review-1/extract.py",
            "~/.cache/.guard-probe-w04-review/x.py",
        ] {
            let decision = decide_with_tmp(name, &home, PROBE, Some(&tmp), &[(field, target)]);
            assert_verdict(&decision, Verdict::Allow, "scratch");
        }
        for target in [
            "~/.cache/.guard-probe-w04-review",
            "~/.cache/.guard-probe-w04x/x.py",
            "~/.cache/delta-review/x.py",
            "~/.cache/x.py",
            "~/.cache/claude-tmp",
        ] {
            let decision = decide_with_tmp(name, &home, PROBE, Some(&tmp), &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "outside");
        }
        let decision = decide_with_tmp(
            name,
            &home,
            PROBE,
            None,
            &[(field, "~/.cache/claude-tmp/x.py")],
        );
        assert_verdict(&decision, Verdict::Deny, "outside");
    }
}

/// Conductor writes scratch under `$TMPDIR` only: `~/.cache/conductor-*` holds the dashboard's
/// cache and other sessions' directories.
#[test]
fn conductor_writes_scratch_under_tmpdir_only() {
    let home = home("conductor-scratch");
    let tmp = home.join(".cache/claude-tmp");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    for (name, field) in FILE_TOOLS {
        let decision = decide_with_tmp(
            name,
            &home,
            CONDUCTOR,
            Some(&tmp),
            &[(field, "~/.cache/claude-tmp/review/x.py")],
        );
        assert_verdict(&decision, Verdict::Allow, "scratch under $TMPDIR");
        for target in [
            "~/.cache/conductor-review/x.py",
            "~/.cache/conductor-watch/x",
        ] {
            let decision = decide_with_tmp(name, &home, CONDUCTOR, Some(&tmp), &[(field, target)]);
            assert_verdict(
                &decision,
                Verdict::Deny,
                "conductor writes only its records",
            );
        }
    }
}

/// A `$TMPDIR` in a repository, a link under `$TMPDIR` into one, or `$TMPDIR` at the home directory
/// gives no session a way into a checkout or conductor's records.
#[test]
fn tmpdir_is_no_way_into_a_checkout_or_the_records() {
    let home = home("scratch-escape");
    let delta = home.join("example-org/delta");
    let conductor = home.join("example-org/conductor");
    for dir in [&delta, &conductor] {
        fs::create_dir_all(dir.join(".git")).expect("create a checkout");
    }
    let tmp = home.join(".cache/claude-tmp");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    std::os::unix::fs::symlink(&delta, tmp.join("into-delta")).expect("plant a link");
    std::os::unix::fs::symlink(&conductor, tmp.join("into-records")).expect("plant a link");
    for (name, field) in FILE_TOOLS {
        for (cwd, why) in [(PROBE, "outside"), (CONDUCTOR, "conductor writes only")] {
            for target in [
                "~/.cache/claude-tmp/into-delta/x",
                "~/.cache/claude-tmp/into-records/STATUS.md.x",
            ] {
                let decision = decide_with_tmp(name, &home, cwd, Some(&tmp), &[(field, target)]);
                assert_eq!(
                    decision.verdict,
                    Verdict::Deny,
                    "{cwd} {target}: {decision:#?}"
                );
            }
            let decision = decide_with_tmp(
                name,
                &home,
                cwd,
                Some(&delta),
                &[(field, "~/example-org/delta/x")],
            );
            assert_verdict(&decision, Verdict::Deny, why);
            let decision = decide_with_tmp(name, &home, cwd, Some(&home), &[(field, "~/x")]);
            assert_verdict(&decision, Verdict::Deny, why);
        }
    } // The case's checkouts hold `.git` directories, which `worktree gc` will not remove from a
    // finished tree's build scratch: remove the case when it passed.
    fs::remove_dir_all(&home).expect("remove the case's home");
}

/// A `$TMPDIR` that is the home directory or holds it (`/home`, `/`) is no scratch: it would make
/// the whole home directory writable. A `$TMPDIR` under the home directory still is.
#[test]
fn a_tmpdir_at_or_above_the_home_directory_is_no_scratch() {
    let home = home("scratch-above-home");
    let tmp = home.join(".cache/claude-tmp");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    let parent = home.parent().expect("the case's directory").to_owned();
    let grandparent = parent.parent().expect("above the case").to_owned();
    for (name, field) in FILE_TOOLS {
        for (cwd, why) in [(PROBE, "outside"), (CONDUCTOR, "conductor writes only")] {
            let control = decide_with_tmp(
                name,
                &home,
                cwd,
                Some(&tmp),
                &[(field, "~/.cache/claude-tmp/x.py")],
            );
            assert_verdict(&control, Verdict::Allow, "scratch");
            for dir in [&home, &parent, &grandparent, Path::new("/")] {
                for target in ["~/x.py", "~/.bashrc", "~/.cache/claude-tmp/x.py"] {
                    let decision = decide_with_tmp(name, &home, cwd, Some(dir), &[(field, target)]);
                    assert_verdict(&decision, Verdict::Deny, why);
                }
            }
        }
    }
}

/// A `$TMPDIR` that holds a `.claude` directory does not make its settings files scratch: they
/// can switch the guard off wherever they lie (second pre-publication review, finding 3).
#[test]
fn a_settings_file_under_tmpdir_is_no_scratch() {
    let home = home("scratch-settings");
    let tmp = home.join(".claude");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    for (name, field) in FILE_TOOLS {
        for (cwd, why) in [(PROBE, "settings"), (CONDUCTOR, "settings")] {
            let control =
                decide_with_tmp(name, &home, cwd, Some(&tmp), &[(field, "~/.claude/x.py")]);
            assert_verdict(&control, Verdict::Allow, "scratch");
            for target in ["~/.claude/settings.json", "~/.claude/settings.local.json"] {
                let decision = decide_with_tmp(name, &home, cwd, Some(&tmp), &[(field, target)]);
                assert_verdict(&decision, Verdict::Deny, why);
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// The settings files that wire the guard (pre-publication review, finding 2): no session writes
// a `.claude/` settings file (`settings*.json`, `controller-settings.json`,
// `conductor-settings.json`) or the `settings` file of a configured role, by a file tool or by a
// Bash write form.
// -------------------------------------------------------------------------------------------------

/// A controller's file tools write no `.claude/` settings file in its checkout or worktrees, and
/// conductor's write none even among its records.
#[test]
fn no_session_writes_a_guard_settings_file_with_a_file_tool() {
    let home = home("guard-settings-file-tools");
    for (name, field) in FILE_TOOLS {
        for target in [
            "~/example-org/.guard-probe-w04/.claude/controller-settings.json",
            "~/example-org/.guard-probe-w04/.claude/conductor-settings.json",
            "~/example-org/.guard-probe-w04/.claude/settings.dev.json",
            "~/.local/state/worktree/trees/example-org/.guard-probe-w04/t/.claude/controller-settings.json",
            ".claude/controller-settings.json",
        ] {
            let decision = decide(name, &home, PROBE, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "settings");
        }
        for target in [
            "~/example-org/conductor/charters/.claude/settings.json",
            "~/example-org/conductor/docs/handoff/.claude/conductor-settings.json",
        ] {
            let decision = decide(name, &home, CONDUCTOR, &[(field, target)]);
            assert_verdict(&decision, Verdict::Deny, "settings");
        }
        let control = decide(
            name,
            &home,
            PROBE,
            &[(field, ".claude/agents/controller.md")],
        );
        assert_verdict(&control, Verdict::Allow, "inside");
    }
}

/// Neither a controller's nor conductor's Bash writes a `.claude/` settings file; both read one.
#[test]
fn no_session_writes_a_guard_settings_file_through_bash() {
    let home = home("guard-settings-bash");
    for cwd in [PROBE, CONDUCTOR] {
        for command in [
            "echo '{}' > .claude/controller-settings.json",
            "cp x .claude/conductor-settings.json",
            "jq . x | tee ~/example-org/.guard-probe-w04/.claude/settings.dev.json",
            "sed -i 's/a/b/' .claude/settings.json",
            "printf x > \".claude/conductor-settings.json\"",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Deny, "settings");
        }
        for command in [
            "cat .claude/controller-settings.json",
            "jq . .claude/conductor-settings.json",
            "echo x > notes.txt",
        ] {
            assert_verdict(&bash(&home, cwd, command), Verdict::Allow, "");
        }
    }
}

/// The `settings` file each role of the config names is written by no session, wherever it is:
/// a controller's file tools do not write one in its checkout, conductor's do not write one among
/// its records, and no session's Bash writes one by any spelling of its path; reading it is
/// allowed.
#[test]
fn no_session_writes_a_configured_role_s_settings_file() {
    let home = home("guard-settings-configured");
    let case = home.parent().expect("the case's directory").to_owned();
    let (root, trees, records) = (case.join("root"), case.join("trees"), case.join("records"));
    fs::create_dir_all(root.join("probe/.git")).expect("create the probe checkout");
    let text = format!(
        "{}\x20   roles:\n\
         \x20     - {{role: controller, harness: claude, model: opus, settings: \
         ~/.config/fixture-roles/controller.json}}\n\
         \x20     - {{role: conductor-dev, harness: claude, model: opus, settings: {}}}\n\
         \x20     - {{role: conductor, harness: claude, model: opus, settings: {}}}\n",
        config_text(&root, &trees, &records),
        root.join("probe/ops/dev.json").display(),
        records.join("charters/conductor.json").display(),
    );
    let places = places_from_file(&home, &text);
    let decide_at = |name: &str, cwd: &Path, field: &str, value: &str| {
        let mut raw = localise(fixture(name), &home);
        raw["cwd"] = Value::String(cwd.display().to_string());
        raw["tool_input"][field] = Value::String(value.to_owned());
        guard::decide_in(&raw, &places, &unlisted())
    };
    let controller = root.join("probe");
    let dev = root.join("probe/ops/dev.json").display().to_string();
    let charter = records
        .join("charters/conductor.json")
        .display()
        .to_string();
    let user = home
        .join(".config/fixture-roles/controller.json")
        .display()
        .to_string();
    for (name, field) in FILE_TOOLS {
        let decision = decide_at(name, &controller, field, &dev);
        assert_verdict(&decision, Verdict::Deny, "settings");
        let decision = decide_at(name, &records, field, &charter);
        assert_verdict(&decision, Verdict::Deny, "settings");
        let control = decide_at(
            name,
            &controller,
            field,
            &root.join("probe/ops/other.json").display().to_string(),
        );
        assert_verdict(&control, Verdict::Allow, "inside");
    }
    for cwd in [&controller, &records] {
        for command in [
            format!("echo '{{}}' > {user}"),
            "echo '{}' > ~/.config/fixture-roles/controller.json".to_owned(),
            "cp x $HOME/.config/fixture-roles/controller.json".to_owned(),
            format!("tee {dev} < x"),
            format!("sed -i 's/a/b/' {charter}"),
        ] {
            let decision = decide_at("bash", cwd, "command", &command);
            assert_verdict(&decision, Verdict::Deny, "settings");
        }
        let control = decide_at(
            "bash",
            cwd,
            "command",
            "cat ~/.config/fixture-roles/controller.json",
        );
        assert_verdict(&control, Verdict::Allow, "");
    }
}

// -------------------------------------------------------------------------------------------------
// The `profile` file of each configured role (`story:guard-profile-files`): the system prompt its
// sessions start with, which no session writes, as no session writes a role's `settings` file.
// -------------------------------------------------------------------------------------------------

/// The case's checkouts root and conductor's records, beside `home`.
fn case_dirs(home: &Path) -> (PathBuf, PathBuf) {
    let case = home.parent().expect("the case's directory");
    (case.join("root"), case.join("records"))
}

/// A config file's places beside `home`, with the checkout `root/probe/`, whose roles name the
/// `profile` files of `profiles`: the controller's, conductor-dev's and conductor's. The root and
/// the records start empty, so that the links a case plants are its own.
fn places_with_profiles(home: &Path, profiles: [&str; 3]) -> Places {
    let case = home.parent().expect("the case's directory");
    let (root, records) = case_dirs(home);
    for dir in [&root, &records] {
        if dir.exists() {
            fs::remove_dir_all(dir).expect("clear the case's directory");
        }
    }
    fs::create_dir_all(root.join("probe/.git")).expect("create the probe checkout");
    let [controller, dev, conductor] = profiles;
    let text = format!(
        "{}\x20   roles:\n\
         \x20     - {{role: controller, harness: claude, model: opus, profile: {controller}}}\n\
         \x20     - {{role: conductor-dev, harness: claude, model: opus, profile: {dev}}}\n\
         \x20     - {{role: conductor, harness: claude, model: opus, profile: {conductor}}}\n",
        config_text(&root, &case.join("trees"), &records),
    );
    places_from_file(home, &text)
}

/// The decision on the recorded payload `name` in `places`, from `cwd`, with its `field` set to
/// `value` as written.
fn decide_in_places(
    places: &Places,
    home: &Path,
    name: &str,
    cwd: &Path,
    field: &str,
    value: &str,
) -> Decision {
    let mut raw = localise(fixture(name), home);
    raw["cwd"] = Value::String(cwd.display().to_string());
    raw["tool_input"][field] = Value::String(value.to_owned());
    guard::decide_in(&raw, places, &unlisted())
}

/// The `profile` file each role of the config names is written by no session, however the path
/// is spelled: a controller's file tools do not write one in its checkout or outside it, absolute,
/// relative to the cwd or through a link; conductor's do not write one among its records; and no
/// session's Bash writes one by any spelling of its path. Reading it is allowed.
#[test]
fn no_session_writes_a_configured_role_s_profile_file() {
    let home = home("guard-profile-configured");
    let (root, records) = case_dirs(&home);
    let dev = root.join("probe/ops/dev.md").display().to_string();
    let charter = records.join("charters/conductor.md").display().to_string();
    let places = places_with_profiles(
        &home,
        ["~/.config/fixture-roles/controller.md", &dev, &charter],
    );
    let user = home
        .join(".config/fixture-roles/controller.md")
        .display()
        .to_string();
    let controller = root.join("probe");
    fs::create_dir_all(root.join("probe/ops")).expect("create the profile's directory");
    std::os::unix::fs::symlink(&dev, root.join("probe/alias.md")).expect("plant a link");
    fs::create_dir_all(records.join("docs/handoff")).expect("create the handoff directory");
    std::os::unix::fs::symlink(&charter, records.join("docs/handoff/alias.md"))
        .expect("plant a link");
    for (name, field) in FILE_TOOLS {
        for target in [
            dev.as_str(),
            "ops/dev.md",
            "ops/x/../dev.md",
            "alias.md",
            user.as_str(),
            "~/.config/fixture-roles/controller.md",
        ] {
            let decision = decide_in_places(&places, &home, name, &controller, field, target);
            assert_verdict(&decision, Verdict::Deny, "profile");
        }
        for target in [
            charter.as_str(),
            "charters/conductor.md",
            "docs/handoff/alias.md",
        ] {
            let decision = decide_in_places(&places, &home, name, &records, field, target);
            assert_verdict(&decision, Verdict::Deny, "profile");
        }
        let control = decide_in_places(&places, &home, name, &controller, field, "ops/other.md");
        assert_verdict(&control, Verdict::Allow, "inside");
        let control = decide_in_places(&places, &home, name, &records, field, "charters/other.md");
        assert_verdict(&control, Verdict::Allow, "records");
    }
    for cwd in [&controller, &records] {
        for command in [
            format!("echo x > {user}"),
            "echo x > ~/.config/fixture-roles/controller.md".to_owned(),
            "cp x $HOME/.config/fixture-roles/controller.md".to_owned(),
            "cp x ${HOME}/.config/fixture-roles/controller.md".to_owned(),
            format!("tee {dev} < x"),
            format!("sed -i 's/a/b/' {charter}"),
        ] {
            let decision = decide_in_places(&places, &home, "bash", cwd, "command", &command);
            assert_verdict(&decision, Verdict::Deny, "profile");
        }
        for command in [
            "cat ~/.config/fixture-roles/controller.md".to_owned(),
            format!("cat {dev}"),
        ] {
            let decision = decide_in_places(&places, &home, "bash", cwd, "command", &command);
            assert_verdict(&decision, Verdict::Allow, "");
        }
    }
}

/// A role's `profile` file is no scratch: not under `$TMPDIR`, for any session, and not under a
/// controller's `~/.cache/<repo>-*/`.
#[test]
fn a_role_s_profile_file_is_no_scratch() {
    let home = home("guard-profile-scratch");
    let (root, records) = case_dirs(&home);
    let tmp = home.join(".cache/claude-tmp");
    fs::create_dir_all(&tmp).expect("create the temporary directory");
    let charter = records.join("charters/conductor.md").display().to_string();
    let places = places_with_profiles(
        &home,
        [
            "~/.cache/probe-notes/controller.md",
            "~/.cache/claude-tmp/profiles/dev.md",
            &charter,
        ],
    )
    .with_tmp(Some(&tmp));
    let controller = root.join("probe");
    for (name, field) in FILE_TOOLS {
        let decision = decide_in_places(
            &places,
            &home,
            name,
            &controller,
            field,
            "~/.cache/probe-notes/controller.md",
        );
        assert_verdict(&decision, Verdict::Deny, "profile");
        let control = decide_in_places(
            &places,
            &home,
            name,
            &controller,
            field,
            "~/.cache/probe-notes/other.md",
        );
        assert_verdict(&control, Verdict::Allow, "scratch");
        for cwd in [&controller, &records] {
            let decision = decide_in_places(
                &places,
                &home,
                name,
                cwd,
                field,
                "~/.cache/claude-tmp/profiles/dev.md",
            );
            assert_verdict(&decision, Verdict::Deny, "profile");
            let control = decide_in_places(
                &places,
                &home,
                name,
                cwd,
                field,
                "~/.cache/claude-tmp/profiles/other.md",
            );
            assert_verdict(&control, Verdict::Allow, "scratch");
        }
    }
}
