//! The instance API's routes for the operator's session commands run on the host
//! (`story:host-commands-reach-boxed-session`); the daemon proxies them as writes under
//! `/v1/instances/{name}/`:
//!
//! - `POST sessions/{name}/deliver`, body `{"text": "..."}`: `conductor session deliver` in the
//!   runtime ([`crate::session::deliver_here`]), answering `{"delivery_id": "..."}`;
//! - `POST controllers/{id}/stop`, body `{"reason": "..."}`: `conductor controller
//!   stop-controller` in the runtime ([`crate::controller::stop_here`]), answering
//!   `{"stopped": "<session name> (<id>)"}`, or `null` for a controller whose session it touched
//!   none of.
//!
//! A child session in a box lives in the box's pid namespace and reads its input from a FIFO the
//! box's registry names: only a process in the box, the runtime, can signal or feed it. The host
//! commands send these writes when the session runs in the box
//! ([`crate::confine::child::runs_in_box`]); in the runtime the same code takes the local path.
//!
//! A write is taken as every write of the instance API is: with the instance's token and a JSON
//! body ([`super::answers::admitted`]). It is recorded as the operator conductord names in
//! `X-Conductor-Operator`, else the operator the runtime's Unix user names. A refusal of the
//! command is `409` with its one line in `refused`, which the host prints as its own.

use std::io;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Server, answers, response};

/// The prefix of the session routes.
pub(super) const SESSIONS: &str = "/v1/sessions/";

/// The prefix of the controller routes.
pub(super) const CONTROLLERS: &str = "/v1/controllers/";

/// The body of `sessions/{name}/deliver`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delivery {
    text: String,
}

/// The body of `controllers/{id}/stop`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stop {
    reason: String,
}

/// The routes this module answers: `path` is one of them when it is `<prefix><name>/<verb>`.
#[must_use]
pub(super) fn serves(path: &str) -> bool {
    route(path).is_some()
}

/// `path` read as one of the routes: its prefix, the name or id, and the verb.
fn route(path: &str) -> Option<(&'static str, &str)> {
    for (prefix, verb) in [(SESSIONS, "/deliver"), (CONTROLLERS, "/stop")] {
        if let Some(name) = path
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(verb))
            .filter(|name| !name.is_empty() && !name.contains('/'))
        {
            return Some((prefix, name));
        }
    }
    None
}

/// The answer to `method` on `path`, one of the routes ([`serves`]).
pub(super) fn answer<'a>(
    method: &str,
    path: &str,
    header: &impl Fn(&str) -> Option<&'a str>,
    server: &Server,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
) -> Vec<u8> {
    let Some((prefix, name)) = route(path) else {
        return json_answer(
            404,
            "Not Found",
            &json!({"error": format!("no route {path}")}),
        );
    };
    if method != "POST" {
        return json_answer(
            405,
            "Method Not Allowed",
            &json!({"error": format!("{path} answers POST only")}),
        );
    }
    let length = match answers::admitted(header, &server.token) {
        Ok(length) => length,
        Err(why) => {
            return response(
                why.status,
                why.reason,
                "text/plain; charset=utf-8",
                why.why.as_bytes(),
                "",
            );
        }
    };
    let Ok(bytes) = body(length) else {
        return json_answer(
            400,
            "Bad Request",
            &json!({"error": "the body ended before its Content-Length"}),
        );
    };
    let actor = crate::actor::as_operator(header(answers::OPERATOR_HEADER), crate::actor::human);
    let state = server.state.as_deref();
    let done = if prefix == SESSIONS {
        let delivery = match serde_json::from_slice::<Delivery>(&bytes) {
            Ok(delivery) => delivery,
            Err(error) => return not_read(&format!("{error}; write {{\"text\": \"...\"}}")),
        };
        crate::session::deliver_here(state, name, &delivery.text, None, actor)
            .map(|id| json!({"session": name, "delivery_id": id}))
    } else {
        let stop = match serde_json::from_slice::<Stop>(&bytes) {
            Ok(stop) => stop,
            Err(error) => return not_read(&format!("{error}; write {{\"reason\": \"...\"}}")),
        };
        if !uuid(name) {
            return json_answer(
                404,
                "Not Found",
                &json!({"error": format!("{name:?} is no controller id")}),
            );
        }
        let id = conductor_model::dispatch::ControllerId(conductor_model::primitives::Uuid(
            name.to_owned(),
        ));
        let stopped =
            crate::controller::stop_here(state, id, stop.reason, actor, &server.providers);
        server.roster.clear();
        stopped.map(|stopped| json!({"controller_id": name, "stopped": stopped}))
    };
    match done {
        Ok(answer) => json_answer(200, "OK", &answer),
        Err(error) => {
            let line = format!("{error:#}")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            json_answer(409, "Conflict", &json!({"refused": line}))
        }
    }
}

/// Whether `text` is a UUID as the controller ids are written: 8-4-4-4-12 hexadecimal digits.
fn uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length && group.chars().all(|c| c.is_ascii_hexdigit())
        })
}

fn not_read(why: &str) -> Vec<u8> {
    json_answer(
        400,
        "Bad Request",
        &json!({"error": format!("the body does not read: {why}")}),
    )
}

fn json_answer(status: u16, reason: &str, body: &Value) -> Vec<u8> {
    let mut text = body.to_string();
    text.push('\n');
    response(status, reason, "application/json", text.as_bytes(), "")
}
