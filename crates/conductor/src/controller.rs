//! `conductor controller`: one controller session per repository.
//!
//! Each command records its outcome in the store under the state directory (`--state-dir`, else
//! `state/`; [`crate::state::dir`]) through the generated behaviours, and `controllers` answers
//! from those records. `start-controller` and `revise-charter` start, stop or message no session:
//! launching the session itself is [`crate::spawn`]. `pause-controller` and `stop-controller` stop
//! the controller's session through its instance's provider before they record it, and
//! `resume-controller` resumes it from its transcript ([`crate::spawn::end`],
//! [`crate::spawn::resume`]); a session that fails to stop or resume leaves the record as it was.
//!
//! `start-controller` prints the new controller's id, the one every other command takes;
//! `pause-controller`, `resume-controller` and `stop-controller` print one line naming the session
//! they stopped or resumed, and nothing when they touched none; the other commands print nothing. A refusal the specification declares exits 1 with one line on
//! standard error that names its error. Input through `--input-json -` is not written yet.

use std::fs::{File, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{ControllerStorage, Generated};
use conductor_model::config::SessionName;
use conductor_model::dispatch::obligations::{
    ControllersQuery, PauseControllerBehavior, ResumeControllerBehavior, ReviseCharterBehavior,
    StartControllerBehavior, StopControllerBehavior,
};
use conductor_model::dispatch::{
    AnyController, Controller, ControllerAlreadyRunning, ControllerData, ControllerId,
    ControllerNotFound, ControllerSnapshot, ControllerStarted, ControllerState,
    ControllerStateConflict, DispatchLogEvent, PauseController, PauseControllerOutcome,
    ResumeController, ResumeControllerOutcome, ReviseCharter, ReviseCharterOutcome,
    StartController, StartControllerOutcome, StopController, StopControllerOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::primitives::Uuid;
use serde_json::Value;

use crate::cli::{
    JsonInput, PauseControllerArgs, ResumeControllerArgs, ReviseCharterArgs, StartControllerArgs,
    StopControllerArgs, ViewArgs,
};
use crate::confine::Providers;
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
        } => {
            let held = holder(&generated, &repository)
                .map_or_else(String::new, |held| format!(": controller {held}"));
            refused(
                WHAT,
                "ControllerAlreadyRunning",
                &format!(
                    "repository {:?} already has a running or paused controller{held}",
                    repository.0
                ),
            )
        }
    }
}

/// The controller `controller_id` as the store holds it now, if it does; the store is let go
/// before the session is touched.
fn held(state: Option<&Path>, controller_id: &ControllerId) -> Result<Option<ControllerSnapshot>> {
    let generated = record(state)?;
    let held = ControllerStorage::get(&generated.ports, controller_id);
    generated
        .ports
        .store
        .check()
        .context("read the controller")?;
    Ok(held)
}

/// The controller `controller_id`'s repository and session name, when the store holds it in one
/// of `states`.
fn in_state(
    state: Option<&Path>,
    controller_id: &ControllerId,
    states: &[ControllerState],
) -> Result<Option<(String, SessionName)>> {
    Ok(held(state, controller_id)?
        .filter(|held| states.contains(&held.state))
        .map(|held| (held.data.repository.0, held.data.session_name)))
}

/// Writes `line` on standard output, when there is one.
fn say(line: Option<String>) -> Result<()> {
    if let Some(line) = line {
        writeln!(io::stdout().lock(), "{line}").context("write the session to standard output")?;
    }
    Ok(())
}

/// `conductor controller pause-controller`: the command `conductor.dispatch.PauseController`.
/// A running controller's session is stopped first through its instance's provider
/// ([`crate::spawn::end`]), and the command prints `stopped <session name> (<id>)`; a session no
/// provider's measurement records is left as it is.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, the
/// session is not stopped, or the controller is unknown (`ControllerNotFound`) or not running
/// (`ControllerStateConflict`).
pub fn pause_controller(
    state: Option<&Path>,
    args: PauseControllerArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    const WHAT: &str = "controller pause-controller";
    flags_only(args.input_json, "controller pause-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let stopped = match in_state(state, &controller_id, &[ControllerState::Running])? {
        Some((repository, session_name)) => {
            crate::spawn::end(state, &repository, &session_name, providers)
                .with_context(|| format!("{WHAT}: the controller stays running"))?
        }
        None => None,
    };
    let mut generated = record(state)?;
    let outcome = generated
        .pause_controller(PauseController { controller_id })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the pause")?;
    match outcome {
        PauseControllerOutcome::Paused { .. } => {
            say(stopped.map(|session| format!("stopped {session}")))?;
            Ok(ExitCode::SUCCESS)
        }
        PauseControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        PauseControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "only a running controller is paused")
        }
    }
}

/// `conductor controller resume-controller`: the command `conductor.dispatch.ResumeController`.
/// A paused controller's session is resumed first from its transcript through its instance's
/// provider ([`crate::spawn::resume`]), and the command prints `resumed <session name> (<id>)`,
/// with `in <unit>` under the `systemd` provider; a session no provider's measurement records, or
/// one the session list shows live, is left as it is.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, the
/// session is not resumed, or the controller is unknown (`ControllerNotFound`) or not paused
/// (`ControllerStateConflict`).
pub fn resume_controller(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: ResumeControllerArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    const WHAT: &str = "controller resume-controller";
    flags_only(args.input_json, "controller resume-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let resumed = match in_state(state, &controller_id, &[ControllerState::Paused])? {
        Some((repository, session_name)) => {
            crate::spawn::resume(config_flag, state, &repository, &session_name, providers)
                .with_context(|| format!("{WHAT}: the controller stays paused"))?
        }
        None => None,
    };
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
        ResumeControllerOutcome::Resumed { .. } => {
            say(resumed.map(|session| format!("resumed {session}")))?;
            Ok(ExitCode::SUCCESS)
        }
        ResumeControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        ResumeControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "only a paused controller is resumed")
        }
    }
}

/// `conductor controller stop-controller`: the command `conductor.dispatch.StopController`.
/// A running or paused controller's session is stopped first through its instance's provider
/// ([`crate::spawn::end`]), and the command prints `stopped <session name> (<id>)`; a session
/// that is gone already, or that no provider's measurement records, is left as it is.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the record, the
/// session is not stopped, or the controller is unknown (`ControllerNotFound`) or already stopped
/// (`ControllerStateConflict`).
pub fn stop_controller(
    state: Option<&Path>,
    args: StopControllerArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    const WHAT: &str = "controller stop-controller";
    flags_only(args.input_json, "controller stop-controller --input-json")?;
    let controller_id = controller_id(args.controller_id, WHAT)?;
    let reason = required(args.reason, WHAT, "--reason")?;
    let stopped = match in_state(
        state,
        &controller_id,
        &[ControllerState::Running, ControllerState::Paused],
    )? {
        Some((repository, session_name)) => {
            crate::spawn::end(state, &repository, &session_name, providers)
                .with_context(|| format!("{WHAT}: the controller is not recorded stopped"))?
        }
        None => None,
    };
    let mut generated = record(state)?;
    let outcome = generated
        .stop_controller(StopController {
            controller_id,
            reason,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the stop")?;
    match outcome {
        StopControllerOutcome::Stopped { .. } => {
            say(stopped.map(|session| format!("stopped {session}")))?;
            Ok(ExitCode::SUCCESS)
        }
        StopControllerOutcome::NoSuchController { error } => not_found(WHAT, &error),
        StopControllerOutcome::WrongState { error } => {
            state_conflict(WHAT, &error, "the controller is already stopped")
        }
    }
}

/// `conductor controller revise-charter`: the command `conductor.dispatch.ReviseCharter`. Raises
/// the controller's `charter_revision` by one, running or paused, and once the store kept it
/// appends one `charter-revised` line to the records' dispatch log ([`crate::records`]) with the
/// controller's id, session name and new revision.
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
        ReviseCharterOutcome::Revised { charter_revised }
        | ReviseCharterOutcome::PausedRevised { charter_revised } => {
            let store = &generated.ports.store;
            let held = ControllerStorage::get(store, &charter_revised.controller_id);
            store.check().context("read the revision back")?;
            let held = held.ok_or_else(|| {
                anyhow!(
                    "{WHAT}: controller {} was revised, yet reads back as absent",
                    charter_revised.controller_id.0.0
                )
            })?;
            let mut line = crate::records::dispatch_line(
                DispatchLogEvent::CharterRevised,
                &held.data.repository.0,
            )?;
            line.controller_id = Some(charter_revised.controller_id.0.0.clone());
            line.to = Some(held.data.session_name.0.clone());
            line.charter_revision = Some(held.data.charter_revision);
            crate::records::append_dispatch(&line).with_context(|| {
                format!("{WHAT}: controller {}", charter_revised.controller_id.0.0)
            })?;
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
/// the specification leaves to the implementation. `conductor spawn` records through them too
/// ([`crate::spawn`]).
pub(crate) struct Ports {
    pub(crate) store: Store,
    /// The controllers' lock ([`LOCK`]), held as long as the ports are.
    _lock: File,
}

/// The lock in the state directory every command that records a controller holds from before it
/// reads the controllers until its record is kept (`story:controller-start-atomic`): the check
/// that a repository has no running or paused controller and the start that records one are then
/// one step, however many starts race. The store's own writer lock is taken by each commit and
/// cannot be held across one, so this lock, as the host ledger's (`host.lock`), is held instead.
const LOCK: &str = "controllers.lock";

/// Opens the store a command records into, creating it when it is not there, under the
/// controllers' lock ([`LOCK`]), which is held until the answer is dropped.
pub(crate) fn record(state: Option<&Path>) -> Result<Generated<Ports>> {
    let dir = crate::state::dir(state)?;
    // The store is opened first, so a state directory it refuses (one `conductor store migrate`
    // is to move) is left as it was. It reads the tree again at each read after another writer
    // committed, so the reads under the lock see every start kept before it.
    let store = crate::store::open(&dir)?;
    let path = dir.join(LOCK);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open the controllers' lock {}", path.display()))?;
    lock.lock()
        .with_context(|| format!("lock the controllers {}", path.display()))?;
    Ok(Generated::new(Ports { store, _lock: lock }))
}

/// The running or paused controller of `repository`, as `<id> (<state>, session <name>)`, read
/// from the store `generated` holds; none when there is none.
pub(crate) fn holder(generated: &Generated<Ports>, repository: &RepositoryName) -> Option<String> {
    ControllerStorage::list(&generated.ports.store)
        .into_iter()
        .find(|held| {
            held.data.repository == *repository
                && matches!(
                    held.state,
                    ControllerState::Running | ControllerState::Paused
                )
        })
        .map(|held| {
            format!(
                "{} ({}, session {})",
                held.data.controller_id.0.0,
                state_name(held.state),
                held.data.session_name.0
            )
        })
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
