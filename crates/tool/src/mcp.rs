//! `tool mcp`: the session's granted verbs as MCP tools, one per verb, over stdio — one JSON-RPC
//! message per line — each call answered through the same socket as the command line, so
//! `tools/call show` answers exactly what `tool show` prints. `tools/list` asks the broker for the
//! session's grant each time, so it lists exactly the granted verbs.

use std::io::{self, BufRead, Write};
use std::path::Path;

use conductor_broker::{Answer, Request, call, describe};
use serde_json::{Value, json};

/// The protocol revision this server answers when the client names none.
const PROTOCOL: &str = "2025-06-18";

/// Serves MCP on `input` and `output` until `input` ends.
///
/// # Errors
///
/// When `input` cannot be read or `output` written.
pub fn serve(socket: &Path, input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(message) => respond(socket, &message),
            Err(error) => Some(failure(
                &Value::Null,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The response to one message; none to a notification.
fn respond(socket: &Path, message: &Value) -> Option<Value> {
    let id = message.get("id")?.clone();
    let method = message["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => json!({
            "protocolVersion": message["params"]["protocolVersion"].as_str().unwrap_or(PROTOCOL),
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "tool", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "The verbs of this session's grant, each answered by the instance's broker. Each answer ends in one summary line; a failure is at most 40 lines and names its log.",
        }),
        "ping" => json!({}),
        "tools/list" => match describe(socket) {
            Ok(verbs) => {
                json!({"tools": verbs.iter().filter_map(|verb| tool(verb)).collect::<Vec<_>>()})
            }
            Err(answer) => return Some(failure(&id, -32000, answer.text.trim_end())),
        },
        "tools/call" => {
            let name = message["params"]["name"].as_str().unwrap_or_default();
            let args = match &message["params"]["arguments"] {
                Value::Object(map) => Value::Object(map.clone()),
                _ => json!({}),
            };
            let answer = call(
                socket,
                &Request {
                    verb: name.to_owned(),
                    args,
                },
            );
            content(&answer)
        }
        _ => return Some(failure(&id, -32601, &format!("method not found: {method}"))),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

fn content(answer: &Answer) -> Value {
    json!({
        "content": [{"type": "text", "text": answer.text}],
        "isError": answer.exit != 0,
    })
}

fn failure(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// The schema of a build verb's `package` argument.
fn package() -> Value {
    json!({"type": "string", "description": "The Cargo package (cargo's -p); none, the working directory's own"})
}

/// The MCP tool of one verb: its name, what it does and its arguments' schema
/// (`conductor.tool.ShowCall`, `SearchCall`, `DiffCall`, `TestCall`, `CheckCall`, `FmtCall`,
/// `CommitCall`, `SendCall`, `ReportCall`).
fn tool(verb: &str) -> Option<Value> {
    let (description, properties, required) = match verb {
        "show" => (
            "Print a bounded range of one file of the grant, or that it is unchanged since this session last read it",
            json!({
                "path": {"type": "string", "description": "The file, relative to the session's working directory"},
                "lines": {"type": "string", "description": "The lines a:b, 1-based, both ends included; either end may be left out"},
                "symbol": {"type": "string", "description": "The item that names this symbol, from its line to the end of its block"},
                "max": {"type": "integer", "minimum": 1, "description": "The most bytes of the answer, at most the grant's limit"},
            }),
            json!(["path"]),
        ),
        "search" => (
            "Print the lines under the grant's paths that hold a text",
            json!({
                "pattern": {"type": "string", "description": "The text, matched as written"},
                "path": {"type": "string", "description": "The file or directory to search under, relative to the session's working directory"},
            }),
            json!(["pattern"]),
        ),
        "diff" => (
            "Print the uncommitted change of the session's checkout, as a stat (the default), or with patch the change itself",
            json!({
                "stat": {"type": "boolean", "description": "A stat of the change (the default)"},
                "patch": {"type": "boolean", "description": "The change itself, bounded as every answer is"},
            }),
            json!([]),
        ),
        "test" => (
            "Run the tests of the session's checkout through the host ledger at nice 19; answers `ok test <n> passed`, or the failing tests with their first assertion lines and the log",
            json!({
                "package": package(),
                "filter": {"type": "string", "description": "Only the tests whose name holds this text"},
            }),
            json!([]),
        ),
        "check" => (
            "Lint the session's checkout (clippy over every target, warnings denied) through the host ledger at nice 19; answers `ok check`, or the errors and the log",
            json!({"package": package()}),
            json!([]),
        ),
        "fmt" => (
            "Format the session's checkout through the host ledger at nice 19; answers `ok fmt <n> files changed`",
            json!({"package": package()}),
            json!([]),
        ),
        "commit" => (
            "Stage the named paths, new ones included, and commit them through the bot route; answers `<sha> <n> files`",
            json!({
                "message": {"type": "string", "description": "The commit message"},
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 1,
                    "description": "The paths to commit, each relative to the session's working directory and under the grant's paths",
                },
            }),
            json!(["message", "paths"]),
        ),
        "send" => (
            "Send a message to a session of the caller's team, by its name or its role; the broker records it and answers whom to deliver it to",
            json!({
                "to": {"type": "string", "description": "The recipient: a session of the caller's team, by its session name or its role"},
                "text": {"type": "string", "description": "The message"},
            }),
            json!(["to", "text"]),
        ),
        "report" => (
            "Report the caller's status one level up, moving its open dispatch when it is a lead; the broker records it and answers whom to deliver it to",
            json!({
                "status": {
                    "type": "string",
                    "enum": ["started", "progress", "blocked", "unblocked", "done", "failed"],
                    "description": "What the report says of the caller's work",
                },
                "line": {"type": "string", "description": "One line: the reason, the evidence or the progress"},
                "to": {"type": "string", "description": "The session the report goes to; it must be the one one level above the caller"},
            }),
            json!(["status", "line"]),
        ),
        _ => return None,
    };
    Some(json!({
        "name": verb,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    }))
}
