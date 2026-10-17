//! The broker of one conductor instance (`story:tool-broker-read`, design
//! docs/design/roles-and-surfaces.md § 6): the `conductor-broker` component of
//! `spec/components.yaml`, over the domain `conductor.tool` (`spec/domains/tool.yaml`).
//!
//! - **Registry.** A session the broker serves is a registration, `sessions/<session>.json` under
//!   the broker's directory (`<state>/broker`): its instance, its working directory and its
//!   grant. [`register`] writes it and asks a serving broker to open the session's socket;
//!   [`unregister`] removes it. A call reads the registration afresh, so a session that stopped is
//!   refused `UnknownSession` on its next call.
//! - **Identity.** Each session has its own socket, `sockets/<session>.sock`; the broker knows the
//!   caller by the socket a call comes in on, never by what the call says.
//! - **Check, run, record.** [`Broker`] checks each call against the session's grant, runs the
//!   verb, answers, and hands the call to its [`Recorder`], refused or not.
//! - **Answers.** A success is the verb's result, bounded by the grant's `max_output_bytes`, then
//!   one summary line; a failure at most [`FAILURE_LINES`] lines, the last naming a log under
//!   `logs/` that holds all of it ([`failure`]); a refusal one line, `<Error>: <rule>`.
//!
//! The client half, [`call`], is all the `tool` binary uses: it holds no store, no grant and no
//! registry.

#![forbid(unsafe_code)]

mod grant;
mod registry;
mod server;
mod verbs;
mod wire;

pub use conductor_model::tool::{CallOutcome, Verb};
pub use grant::SessionGrant;
pub use registry::{Registration, broker_dir, register, socket_path, unregister};
pub use server::{Broker, CallRecord, MemoryRecorder, Recorder};
pub use verbs::{FAILURE_LINES, failure};
pub use wire::{Answer, Request, call, describe};

/// The environment variable that names a session's socket to `tool`.
pub const SOCKET_VARIABLE: &str = "TOOL_SOCKET";

/// A verb as the wire and the config file write it.
#[must_use]
pub fn verb_word(verb: Verb) -> &'static str {
    match verb {
        Verb::Show => "show",
        Verb::Search => "search",
        Verb::Diff => "diff",
    }
}

/// The verb a word names, if any.
#[must_use]
pub fn verb_named(word: &str) -> Option<Verb> {
    [Verb::Show, Verb::Search, Verb::Diff]
        .into_iter()
        .find(|verb| verb_word(*verb) == word)
}

/// A refusal outcome as its error's name.
#[must_use]
pub fn outcome_word(outcome: CallOutcome) -> &'static str {
    match outcome {
        CallOutcome::Done => "Done",
        CallOutcome::Failed => "Failed",
        CallOutcome::NotGranted => "NotGranted",
        CallOutcome::GrantExpired => "GrantExpired",
        CallOutcome::ForeignInstance => "ForeignInstance",
        CallOutcome::UnknownSession => "UnknownSession",
    }
}
