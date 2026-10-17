//! `tool mcp` serves the session's granted verbs as MCP tools over stdio, backed by the same
//! socket as the command line (`story:tool-broker-read`, acceptance 6): `tools/list` returns
//! exactly the granted verbs, and `tools/call show` answers what `tool show` prints.
//!
//! The broker runs in this process over a case directory under the system's temporary directory
//! (a socket's path is short); the `tool` binary runs with that session's socket as `TOOL_SOCKET`
//! and an empty home, and reads nothing else.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use conductor_broker::{Broker, MemoryRecorder, Registration, SessionGrant, Verb, register};
use serde_json::{Value, json};

const INSTANCE: &str = "example-org";

struct Case {
    root: PathBuf,
    broker: Option<Broker>,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("tm-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("work")).expect("create the working directory");
        fs::create_dir_all(root.join("home")).expect("create the home directory");
        fs::write(root.join("work/a.txt"), "alpha\nbeta\n").expect("write a fixture");
        Self { root, broker: None }
    }

    fn dir(&self) -> PathBuf {
        conductor_broker::broker_dir(&self.root.join("state"))
    }

    fn register(&self, session: &str, tools: &[Verb]) -> PathBuf {
        register(
            &self.dir(),
            &Registration {
                session: session.to_owned(),
                instance: INSTANCE.to_owned(),
                workdir: self.root.join("work"),
                grant: SessionGrant {
                    tools: tools.to_vec(),
                    repositories: Vec::new(),
                    paths: Vec::new(),
                    max_output_bytes: 65_536,
                    expires_at: None,
                },
                started_at: "2026-10-17T00:00:00Z".to_owned(),
            },
        )
        .expect("register the session")
    }

    fn serve(&mut self) {
        self.broker = Some(
            Broker::start(&self.dir(), INSTANCE, Arc::new(MemoryRecorder::default()))
                .expect("start the broker"),
        );
    }

    /// The `tool` binary, with `socket` as its session's socket and nothing of the caller's
    /// environment.
    fn tool(&self, socket: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tool"));
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("TOOL_SOCKET", socket);
        command
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        if let Some(broker) = self.broker.take() {
            broker.shutdown();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Sends `requests` to `tool mcp`, one JSON-RPC message per line, and answers each response, by
/// the order the requests with an id were sent.
fn mcp(command: &mut Command, requests: &[Value]) -> Vec<Value> {
    let mut child = command
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start tool mcp");
    let mut stdin = child.stdin.take().expect("tool mcp's stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("tool mcp's stdout"));
    let mut responses = Vec::new();
    for request in requests {
        writeln!(stdin, "{request}").expect("write a request");
        stdin.flush().expect("flush a request");
        if request.get("id").is_some() {
            let mut line = String::new();
            stdout.read_line(&mut line).expect("read a response");
            responses.push(serde_json::from_str(&line).expect("a JSON-RPC response"));
        }
    }
    drop(stdin);
    let status = child.wait().expect("wait for tool mcp");
    assert!(status.success(), "tool mcp exits 0 at the end of its input");
    responses
}

fn initialize() -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "example", "version": "0"}
    }})
}

#[test]
fn tools_list_returns_exactly_the_granted_verbs() {
    let mut case = Case::new("list");
    let socket = case.register("example-org-a", &[Verb::Show, Verb::Diff]);
    case.serve();

    let responses = mcp(
        &mut case.tool(&socket),
        &[
            initialize(),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        ],
    );

    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "tool");
    assert!(responses[0]["result"]["capabilities"]["tools"].is_object());
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools/list answers a list of tools");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("a tool's name"))
        .collect();
    assert_eq!(names, vec!["show", "diff"]);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
    }
    let show = &tools[0]["inputSchema"];
    assert_eq!(show["required"], json!(["path"]));
}

#[test]
fn tools_call_show_answers_what_tool_show_prints() {
    let mut case = Case::new("call");
    // Two sessions with the same grant over the same directory: the first read in each is a full
    // read, so the two answers are comparable.
    let over_mcp = case.register("example-org-a", &[Verb::Show]);
    let over_cli = case.register("example-org-b", &[Verb::Show]);
    case.serve();

    let responses = mcp(
        &mut case.tool(&over_mcp),
        &[
            initialize(),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "show", "arguments": {"path": "a.txt", "lines": "2:2"}
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "search", "arguments": {"pattern": "alpha"}
            }}),
        ],
    );
    let printed = case
        .tool(&over_cli)
        .args(["show", "a.txt", "--lines", "2:2"])
        .output()
        .expect("run tool show");

    assert!(printed.status.success(), "{printed:?}");
    let printed = String::from_utf8(printed.stdout).expect("tool show prints text");
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["content"][0]["type"], "text");
    assert_eq!(result["content"][0]["text"], printed.as_str());
    assert!(printed.starts_with("beta\n"), "{printed}");
    let refused = &responses[2]["result"];
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["content"][0]["text"], "NotGranted: tools: search\n");
}

#[test]
fn tool_show_exits_1_with_one_line_on_a_refusal() {
    let mut case = Case::new("refusal");
    let socket = case.register("example-org-a", &[Verb::Diff]);
    case.serve();

    let output = case
        .tool(&socket)
        .args(["show", "a.txt"])
        .output()
        .expect("run tool show");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "NotGranted: tools: show\n"
    );
}

#[test]
fn tool_without_a_broker_says_so_on_one_line() {
    let case = Case::new("absent");
    let socket = case.root.join("state/broker/sockets/nobody.sock");

    let output = case
        .tool(&socket)
        .args(["show", "a.txt"])
        .output()
        .expect("run tool show");

    assert_eq!(output.status.code(), Some(1));
    let said = String::from_utf8_lossy(&output.stdout);
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(said.starts_with("UnknownSession: "), "{said}");
}
