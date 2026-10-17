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

use conductor_broker::{Registration, Request, SessionGrant, broker_dir, call, register};
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
