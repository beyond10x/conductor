//! `conductor controller`: one controller session per repository.
//!
//! Each command records its outcome in the store under the state directory (`--state-dir`, else
//! `state/`; [`crate::state::dir`]) through the generated behaviours, and `controllers` answers
//! from those records. None of them starts, stops or messages a session: launching and ending the
//! session itself is [`crate::spawn`].
//!
//! `start-controller` prints the new controller's id, the one every other command takes; the
//! other commands print nothing. A refusal the specification declares exits 1 with one line on
//! standard error that names its error. Input through `--input-json -` is not written yet.

use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{ControllerStorage, Generated};
use conductor_model::dispatch::obligations::{
    ControllersQuery, PauseControllerBehavior, ResumeControllerBehavior, ReviseCharterBehavior,
    StartControllerBehavior, StopControllerBehavior,
};
use conductor_model::dispatch::{
    AnyController, Controller, ControllerAlreadyRunning, ControllerData, ControllerId,
    ControllerNotFound, ControllerSnapshot, ControllerStarted, ControllerState,
    ControllerStateConflict, PauseController, PauseControllerOutcome, ResumeController,
    ResumeControllerOutcome, ReviseCharter, ReviseCharterOutcome, StartController,
    StartControllerOutcome, StopController, StopControllerOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::primitives::Uuid;
use serde_json::Value;

use crate::cli::{
    JsonInput, PauseControllerArgs, ResumeControllerArgs, ReviseCharterArgs, StartControllerArgs,
    StopControllerArgs, ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// `conductor controller start-controller`: the command `conductor.dispatch.StartController`.
/// Prints the new controller's id.
///
/// The session name is the instance's for the repository ([`crate::config::session_name`]):
/// `<session_prefix>-<repository>`, or the repository without a prefix
/// (`story:instance-session-names`). `--session-name` may name it, and no other.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, `--session-name` is not the instance's name for
/// the repository, the store does not open or keep the record, or the repository already has a
/// running or paused controller (`ControllerAlreadyRunning`).
pub fn start_controller(state: Option<&Path>, args: StartControllerArgs) -> Result<ExitCode> {
    const WHAT: &str = "controller start-controller";
    flags_only(args.input_json, "controller start-controller --input-json")?;
    let repository = required(args.repository, WHAT, "--repository")?;
    let harness = required(args.harness, WHAT, "--harness")?;
    let harness = harness_named(&harness)
        .ok_or_else(|| anyhow!("{WHAT}: --harness {harness:?} is not a Harness"))?;
    let session_name = crate::config::session_name(&crate::config::active().instance, &repository);
    if let Some(given) = args.session_name
        && given != session_name.0
    {
        bail!(
            "{WHAT}: --session-name {given:?} is not this instance's session name for repository \
             {repository:?}: {:?}, <session_prefix>-<repository> of conductor config show, or the \
             repository without a prefix",
            session_name.0
        );
    }
    let mut generated = record(state)?;
    let outcome = generated
        .start_controller(StartController {
            repository: RepositoryName(repository),
            harness,
            session_name,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated
        .ports
        .store
        .check()
        .context("keep the controller")?;
    match outcome {
        StartControllerOutcome::Started { controller_started } => {
            writeln!(
                io::stdout().lock(),
                "{}",
                controller_started.controller_id.0.0
            )
            .context("write the controller's id to standard output")?;
            Ok(ExitCode::SUCCESS)
        }
        StartControllerOutcome::AlreadyRunning {
            error: ControllerAlreadyRunning { repository },
        } => refused(
            WHAT,
            "ControllerAlreadyRunning",
            &format!(
                "repository {:?} already has a running or paused controller",
                repository.0
            ),
        ),
    }
}

/// `conductor controller pause-controller`: the command `conductor.dispatch.PauseController`.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, or
/// the controller is unknown (`ControllerNotFound`) or not running (`ControllerStateConflict`).
pub fn pause_controller(state: Option<&Path>, args: PauseControllerArgs) -> Result<ExitCode> {
    const WHAT: &str = "controller pause-controller";
    flags_only(args.input_json, "controller pause-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let mut generated = record(state)?;
    let outcome = generated
        .pause_controller(PauseController { controller_id })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the pause")?;
    match outcome {
        PauseControllerOutcome::Paused { .. } => Ok(ExitCode::SUCCESS),
        PauseControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        PauseControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "only a running controller is paused")
        }
    }
}

/// `conductor controller resume-controller`: the command `conductor.dispatch.ResumeController`.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, or
/// the controller is unknown (`ControllerNotFound`) or not paused (`ControllerStateConflict`).
pub fn resume_controller(state: Option<&Path>, args: ResumeControllerArgs) -> Result<ExitCode> {
    const WHAT: &str = "controller resume-controller";
    flags_only(args.input_json, "controller resume-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let mut generated = record(state)?;
    let outcome = generated
        .resume_controller(ResumeController { controller_id })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated
        .ports
        .store
        .check()
        .context("keep the resumption")?;
    match outcome {
        ResumeControllerOutcome::Resumed { .. } => Ok(ExitCode::SUCCESS),
        ResumeControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        ResumeControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "only a paused controller is resumed")
        }
    }
}

/// `conductor controller stop-controller`: the command `conductor.dispatch.StopController`.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, or
/// the controller is unknown (`ControllerNotFound`) or already stopped (`ControllerStateConflict`).
pub fn stop_controller(state: Option<&Path>, args: StopControllerArgs) -> Result<ExitCode> {
    const WHAT: &str = "controller stop-controller";
    flags_only(args.input_json, "controller stop-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let reason = required(args.reason, WHAT, "--reason")?;
    let mut generated = record(state)?;
    let outcome = generated
        .stop_controller(StopController {
            controller_id,
            reason,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the stop")?;
    match outcome {
        StopControllerOutcome::Stopped { .. } => Ok(ExitCode::SUCCESS),
        StopControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        StopControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "the controller is already stopped")
        }
    }
}

/// `conductor controller revise-charter`: the command `conductor.dispatch.ReviseCharter`. Raises
/// the controller's `charter_revision` by one, running or paused.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, or
/// the controller is unknown (`ControllerNotFound`) or stopped (`ControllerStateConflict`).
pub fn revise_charter(state: Option<&Path>, args: ReviseCharterArgs) -> Result<ExitCode> {
    const WHAT: &str = "controller revise-charter";
    flags_only(args.input_json, "controller revise-charter --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let mut generated = record(state)?;
    let outcome = generated
        .revise_charter(ReviseCharter { controller_id })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the revision")?;
    match outcome {
        ReviseCharterOutcome::Revised { .. } | ReviseCharterOutcome::PausedRevised { .. } => {
            Ok(ExitCode::SUCCESS)
        }
        ReviseCharterOutcome::NoSuchController { error } => not_found(WHAT, &error),
        ReviseCharterOutcome::Stopped { error } => {
            state_conflict(WHAT, &error, "a stopped controller takes no charter")
        }
    }
}

/// `conductor controller controllers`: the view `conductor.dispatch.Controllers`, served by the
/// generated query over the existing store under the state directory (`--state-dir`, else
/// `state/`; [`crate::state::dir`]) and rendered as `--format` asks. A view creates no store.
///
/// Lists every controller it can read, and names each record it cannot on standard error, after
/// them, exiting 1 ([`crate::store::show`]).
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn controllers(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 6] = [
        "controller_id",
        "state",
        "repository",
        "harness",
        "session_name",
        "charter_revision",
    ];

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .controllers()
        .map_err(|unmet| anyhow!("controller controllers: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.controller_id.0.0),
                Value::from(state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(harness_name(row.harness)),
                Value::from(row.session_name.0),
                Value::from(row.charter_revision),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "controllers",
        &view.render(&COLUMNS, &rows),
    )
}

/// The ports the controller commands run over: the store, and the `StartController` behaviour
/// the specification leaves to the implementation.
struct Ports {
    store: Store,
}

/// Opens the store a command records into, creating it when it is not there.
fn record(state: Option<&Path>) -> Result<Generated<Ports>> {
    Ok(Generated::new(Ports {
        store: crate::state::open(state)?,
    }))
}

impl ControllerStorage for Ports {
    fn get(&self, identity: &ControllerId) -> Option<ControllerSnapshot> {
        ControllerStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: ControllerSnapshot) {
        ControllerStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &ControllerId) {
        ControllerStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<ControllerSnapshot> {
        ControllerStorage::list(&self.store)
    }
}

/// `conductor.dispatch.StartController`, as its contract in `crates/conductor-model/PLAN.md`
/// states it: `already-running` when a `Controller` of the input's repository is `Running` or
/// `Paused`; `started` otherwise, creating a `Running` controller under a generated id with
/// the input's `session_name` and `charter_revision` 1.
impl StartControllerBehavior for Ports {
    fn start_controller(
        &mut self,
        input: StartController,
    ) -> Result<StartControllerOutcome, UnmetObligation> {
        let held = ControllerStorage::list(&self.store)
            .into_iter()
            .any(|held| {
                held.data.repository == input.repository
                    && matches!(
                        held.state,
                        ControllerState::Running | ControllerState::Paused
                    )
            });
        if held {
            return Ok(StartControllerOutcome::AlreadyRunning {
                error: ControllerAlreadyRunning {
                    repository: input.repository,
                },
            });
        }
        let controller_id = ControllerId(Uuid(eventlog_core::new_event_id()));
        let data = ControllerData {
            controller_id: controller_id.clone(),
            repository: input.repository.clone(),
            harness: input.harness,
            session_name: input.session_name,
            charter_revision: 1,
        };
        ControllerStorage::put(
            &mut self.store,
            AnyController::Running(Controller::new(data)).snapshot(),
        );
        Ok(StartControllerOutcome::Started {
            controller_started: ControllerStarted {
                controller_id,
                repository: input.repository,
                harness: input.harness,
            },
        })
    }
}

/// Refuses `--input-json`, which no controller command reads yet.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of `flag`, which `what` cannot do without.
fn required(value: Option<String>, what: &str, flag: &str) -> Result<String> {
    match value {
        Some(value) => Ok(value),
        None => bail!("{what}: {flag} is required"),
    }
}

/// The controller `--controller-id` names; the flag's parser has already held it to a UUID.
fn controller_id(value: Option<String>, what: &str) -> Result<ControllerId> {
    required(value, what, "--controller-id").map(|id| ControllerId(Uuid(id)))
}

/// Answers a refusal the specification declares: one line naming `error`, and exit 1.
fn refused(what: &str, error: &str, why: &str) -> Result<ExitCode> {
    bail!("{what}: refused: {error}: {why}")
}

fn not_found(what: &str, error: &ControllerNotFound) -> Result<ExitCode> {
    refused(
        what,
        "ControllerNotFound",
        &format!("no controller has id {}", error.controller_id.0.0),
    )
}

fn state_conflict(what: &str, error: &ControllerStateConflict, why: &str) -> Result<ExitCode> {
    refused(
        what,
        "ControllerStateConflict",
        &format!("controller {}: {why}", error.controller_id.0.0),
    )
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn state_name(state: ControllerState) -> &'static str {
    match state {
        ControllerState::Paused => "Paused",
        ControllerState::Running => "Running",
        ControllerState::Stopped => "Stopped",
    }
}

/// The harness's name as the specification spells it. Exhaustive, so a harness the specification
/// adds fails to compile here rather than rendering under a guessed name.
fn harness_name(harness: Harness) -> &'static str {
    match harness {
        Harness::Claude => "Claude",
        Harness::Codex => "Codex",
    }
}

fn harness_named(name: &str) -> Option<Harness> {
    [Harness::Claude, Harness::Codex]
        .into_iter()
        .find(|harness| harness_name(*harness) == name)
}
