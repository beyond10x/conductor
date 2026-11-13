//! The work call of the instance API (`story:control-plane-work-handler`), which conductord
//! forwards as `/v1/instances/{name}/work` ([`crate::daemon::api`]):
//!
//! - `POST /v1/work`, body `conductor.dispatch.WorkRequest` as JSON: one unit of work sent as a
//!   dispatch with the caller's `reference` and no goal, through the code `dispatch send-dispatch
//!   --reference` runs ([`crate::dispatch::send_work`]), with the priority that command defaults
//!   to. A dispatch it created it starts the repository's controller for at once, under the same
//!   one write at a time, as the loop's cycle starts one ([`crate::run::start_work`]: no second
//!   controller beside a running one, `controllers.max_working`, then `conductor spawn`'s code with
//!   the default charter where the repository has none and the brief in the session's first
//!   prompt), without waiting for the session; a refused start is the row's `start` and the
//!   dispatch's `start-refused` line, and the next cycle tries again. It answers the unit's
//!   `conductor.dispatch.WorkState` as one JSON object: `201` for a
//!   dispatch it created, whether its start was made or refused; `200` for a reference a dispatch
//!   already holds, starting nothing, whatever the other
//!   fields say, creating nothing; `409` `RepositoryBusy` with the open dispatches (`open`) of a
//!   repository that works another; `422` `InvalidWorkRequest` (no JSON object, a member the
//!   request does not declare, one it declares missing or of another type, a `start_ref` or an
//!   `issue` a dispatch does not carry, [`crate::dispatch::refused_member`]), `InvalidReference`,
//!   `InvalidBrief` (a brief of 1 to [`BRIEF_MOST`] bytes is taken) or `UnknownRepository` (one
//!   neither the newest complete snapshot nor the workspace file names,
//!   [`crate::dispatch::named_repositories`]). It is taken as every write of the instance API is,
//!   with the instance's token and as JSON, and one write at a time; its body is at most
//!   [`MAX_BODY`], refused `413` from its `Content-Length` before it is read. The dispatch is the
//!   control plane's when the request carries [`ACTOR_KIND_HEADER`] `control-plane`, read only
//!   on a request that holds the token, which only the daemon holds; else the operator
//!   `X-Conductor-Operator` names, else the Unix user's ([`crate::actor::as_operator`]).
//! - `GET /v1/work/{reference}`: the same row ([`crate::dispatch::work_state`]), `404`
//!   `WorkNotFound` for a reference no dispatch holds, `422` `InvalidReference` for one that is no
//!   reference. A read, as the instance API's other reads.
//!
//! Every refusal is a JSON object naming the specification's error, `conductor.dispatch.<Error>`,
//! and its one line in `detail`. A store or records that fail answer `500`
//! `conductor.dispatch.WorkFailed` with the reference and nothing of the failure; the error's
//! whole chain, with its paths, is the runtime's one line on standard error.

use std::io;
use std::sync::PoisonError;

use conductor_model::dispatch::{WorkRequest, WorkStartRefusal};
use conductor_model::server::{json as wire_json, wire};
use serde_json::{Value, json};

use super::{Server, answers, response};
use crate::dispatch::{self, Sent};

/// The path of the write.
pub(super) const PATH: &str = "/v1/work";

/// The prefix of the read: `/v1/work/{reference}`.
pub(super) const PREFIX: &str = "/v1/work/";

/// The longest body the write takes, 1 MiB.
pub const MAX_BODY: usize = 1024 * 1024;

/// The longest brief taken, in bytes.
pub const BRIEF_MOST: usize = 65_536;

/// The header the daemon names a control plane in, on a write it admitted by the control token.
pub const ACTOR_KIND_HEADER: &str = "x-conductor-actor-kind";

/// The value of [`ACTOR_KIND_HEADER`] that names the control plane.
pub const CONTROL_PLANE: &str = "control-plane";

/// The answer to `method` on `path`, [`PATH`] or under [`PREFIX`].
pub(super) fn answer<'a>(
    method: &str,
    path: &str,
    header: &impl Fn(&str) -> Option<&'a str>,
    server: &Server,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
) -> Vec<u8> {
    if let Some(reference) = path.strip_prefix(PREFIX) {
        if method != "GET" {
            return refusal(405, "Method Not Allowed", "only GET", "Allow: GET\r\n");
        }
        return read(server, reference);
    }
    if method != "POST" {
        return refusal(405, "Method Not Allowed", "only POST", "Allow: POST\r\n");
    }
    let length = match answers::admitted_within(header, &server.token, MAX_BODY) {
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
        return refusal(
            400,
            "Bad Request",
            "the body ended before its Content-Length",
            "",
        );
    };
    let control = header(ACTOR_KIND_HEADER)
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case(CONTROL_PLANE));
    let _one = server.writes.lock().unwrap_or_else(PoisonError::into_inner);
    let actor = if control {
        crate::actor::control_plane()
    } else {
        crate::actor::as_operator(header(answers::OPERATOR_HEADER), crate::actor::human)
    };
    let answer = send(server, &bytes, actor);
    server.answers.clear();
    server.facts.clear();
    server.unanswered.clear();
    server.roster.clear();
    answer
}

/// `POST /v1/work` with `bytes` as `actor`, as the module documentation says.
fn send(server: &Server, bytes: &[u8], actor: conductor_model::operator::Actor) -> Vec<u8> {
    const SAID: &str = "POST /v1/work";
    let state = server.state.as_deref();
    let work = match request(bytes) {
        Ok(work) => work,
        Err(detail) => return refused(422, "InvalidWorkRequest", &json!({"detail": detail})),
    };
    if !dispatch::valid_reference(&work.reference) {
        return invalid_reference(&work.reference);
    }
    let failure = |error: &anyhow::Error| failed(SAID, &work.reference, error);
    match dispatch::held_by(state, &work.reference) {
        Ok(Some(_)) => return row(server, &work.reference, 200, "OK", SAID),
        Ok(None) => {}
        Err(error) => return failure(&error),
    }
    if work.brief.is_empty() || work.brief.len() > BRIEF_MOST {
        return refused(
            422,
            "InvalidBrief",
            &json!({
                "reference": work.reference,
                "detail": format!(
                    "the brief is {} bytes; a brief is 1 to {BRIEF_MOST} bytes, so nothing was sent",
                    work.brief.len()
                ),
            }),
        );
    }
    if let Some((member, rule)) =
        dispatch::refused_member(work.start_ref.as_deref(), work.issue.as_deref())
    {
        return refused(
            422,
            "InvalidWorkRequest",
            &json!({"detail": format!("{member} is not taken: {rule}; nothing was sent")}),
        );
    }
    match dispatch::named_repositories(state) {
        Ok(named) if named.contains(&work.repository.0) => {}
        Ok(named) => {
            return refused(
                422,
                "UnknownRepository",
                &json!({
                    "repository": work.repository.0,
                    "detail": format!(
                        "neither the newest complete snapshot nor the workspace names {}; they \
                         name {}; nothing was sent",
                        work.repository.0,
                        if named.is_empty() {
                            "none".to_owned()
                        } else {
                            named.into_iter().collect::<Vec<_>>().join(", ")
                        }
                    ),
                }),
            );
        }
        Err(error) => return failure(&error),
    }
    match dispatch::send_work(state, &work, actor) {
        Ok(Sent::Created(id)) => {
            start(server, &work.repository.0, &id);
            row(server, &work.reference, 201, "Created", SAID)
        }
        Ok(Sent::Held(_)) => row(server, &work.reference, 200, "OK", SAID),
        Ok(Sent::Refused {
            error: "RepositoryBusy",
            detail,
            open,
        }) => refused(
            409,
            "RepositoryBusy",
            &json!({"repository": work.repository.0, "open": open, "detail": detail}),
        ),
        Ok(Sent::Refused { error, detail, .. }) => refused(
            422,
            error,
            &json!({"reference": work.reference, "detail": detail}),
        ),
        Err(error) => failure(&error),
    }
}

/// Starts `repository`'s controller for the dispatch `id` the call created, as the loop's cycle
/// starts one ([`crate::run::start_work`]), through the runtime's providers and with the config
/// file its `--config` named; a refused start is kept as the dispatch's `start-refused` line, which
/// the row reads. What failed is the runtime's line on standard error; the call goes on.
fn start(server: &Server, repository: &str, id: &str) {
    let Some((refusal, detail)) = crate::run::start_work(
        crate::config::explicit(),
        server.state.as_deref(),
        &server.providers,
        repository,
        id,
    ) else {
        return;
    };
    let name = crate::run::work_refusal_name(refusal);
    if refusal == WorkStartRefusal::StartFailed {
        eprintln!("conductor instance serve: POST /v1/work: the start of {id} failed: {detail}");
    }
    if let Err(error) = crate::run::start_refused_line(repository, id, refusal, &detail)
        .and_then(|line| crate::records::append_dispatch(&line))
    {
        eprintln!(
            "conductor instance serve: POST /v1/work: the start of {id} was refused ({name}) and \
             its line not written: {error:#}"
        );
    }
}

/// The work request `bytes` holds, or why it holds none.
fn request(bytes: &[u8]) -> Result<WorkRequest, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the body is no UTF-8 text".to_owned())?;
    let value = wire_json::parse(text).map_err(|error| format!("the body is no JSON: {error}"))?;
    wire::decode_conductor_dispatch_work_request(&value, "").map_err(|error| {
        format!(
            "the body is no work request {{\"repository\", \"brief\", \"reference\", \
             \"start_ref\"?, \"issue\"?}}: {error}"
        )
    })
}

/// `GET /v1/work/{reference}`.
fn read(server: &Server, reference: &str) -> Vec<u8> {
    if !dispatch::valid_reference(reference) {
        return invalid_reference(reference);
    }
    row(
        server,
        reference,
        200,
        "OK",
        &format!("GET {PREFIX}{reference}"),
    )
}

/// The row of `reference` answered `status`, or `404` when no dispatch holds it; `said` is the
/// request, which a failure's log line names.
fn row(server: &Server, reference: &str, status: u16, reason: &str, said: &str) -> Vec<u8> {
    match dispatch::work_state(server.state.as_deref(), &server.sources.root, reference) {
        Ok(Some(state)) => {
            let mut text = String::new();
            wire::encode_conductor_dispatch_work_state(&state, &mut text);
            text.push('\n');
            response(status, reason, "application/json", text.as_bytes(), "")
        }
        Ok(None) => refused(
            404,
            "WorkNotFound",
            &json!({
                "reference": reference,
                "detail": format!("no dispatch of this instance holds the reference {reference}"),
            }),
        ),
        Err(error) => failed(said, reference, &error),
    }
}

/// The `422` of a reference that is no reference.
fn invalid_reference(reference: &str) -> Vec<u8> {
    refused(
        422,
        "InvalidReference",
        &json!({
            "reference": reference,
            "detail": format!(
                "a reference is 1 to {} characters of A-Z a-z 0-9 . _ - led by a letter or a \
                 digit",
                dispatch::REFERENCE_MOST
            ),
        }),
    )
}

/// The refusal `error` of `conductor.dispatch` answered `status`, with the members of `fields`.
fn refused(status: u16, error: &str, fields: &Value) -> Vec<u8> {
    let mut body = json!({"error": format!("conductor.dispatch.{error}")});
    if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
        body.extend(fields.clone());
    }
    let reason = match status {
        404 => "Not Found",
        409 => "Conflict",
        _ => "Unprocessable Content",
    };
    let mut text = body.to_string();
    text.push('\n');
    response(status, reason, "application/json", text.as_bytes(), "")
}

/// A refusal that names no error of the specification: the method or the body.
fn refusal(status: u16, reason: &str, why: &str, extra: &str) -> Vec<u8> {
    let mut text = json!({"error": why}).to_string();
    text.push('\n');
    response(status, reason, "application/json", text.as_bytes(), extra)
}

/// A failure of the store or the records on the request `said` for `reference`: `500`
/// `conductor.dispatch.WorkFailed`, naming nothing of the failure, and the runtime's one line on
/// standard error with the whole chain, which names the store's and the records' paths.
fn failed(said: &str, reference: &str, error: &anyhow::Error) -> Vec<u8> {
    eprintln!("conductor instance serve: {said} answered 500: {error:#}");
    let mut text = json!({
        "error": "conductor.dispatch.WorkFailed",
        "reference": reference,
        "detail": "the instance could not read or write its store or its records; its log says why",
    })
    .to_string();
    text.push('\n');
    response(
        500,
        "Internal Server Error",
        "application/json",
        text.as_bytes(),
        "",
    )
}
