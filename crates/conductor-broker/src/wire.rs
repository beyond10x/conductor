//! One call over a session's socket: a request line, `{"verb": <word>, "args": {…}}`, and an
//! answer line, `{"text": <what tool prints>, "exit": <its status>}` (`conductor.tool.CallAnswer`).
//! The connection carries one call and closes.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde_json::{Value, json};

/// One call: the verb's word and its arguments (`conductor.tool.ShowCall`, `SearchCall`,
/// `DiffCall`). The word `grant` asks for the session's granted verbs and is no verb: it runs
/// nothing and is not recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub verb: String,
    pub args: Value,
}

/// What the broker answers: the text `tool` prints and the status it exits with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub exit: i32,
}

impl Answer {
    pub(crate) fn done(text: String) -> Self {
        Self { text, exit: 0 }
    }

    pub(crate) fn refused(error: &str, detail: &str) -> Self {
        Self {
            text: format!("{error}: {detail}\n"),
            exit: 1,
        }
    }

    pub(crate) fn to_line(&self) -> String {
        json!({"text": self.text, "exit": self.exit}).to_string()
    }

    fn from_line(line: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(line).ok()?;
        Some(Self {
            text: value["text"].as_str()?.to_owned(),
            exit: i32::try_from(value["exit"].as_i64()?).ok()?,
        })
    }
}

impl Request {
    pub(crate) fn to_line(&self) -> String {
        json!({"verb": self.verb, "args": self.args}).to_string()
    }

    pub(crate) fn from_line(line: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(line).ok()?;
        Some(Self {
            verb: value["verb"].as_str()?.to_owned(),
            args: match value.get("args") {
                Some(Value::Object(map)) => Value::Object(map.clone()),
                _ => json!({}),
            },
        })
    }
}

/// Sends `request` over the socket `socket` and answers the broker's reply. A socket no broker
/// answers on is answered as the broker would answer a session it does not know:
/// `UnknownSession`, on one line, exit status 1.
#[must_use]
pub fn call(socket: &Path, request: &Request) -> Answer {
    let unknown = |why: String| {
        Answer::refused(
            "UnknownSession",
            &format!("no broker answers on {}: {why}", socket.display()),
        )
    };
    let mut stream = match UnixStream::connect(socket) {
        Ok(stream) => stream,
        Err(error) => return unknown(error.to_string()),
    };
    if let Err(error) = writeln!(stream, "{}", request.to_line()) {
        return unknown(error.to_string());
    }
    let mut line = String::new();
    match BufReader::new(stream).read_line(&mut line) {
        Ok(_) => Answer::from_line(&line)
            .unwrap_or_else(|| unknown("the broker's answer is not an answer".to_owned())),
        Err(error) => unknown(error.to_string()),
    }
}

/// The verbs the session of `socket` is granted, as their words, or the refusal that says why
/// there are none.
///
/// # Errors
///
/// The broker's refusal, as `tool` would print it.
pub fn describe(socket: &Path) -> Result<Vec<String>, Answer> {
    let answer = call(
        socket,
        &Request {
            verb: "grant".to_owned(),
            args: json!({}),
        },
    );
    if answer.exit != 0 {
        return Err(answer);
    }
    Ok(answer.text.split_whitespace().map(str::to_owned).collect())
}
