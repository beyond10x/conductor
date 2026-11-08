//! The instance API's agent routes (`story:agent-kinds`, design architecture-next § 5.1), wired by
//! `story:federated-views`; the daemon proxies them as `/v1/instances/{name}/agents/{agent}/...`:
//!
//! - `POST agents/{name}/deliver`, body `{"text": "..."}`: [`Runtime::deliver`];
//! - `POST agents/{name}/restart`, body `{}`: [`Runtime::restart`], answering the agent's state
//!   and generation;
//! - `GET agents/{name}/attach`: a PTY stream over HTTP is not served yet; an agent whose routes
//!   ([`Runtime::routes`]) offer attach is answered `501` naming `conductor agent attach`, the way
//!   to attach on the host, and any other is refused as the runtime refuses it.
//!
//! A write is taken as every write of the instance API is: with the instance's token and a JSON
//! body ([`super::answers::admitted`]). A refusal is answered with the specification's error name
//! (`conductor.agent.<Error>`) and the runtime's line.

use std::io;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Server, answers, response};
use crate::agent::{AgentName, Context, Refused, Route, Runtime};

/// The prefix of the agent routes.
pub(super) const PREFIX: &str = "/v1/agents/";

/// The report sink of the instance runtime's agents: each line on standard output, the runtime's
/// journal.
pub(super) fn print(agent: &AgentName, line: &str) -> anyhow::Result<()> {
    println!("agent {}: {line}", agent.0);
    Ok(())
}

/// The body of `deliver`.
#[derive(Deserialize)]
struct Delivery {
    text: String,
}

/// The answer to `method` on `agents/<rest>`.
pub(super) fn answer<'a>(
    method: &str,
    rest: &str,
    header: &impl Fn(&str) -> Option<&'a str>,
    server: &Server,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
) -> Vec<u8> {
    let Some((name, verb)) = rest.split_once('/').filter(|(name, _)| !name.is_empty()) else {
        return json_answer(
            404,
            "Not Found",
            &json!({"error": format!("no route {PREFIX}{rest}")}),
        );
    };
    let allowed = match verb {
        "attach" => "GET",
        "deliver" | "restart" => "POST",
        _ => {
            return json_answer(
                404,
                "Not Found",
                &json!({"error": format!("no route {PREFIX}{rest}")}),
            );
        }
    };
    if method != allowed {
        return json_answer(
            405,
            "Method Not Allowed",
            &json!({"error": format!("{PREFIX}{rest} answers {allowed} only")}),
        );
    }
    let context = Context {
        config: crate::config::explicit(),
        state: server.state.as_deref(),
        providers: &server.providers,
    };
    if verb == "attach" {
        let runtime = lock(server);
        return match runtime.routes(name) {
            Ok(routes) if routes.contains(&Route::Attach) => json_answer(
                501,
                "Not Implemented",
                &json!({
                    "error": format!(
                        "{PREFIX}{name}/attach: a PTY stream over HTTP is not served yet; attach on \
                         the instance's host with conductor agent attach"
                    ),
                    "agent": name,
                }),
            ),
            Ok(_) => refused(&Refused::capability_missing(
                &AgentName(name.to_owned()),
                crate::agent::Capability::Interactive,
            )),
            Err(refusal) => refused(&refusal),
        };
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
    if verb == "deliver" {
        let delivery = match serde_json::from_slice::<Delivery>(&bytes) {
            Ok(delivery) => delivery,
            Err(error) => {
                return json_answer(
                    400,
                    "Bad Request",
                    &json!({"error": format!("the body is not a delivery: {error}; write {{\"text\": \"...\"}}")}),
                );
            }
        };
        return match lock(server).deliver(&context, name, &delivery.text) {
            Ok(()) => json_answer(200, "OK", &json!({"agent": name, "delivered": true})),
            Err(refusal) => refused(&refusal),
        };
    }
    let mut runtime = lock(server);
    match runtime.restart(&context, name) {
        Ok(agent) => json_answer(
            200,
            "OK",
            &json!({
                "agent": name,
                "state": format!("{:?}", agent.state),
                "generation": agent.generation,
            }),
        ),
        Err(refusal) => refused(&refusal),
    }
}

fn lock(server: &Server) -> std::sync::MutexGuard<'_, Runtime> {
    server
        .agents
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A refusal of the runtime, by the specification's error name, with its line.
fn refused(refusal: &Refused) -> Vec<u8> {
    let (status, reason, error) = match refusal {
        Refused::AgentNotFound(_) => (404, "Not Found", "conductor.agent.AgentNotFound"),
        Refused::CapabilityMissing(_) => (409, "Conflict", "conductor.agent.CapabilityMissing"),
        Refused::AgentNotRunning(_) => (409, "Conflict", "conductor.agent.AgentNotRunning"),
        Refused::AgentExists(_) => (409, "Conflict", "conductor.agent.AgentExists"),
        Refused::UnknownAgentKind(_) => (
            422,
            "Unprocessable Content",
            "conductor.agent.UnknownAgentKind",
        ),
        Refused::AgentNotReachable(..) => (502, "Bad Gateway", "conductor.agent.AgentNotReachable"),
        Refused::AgentLaunchFailed(..) => (502, "Bad Gateway", "conductor.agent.AgentLaunchFailed"),
        Refused::Failed(_) => (500, "Internal Server Error", "failed"),
    };
    json_answer(
        status,
        reason,
        &json!({"error": error, "detail": refusal.to_string()}),
    )
}

fn json_answer(status: u16, reason: &str, body: &Value) -> Vec<u8> {
    let mut text = body.to_string();
    text.push('\n');
    response(status, reason, "application/json", text.as_bytes(), "")
}
