//! `conductor broker serve` (`story:tool-broker-read`, acceptances 2 and 5): it serves the
//! instance's broker over the state directory's `broker/`, opens the socket of a session
//! registered while it serves, answers a call on it, records the call in the instance's store as
//! a `conductor.tool.ToolCall`, and on SIGTERM closes its sockets and exits 0.
//!
//! The case lies under the system's temporary directory, not `target/tmp`: a socket's path holds
//! at most 107 bytes. The binary runs through [`common::conductor`], with the case's own home and
//! `--state-dir`.

mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Stdio};

use conductor_broker::{
    Level, Placement, Registration, Request, SessionGrant, broker_dir, call, register,
};
use conductor_cli::cli::{Access, LEAVES};
use conductor_cli::store;
use conductor_model::behaviour::ToolCallStorage;
use conductor_model::tool::{CallOutcome, Verb};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde_json::json;

struct Case {
    root: PathBuf,
    child: Option<Child>,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("cbs-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["home", "s", "w"] {
            fs::create_dir_all(root.join(dir)).expect("create the case's directories");
        }
        fs::write(root.join("w/a.txt"), "alpha\n").expect("write a fixture");
        Self { root, child: None }
    }

    fn state(&self) -> PathBuf {
        self.root.join("s")
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn broker_serve_is_declared_a_writing_leaf_only_conductor_runs() {
    let declared = LEAVES
        .iter()
        .find(|(words, _)| *words == ["broker", "serve"])
        .map(|(_, access)| *access);
    assert_eq!(declared, Some(Access::Writes));
}

#[test]
fn broker_serve_answers_a_registered_session_and_records_the_call_in_the_store() {
    let mut case = Case::new("serve");
    let mut child = common::conductor(case.root.join("home"))
        .args(["broker", "serve", "--state-dir"])
        .arg(case.state())
        .current_dir(&case.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start conductor broker serve");
    let mut first = String::new();
    BufReader::new(child.stdout.take().expect("the broker's output"))
        .read_line(&mut first)
        .expect("read the broker's first line");
    case.child = Some(child);
    let instance = first
        .strip_prefix("broker of ")
        .and_then(|rest| rest.split(' ').next())
        .unwrap_or_else(|| panic!("no serving line: {first:?}"))
        .to_owned();
    assert!(
        first
            .trim_end()
            .ends_with(&broker_dir(&case.state()).display().to_string()),
        "{first}"
    );

    let socket = register(
        &broker_dir(&case.state()),
        &Registration {
            session: "example-org-a".to_owned(),
            instance: instance.clone(),
            workdir: case.root.join("w"),
            grant: SessionGrant {
                tools: vec![Verb::Show],
                repositories: Vec::new(),
                paths: Vec::new(),
                max_output_bytes: 65_536,
                expires_at: None,
            },
            started_at: "2026-10-17T00:00:00Z".to_owned(),
            place: Placement::default(),
        },
    )
    .expect("register a session");
    let shown = call(
        &socket,
        &Request {
            verb: "show".to_owned(),
            args: json!({"path": "a.txt"}),
        },
    );
    let refused = call(
        &socket,
        &Request {
            verb: "diff".to_owned(),
            args: json!({}),
        },
    );

    assert_eq!(shown.exit, 0, "{}", shown.text);
    assert!(shown.text.starts_with("alpha\n"), "{}", shown.text);
    assert_eq!(refused.text, "NotGranted: tools: diff\n");
    let mut child = case.child.take().expect("the broker runs");
    kill(
        Pid::from_raw(i32::try_from(child.id()).expect("a pid")),
        Signal::SIGTERM,
    )
    .expect("stop the broker");
    let status = child.wait().expect("wait for the broker");
    assert!(
        status.success(),
        "broker serve exits 0 on SIGTERM: {status}"
    );
    assert!(
        !socket.exists(),
        "the socket is removed when the broker stops"
    );

    let store = store::open_existing(&case.state()).expect("the broker created the store");
    let calls: Vec<_> = ToolCallStorage::list(&store)
        .into_iter()
        .map(|call| call.data)
        .collect();
    assert_eq!(calls.len(), 2, "{calls:#?}");
    let show = calls
        .iter()
        .find(|call| call.verb == Verb::Show)
        .expect("the show is recorded");
    assert_eq!(show.session.0, "example-org-a");
    assert_eq!(show.instance.0, instance);
    assert_eq!(show.outcome, CallOutcome::Done);
    assert_eq!(show.paths, vec!["a.txt".to_owned()]);
    assert_eq!(show.output_bytes, shown.text.len() as i64);
    let diff = calls
        .iter()
        .find(|call| call.verb == Verb::Diff)
        .expect("the refused diff is recorded");
    assert_eq!(diff.outcome, CallOutcome::NotGranted);
    assert_eq!(diff.rule.as_deref(), Some("tools: diff"));
}

/// `conductor broker serve` records `send` and `report` in the instance's store
/// (`story:tool-build-commit-report`, acceptances 4 and 5): a team message as a `Message` of kind
/// `Team`, and a lead's report as a `[REPORT]` message that moves the lead's open dispatch as
/// `conductor dispatch report-done` does.
#[test]
fn broker_serve_records_a_team_message_and_a_leads_report_moves_its_dispatch() {
    let mut case = Case::new("team");
    let (root, state) = (case.root.clone(), case.state());
    let conductor = |args: &[&str]| {
        let output = common::conductor(root.join("home"))
            .arg("--state-dir")
            .arg(&state)
            .args(args)
            .current_dir(&root)
            .stdin(Stdio::null())
            .output()
            .expect("run conductor");
        assert!(output.status.success(), "conductor {args:?}: {output:?}");
        String::from_utf8(output.stdout).expect("conductor prints text")
    };
    conductor(&[
        "goal",
        "propose-goal",
        "--goal-id",
        "G1",
        "--title",
        "Ship the parser",
        "--exit-evidence",
        "G1 is listed as met",
    ]);
    let dispatch = conductor(&[
        "dispatch",
        "send-dispatch",
        "--repository",
        "alpha",
        "--goal-id",
        "G1",
        "--brief",
        "Ship the parser",
        "--sent-at",
        "2026-10-17T00:00:00Z",
        "--priority",
        "1",
    ])
    .trim()
    .to_owned();
    let mut child = common::conductor(case.root.join("home"))
        .args(["broker", "serve", "--state-dir"])
        .arg(case.state())
        .current_dir(&case.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start conductor broker serve");
    let mut first = String::new();
    BufReader::new(child.stdout.take().expect("the broker's output"))
        .read_line(&mut first)
        .expect("read the broker's first line");
    case.child = Some(child);
    let instance = first
        .strip_prefix("broker of ")
        .and_then(|rest| rest.split(' ').next())
        .unwrap_or_else(|| panic!("no serving line: {first:?}"))
        .to_owned();
    let registered = |session: &str, place: Placement| {
        register(
            &broker_dir(&case.state()),
            &Registration {
                session: session.to_owned(),
                instance: instance.clone(),
                workdir: case.root.join("w"),
                grant: SessionGrant {
                    tools: vec![Verb::Send, Verb::Report],
                    repositories: Vec::new(),
                    paths: Vec::new(),
                    max_output_bytes: 65_536,
                    expires_at: None,
                },
                started_at: "2026-10-17T00:00:00Z".to_owned(),
                place,
            },
        )
        .expect("register a session")
    };
    let lead = registered(
        "example-org-alpha",
        Placement {
            role: Some("controller".to_owned()),
            level: Some(Level::Lead),
            team: Some("alpha".to_owned()),
            above: Some("example-org-conductor".to_owned()),
        },
    );
    let worker = registered(
        "example-org-alpha-implementer",
        Placement {
            role: Some("implementer".to_owned()),
            level: Some(Level::Worker),
            team: Some("alpha".to_owned()),
            above: Some("example-org-alpha".to_owned()),
        },
    );

    let sent = call(
        &worker,
        &Request {
            verb: "send".to_owned(),
            args: json!({"to": "controller", "text": "the parser is green"}),
        },
    );
    let reported = call(
        &lead,
        &Request {
            verb: "report".to_owned(),
            args: json!({"status": "done", "line": "tests green on main"}),
        },
    );
    let mut child = case.child.take().expect("the broker runs");
    kill(
        Pid::from_raw(i32::try_from(child.id()).expect("a pid")),
        Signal::SIGTERM,
    )
    .expect("stop the broker");
    assert!(child.wait().expect("wait for the broker").success());

    assert_eq!(sent.exit, 0, "{}", sent.text);
    assert_eq!(reported.exit, 0, "{}", reported.text);
    assert!(reported.text.contains(&dispatch), "{}", reported.text);
    let rows = |group: &str, view: &str| -> Vec<serde_json::Value> {
        conductor(&[group, view, "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON row"))
            .collect()
    };
    let dispatches = rows("dispatch", "dispatches");
    let moved = dispatches
        .iter()
        .find(|row| row["dispatch_id"] == dispatch.as_str())
        .expect("the dispatch is listed");
    assert_eq!(moved["state"], "Done", "{moved}");
    let messages = rows("message", "messages");
    let team = messages
        .iter()
        .find(|row| row["kind"] == "Team")
        .unwrap_or_else(|| panic!("a team message is recorded: {messages:?}"));
    assert_eq!(team["state"], "Received", "{team}");
    assert_eq!(team["sender"], "example-org-alpha-implementer");
    assert_eq!(team["recipient"], "example-org-alpha");
    assert_eq!(team["first_line"], "[TEAM alpha] the parser is green");
    assert!(
        sent.text
            .contains(team["message_id"].as_str().expect("an id"))
    );
    let report = messages
        .iter()
        .find(|row| row["kind"] == "Report")
        .unwrap_or_else(|| panic!("the report is recorded: {messages:?}"));
    assert_eq!(report["state"], "Received", "{report}");
    assert_eq!(report["sender"], "example-org-alpha");
    assert_eq!(report["recipient"], "example-org-conductor");
    assert_eq!(
        report["first_line"],
        format!("[REPORT alpha {dispatch}] done tests green on main")
    );
}
