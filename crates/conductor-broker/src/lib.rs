//! The broker of one conductor instance (`story:tool-broker-read`, `story:tool-build-commit-report`,
//! design docs/design/roles-and-surfaces.md § 6): the `conductor-broker` component of
//! `spec/components.yaml`, over the domain `conductor.tool` (`spec/domains/tool.yaml`).
//!
//! - **Registry.** A session the broker serves is a registration, `sessions/<session>.json` under
//!   the broker's directory (`<state>/broker`): its instance, its working directory, its grant and
//!   its place (role, level, team, the session above it). [`register`] writes it and asks a
//!   serving broker to open the session's socket; [`unregister`] removes it. A call reads the
//!   registration afresh, so a session that stopped is refused `UnknownSession` on its next call.
//! - **Identity.** Each session has its own socket, `sockets/<session>.sock`; the broker knows the
//!   caller by the socket a call comes in on, never by what the call says.
//! - **Check, run, record.** [`Broker`] checks each call against the session's grant, runs the
//!   verb, answers, and hands the call to its [`Recorder`], refused or not.
//! - **Services.** What a verb needs that this crate does not hold comes in [`Services`]: the host
//!   ledger the build verbs run through ([`Ledger`]), the bot route `commit` commits through with
//!   the instance's Gates policy ([`BotRoute`]), and the store `send` and `report` record their
//!   messages in ([`Messages`]). `conductor broker serve` gives the instance's own.
//! - **Answers.** A success is the verb's result, bounded by the grant's `max_output_bytes`, then
//!   one summary line; a failure at most [`FAILURE_LINES`] lines, the last naming a log under
//!   `logs/` that holds all of it ([`failure`]); a refusal one line, `<Error>: <rule>`.
//!
//! The client half, [`call`], is all the `tool` binary uses: it holds no store, no grant and no
//! registry.

#![forbid(unsafe_code)]

mod build;
mod commit;
mod grant;
mod registry;
mod server;
mod services;
mod team;
mod verbs;
mod wire;

pub use conductor_model::tool::{CallOutcome, Level, ReportStatus, Verb};
pub use grant::SessionGrant;
pub use registry::{Placement, Registration, broker_dir, register, socket_path, unregister};
pub use server::{Broker, CallRecord, MemoryRecorder, Recorder};
pub use services::{
    BotRoute, BuildEnd, BuildJob, Ledger, Messages, Report, Reported, Services, TeamMessage,
};
pub use verbs::{FAILURE_LINES, failure};
pub use wire::{Answer, Request, call, describe};

/// The environment variable that names a session's socket to `tool`.
pub const SOCKET_VARIABLE: &str = "TOOL_SOCKET";

/// Every verb, in the order the design lists them (§ 6.3): the one list every reader of verbs
/// (the config file, the store, `tool mcp`) walks. [`verb_word`] is exhaustive, so a verb the
/// specification adds fails to compile there, and `tests/verbs.rs` holds this list to it.
pub const VERBS: [Verb; 9] = [
    Verb::Show,
    Verb::Search,
    Verb::Diff,
    Verb::Test,
    Verb::Check,
    Verb::Fmt,
    Verb::Commit,
    Verb::Send,
    Verb::Report,
];

/// A verb as the wire and the config file write it.
#[must_use]
pub fn verb_word(verb: Verb) -> &'static str {
    match verb {
        Verb::Show => "show",
        Verb::Search => "search",
        Verb::Diff => "diff",
        Verb::Test => "test",
        Verb::Check => "check",
        Verb::Fmt => "fmt",
        Verb::Commit => "commit",
        Verb::Send => "send",
        Verb::Report => "report",
    }
}

/// The verb a word names, if any.
#[must_use]
pub fn verb_named(word: &str) -> Option<Verb> {
    VERBS.into_iter().find(|verb| verb_word(*verb) == word)
}

/// Every call outcome; [`outcome_word`] is exhaustive, as [`verb_word`] is.
pub const OUTCOMES: [CallOutcome; 8] = [
    CallOutcome::Done,
    CallOutcome::Failed,
    CallOutcome::NotGranted,
    CallOutcome::GrantExpired,
    CallOutcome::ForeignInstance,
    CallOutcome::UnknownSession,
    CallOutcome::OutsideTeam,
    CallOutcome::NotOneLevelUp,
];

/// An outcome as the specification spells it; a refusal's is its error's name.
#[must_use]
pub fn outcome_word(outcome: CallOutcome) -> &'static str {
    match outcome {
        CallOutcome::Done => "Done",
        CallOutcome::Failed => "Failed",
        CallOutcome::NotGranted => "NotGranted",
        CallOutcome::GrantExpired => "GrantExpired",
        CallOutcome::ForeignInstance => "ForeignInstance",
        CallOutcome::UnknownSession => "UnknownSession",
        CallOutcome::OutsideTeam => "OutsideTeam",
        CallOutcome::NotOneLevelUp => "NotOneLevelUp",
    }
}

/// Every report status, in the order a dispatch moves.
pub const STATUSES: [ReportStatus; 6] = [
    ReportStatus::Started,
    ReportStatus::Progress,
    ReportStatus::Blocked,
    ReportStatus::Unblocked,
    ReportStatus::Done,
    ReportStatus::Failed,
];

/// A report's status as the § 6 `[REPORT]` form and the wire write it.
#[must_use]
pub fn status_word(status: ReportStatus) -> &'static str {
    match status {
        ReportStatus::Started => "started",
        ReportStatus::Progress => "progress",
        ReportStatus::Blocked => "blocked",
        ReportStatus::Unblocked => "unblocked",
        ReportStatus::Done => "done",
        ReportStatus::Failed => "failed",
    }
}

/// The status a word names, if any.
#[must_use]
pub fn status_named(word: &str) -> Option<ReportStatus> {
    STATUSES
        .into_iter()
        .find(|status| status_word(*status) == word)
}

/// Every level, from the top.
pub const LEVELS: [Level; 3] = [Level::Director, Level::Lead, Level::Worker];

/// A level as the registration writes it.
#[must_use]
pub fn level_word(level: Level) -> &'static str {
    match level {
        Level::Director => "director",
        Level::Lead => "lead",
        Level::Worker => "worker",
    }
}

/// The level a word names, if any.
#[must_use]
pub fn level_named(word: &str) -> Option<Level> {
    LEVELS.into_iter().find(|level| level_word(*level) == word)
}
