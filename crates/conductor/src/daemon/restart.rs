//! `POST /v1/instances/{name}/restart` (`conductor.instance.RestartInstance`, design A6): the
//! instance's runtime stopped and started again on the binary it runs, as the reconciler restarts
//! one, recorded as one `Reconciliation` of action `Restart` for `OperatorCommand` by the request's
//! actor: a person or the control plane (decision D2).
//!
//! The daemon's API thread hands a request to the reconciling loop with [`post`], over the
//! channel [`listen`] opened; the loop answers it with [`Daemon::answer_restart`]. The answers:
//! - `202` `{"name", "action": "Restart", "result": "Succeeded", "actor"}`: restarted;
//! - `404` `conductor.instance.UnknownInstance`: the config names no such instance;
//! - `409` `conductor.instance.InstanceStateConflict`: the daemon started no runtime for it, or
//!   it is upgrading; only a running or unreachable instance is restarted;
//! - `502` `conductor.instance.PlacementRefused`: the runtime did not start again, recorded
//!   `Refused` with why; nothing falls back;
//! - `503` while the loop takes no restart or holds a record the store refused.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use conductor_model::operator::Actor;
use serde_json::{Value, json};

use super::reconcile::{Binary, Daemon, Started};
use super::units::Units;

/// How long the API waits for the loop to answer a restart.
const ANSWER_WAIT: Duration = Duration::from_secs(120);

/// One `POST /v1/instances/{name}/restart`, handed from the API thread to the loop.
#[derive(Debug)]
pub struct Request {
    /// The instance.
    pub name: String,
    /// Who asks.
    pub actor: Actor,
    /// Where the loop sends the answer: the status line and the JSON body.
    pub reply: Sender<(&'static str, Value)>,
}

/// Where [`post`] hands a request: the sender of the channel [`listen`] opened last.
static QUEUE: Mutex<Option<Sender<Request>>> = Mutex::new(None);

/// Opens the channel [`post`] hands requests over; the loop reads the answer's end.
pub fn listen() -> Receiver<Request> {
    let (sender, receiver) = mpsc::channel();
    if let Ok(mut queue) = QUEUE.lock() {
        *queue = Some(sender);
    }
    receiver
}

/// Hands the restart of `name` for `actor` to the loop and answers what the loop answered; `503`
/// when no loop listens, `504` when it does not answer in time.
#[must_use]
pub fn post(name: &str, actor: Actor) -> (&'static str, Value) {
    let (reply, answer) = mpsc::channel();
    let request = Request {
        name: name.to_owned(),
        actor,
        reply,
    };
    let sent = QUEUE
        .lock()
        .ok()
        .and_then(|queue| queue.as_ref().map(|queue| queue.send(request).is_ok()));
    if sent != Some(true) {
        return (
            "503 Service Unavailable",
            json!({"error": "the daemon's loop takes no restart"}),
        );
    }
    answer.recv_timeout(ANSWER_WAIT).unwrap_or_else(|_| {
        (
            "504 Gateway Timeout",
            json!({"error": "the daemon's loop did not answer the restart"}),
        )
    })
}

impl<U: Units> Daemon<U> {
    /// Answers one request [`post`] handed over: while a record the store refused is held,
    /// `503` and nothing done, since nothing would be recorded; otherwise the restart.
    pub fn answer_restart(&mut self, request: Request) {
        let reply = if self.holds_records() {
            (
                "503 Service Unavailable",
                json!({"error": super::metrics::UNRECORDED}),
            )
        } else {
            self.restart(&request.name, &request.actor)
        };
        let _ = request.reply.send(reply);
    }

    /// Restarts the runtime of `name` for `actor`, as the module documentation says; answers the
    /// status line and the JSON body.
    pub fn restart(&mut self, name: &str, actor: &Actor) -> (&'static str, Value) {
        let Some(instance) = self
            .config()
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .cloned()
        else {
            return (
                "404 Not Found",
                json!({"error": "conductor.instance.UnknownInstance", "name": name}),
            );
        };
        let upgrading = self
            .pins
            .get(name)
            .is_some_and(|pinned| pinned.upgrading.is_some());
        let Some(held) = self.started.get(name).cloned().filter(|_| !upgrading) else {
            return (
                "409 Conflict",
                json!({
                    "error": "conductor.instance.InstanceStateConflict",
                    "name": name,
                    "detail": format!(
                        "{name} is not running, or is upgrading: only a running or unreachable \
                         instance is restarted; nothing changed"
                    ),
                }),
            );
        };
        let failed = |error: anyhow::Error| {
            (
                "500 Internal Server Error",
                json!({"error": format!("{error:#}")}),
            )
        };
        let token = match crate::dashboard::mint() {
            Ok(token) => token,
            Err(error) => {
                return failed(anyhow::Error::new(error).context("mint the runtime's token"));
            }
        };
        let binary = Binary {
            path: PathBuf::from(&held.binary),
            version: held.version.clone(),
        };
        let service = self.service(&instance, &binary, &token);
        let outcome = self
            .stop_runtime(name, &held.unit)
            .and_then(|()| self.start_runtime(&instance, &service));
        let (result, detail) = match outcome {
            Ok(unit) => {
                self.started.insert(
                    name.to_owned(),
                    Started {
                        unit,
                        placement: instance.placement.0.clone(),
                        binary: held.binary.clone(),
                        version: held.version.clone(),
                        last_seen: crate::records::now().ok().map(|now| now.0),
                        token: Some(token),
                    },
                );
                self.running.insert(name.to_owned(), true);
                (ReconcileResult::Succeeded, None)
            }
            Err(error) => {
                self.started.remove(name);
                self.running.insert(name.to_owned(), false);
                (ReconcileResult::Refused, Some(format!("{error:#}")))
            }
        };
        let recorded = self.put(
            name,
            ReconcileAction::Restart,
            ReconcileReason::OperatorCommand,
            result,
            detail.clone(),
            actor,
        );
        let kept = self.save();
        if let Err(error) = recorded.and(kept) {
            return failed(error);
        }
        match detail {
            None => (
                "202 Accepted",
                json!({
                    "name": name,
                    "action": "Restart",
                    "result": "Succeeded",
                    "actor": crate::actor::written(actor),
                }),
            ),
            Some(detail) => (
                "502 Bad Gateway",
                json!({
                    "error": "conductor.instance.PlacementRefused",
                    "name": name,
                    "detail": detail,
                }),
            ),
        }
    }
}
