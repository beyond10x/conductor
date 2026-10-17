//! `conductor spawn` (`story:spawn`): starts a repository's controller session; and the session
//! end and resume behind `controller pause-controller`, `resume-controller` and `stop-controller`.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `spawn`, its input `conductor.dispatch.SpawnController`; each refusal is a
//! `conductor.dispatch.SpawnRefused`, named by its `conductor.dispatch.SpawnRefusal` on one line of
//! standard error, exit 1.
//!
//! It starts the controller through the confinement provider the instance's `confinement` names
//! ([`crate::confine`], `story:session-envelope-seam`), with the controller role's request: the
//! checkout as its start directory, its limits, its `writable` places resolved to absolute paths
//! ([`crate::confine::places`]), its network and its `forge_token_env`; the charter is the first
//! prompt. Under `strict` a provider that is not available refuses before anything is recorded
//! (`ProviderUnavailable`), and a box measured short of the request is stopped and the controller
//! recorded stopped (`MeasurementShort`). Through the `none` provider, which `report` falls back
//! to, the session is the controller start form below, one program with its arguments and never
//! through a shell, with its working directory the checkout `<checkouts.root>/<repository>`:
//!
//! ```text
//! claude --bg -n <session name> --append-system-prompt-file <profile> --model <model>
//!   --permission-mode bypassPermissions --setting-sources project,local --settings <settings>
//!   <charter>
//! ```
//!
//! The session name is the instance's for the repository ([`config::session_name`]); profile,
//! model and settings are the instance's `controller` role's; the charter is the text of
//! `<records>/charters/<repository>.md`, one argument. With a config file the session also gets
//! the instance's name and the config file's absolute path in `CONDUCTOR_INSTANCE` and
//! `CONDUCTOR_CONFIG`, so the controller and its guard run as the instance that started it. The
//! settings must not pin another instance (`InstanceNotPinned`, `story:instance-isolation`); ones
//! that pin none, as the shipped `.claude/controller-settings.json` does, are passed as a copy
//! pinned to the instance, `<state>/sessions/<session name>.settings.json`.
//!
//! In order, it refuses, recording and launching nothing: a repository that names no git
//! checkout directly, or one group down, under the checkouts root, one whose physical path lies
//! outside the root's physical path, or one an `exclude` entry of the instance's sources leaves out
//! (`NoCheckout`); a checkout that is the instance's records directory, lies in it or holds it,
//! where the guard takes a session for conductor's (`RecordsCheckout`); a role that is missing (`NoControllerRole`), runs
//! on another harness (`NotClaude`), or names no profile or settings file (`NoProfile`,
//! `NoSettings`); a checkout `~/.claude.json` does not mark trusted, where `claude --bg` would
//! start nothing (`UntrustedCheckout`); a charter that is missing, empty, not UTF-8, or starts
//! with `-`, which `claude` would read as a flag (`NoCharter`); and a live session that holds the
//! session name in `claude agents --json`, as the sessions collector reads the list
//! (`SessionRunning`), or a list that cannot be read (`SessionsUnread`). Then it records
//! `StartController` as `controller start-controller` does, which refuses a repository whose
//! controller is `Running` or `Paused` (`AlreadyRunning`), and launches. A launch that fails,
//! exits other than 0 or is stopped at its bound is refused (`LaunchFailed`), naming what `claude`
//! said; it is recorded `StopController` only when the session list, read again, shows no live
//! session of the name. A live one, or a list that cannot be read, leaves the controller `Running`,
//! and the refusal names the session or says so. It prints what `claude --bg` printed: the background session's id.
//! A provider whose session prints nothing to conductor, as the `systemd` provider's runs in its
//! unit, answers the id it chose, and `spawn` prints that id on one line.
//!
//! [`end`] and [`resume`] stop and resume a controller session through its instance's provider,
//! for `controller pause-controller`, `resume-controller` and `stop-controller`
//! ([`crate::controller`]): the session's provider, unit and id are those its measurement
//! records under the state directory.
//!
//! A launch that succeeds appends one `sent` line to the records' dispatch log
//! ([`crate::records`]): the repository as `repo`, the controller's id, the session name as `to`,
//! that session id and the charter revision 1.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{
    CommandWord, ConfinementProvider, Harness as RoleHarness, Instance, Limits, Network, Role,
    SessionName,
};
use conductor_model::dispatch::obligations::{StartControllerBehavior, StopControllerBehavior};
use conductor_model::dispatch::{
    ControllerAlreadyRunning, DispatchLogEvent, SpawnController, SpawnRefusal, SpawnRefused,
    StartController, StartControllerOutcome, StopController, StopControllerOutcome,
};
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::session::{SessionRequest, StartedSession};
use serde_json::Value;

use crate::cli::SpawnArgs;
use crate::collect::{self, Under};
use crate::config::{self, Environment};
use crate::confine::places::{self, Placement};
use crate::confine::{self, CLAUDE, Launch, Providers, Refusal, live_session};

const COMMAND: &str = "spawn";

/// `conductor spawn`: starts the controller session of `--repository` through the provider the
/// instance's `confinement` names ([`confine`]) and records the controller `Running`, as the
/// module documentation says. Prints what `claude --bg` printed, or the id the provider chose.
///
/// # Errors
///
/// `--repository` is missing, the environment or the store cannot be read, the record is not
/// kept, standard output cannot be written, or a refusal the module documentation names.
pub fn spawn(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &SpawnArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    let input = SpawnController {
        repository: args.repository.clone(),
    };
    let Some(repository) = input.repository else {
        bail!("{COMMAND}: --repository is required");
    };
    let refuse = |refusal: SpawnRefusal, detail: String| {
        refused(&SpawnRefused {
            repository: repository.clone(),
            refusal,
            detail,
        })
    };
    let active = config::active();
    let instance = &active.instance;
    let checkout = match checkout(instance, &repository) {
        Ok(checkout) => checkout,
        Err(detail) => return refuse(SpawnRefusal::NoCheckout, detail),
    };
    if let Err(detail) = outside_records(instance, &checkout) {
        return refuse(SpawnRefusal::RecordsCheckout, detail);
    }
    let (controller, profile, settings, model) = match role(instance) {
        Ok(role) => role,
        Err((refusal, detail)) => return refuse(refusal, detail),
    };
    let environment = Environment::from_process().context(COMMAND)?;
    let home = environment.home().context(COMMAND)?;
    if let Err(detail) = trusted(home, &checkout) {
        return refuse(SpawnRefusal::UntrustedCheckout, detail);
    }
    let charter_file = Path::new(&instance.records)
        .join("charters")
        .join(format!("{repository}.md"));
    let charter = match charter(&charter_file) {
        Ok(charter) => charter,
        Err(detail) => return refuse(SpawnRefusal::NoCharter, detail),
    };
    let session_name = config::session_name(instance, &repository);
    // Under `strict`, a provider that is not available starts nothing and records nothing.
    let chosen = match confine::choose(providers, &instance.confinement) {
        Ok(chosen) => chosen,
        Err(refusal) => return refuse_confined(&refuse, refusal),
    };
    match live_session(&session_name.0) {
        Ok(None) => {}
        Ok(Some(id)) => {
            return refuse(
                SpawnRefusal::SessionRunning,
                format!(
                    "the live session {id} holds the session name {}",
                    session_name.0
                ),
            );
        }
        Err(error) => return refuse(SpawnRefusal::SessionsUnread, format!("{error:#}")),
    }
    let state_dir = crate::state::dir(state)?;
    // Written before the controller is recorded, so a copy that cannot be written records nothing.
    let settings = pinned_settings(instance, settings, &state_dir, &session_name)?;

    // The store is held only while a record is written, not while the session starts.
    let outcome = {
        let mut generated = crate::controller::record(state)?;
        let outcome = generated
            .start_controller(StartController {
                repository: RepositoryName(repository.clone()),
                harness: Harness::Claude,
                session_name: session_name.clone(),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        generated
            .ports
            .store
            .check()
            .context("keep the controller")?;
        let held = crate::controller::holder(&generated, &RepositoryName(repository.clone()));
        (outcome, held)
    };
    let controller_id = match outcome {
        (StartControllerOutcome::Started { controller_started }, _) => {
            controller_started.controller_id
        }
        (
            StartControllerOutcome::AlreadyRunning {
                error: ControllerAlreadyRunning { .. },
            },
            held,
        ) => {
            let held = held.map_or_else(String::new, |held| format!(": controller {held}"));
            return refuse(
                SpawnRefusal::AlreadyRunning,
                format!(
                    "the repository already has a running or paused controller{held} (controller \
                     controllers)"
                ),
            );
        }
    };

    let launch = controller_launch(
        config_flag,
        &state_dir,
        &ControllerRole {
            role: controller,
            profile: &profile,
            settings: &settings,
            model: &model,
        },
        &repository,
        &checkout,
        &environment,
    )?;
    // The none provider's launch is `claude --bg`; another provider's is its own start.
    let launcher = if chosen.name() == "none" {
        format!("{CLAUDE} --bg")
    } else {
        format!("the {} provider's start", chosen.name())
    };
    let failure = match chosen.start(&launch, &charter, &state_dir) {
        Ok(started) => {
            confine::pass_through(&started.said)?;
            // A provider whose harness printed nothing, as the systemd provider's session in its
            // unit prints nothing to conductor, still answers the session's id: spawn prints it.
            if started.said.stdout.is_empty()
                && let Some(id) = &started.session.session_id
            {
                writeln!(io::stdout().lock(), "{id}")
                    .context("write the session's id to standard output")?;
            }
            let mut line = crate::records::dispatch_line(DispatchLogEvent::Sent, &repository)?;
            line.controller_id = Some(controller_id.0.0.clone());
            line.to = Some(session_name.0.clone());
            line.session_id = started.session.session_id.clone();
            line.charter_revision = Some(1);
            crate::records::append_dispatch(&line).with_context(|| {
                format!(
                    "{COMMAND}: the session started and controller {} is recorded",
                    controller_id.0.0
                )
            })?;
            return Ok(ExitCode::SUCCESS);
        }
        Err(Refusal::LaunchFailed { detail, .. }) => detail,
        Err(Refusal::MeasurementShort(detail)) => {
            record_stopped(
                state,
                controller_id,
                format!("{COMMAND}: the box is short of the request: {detail}"),
            )?;
            return refuse(
                SpawnRefusal::MeasurementShort,
                format!("{detail}; the controller is recorded stopped"),
            );
        }
        Err(refusal @ Refusal::ProviderUnavailable(_)) => {
            return refuse_confined(&refuse, refusal);
        }
    };
    // A launch can fail, or be stopped at the bound, after the session started: the session list,
    // read again, decides whether the controller is recorded stopped.
    match live_session(&session_name.0) {
        Ok(None) => {}
        Ok(Some(id)) => {
            return refuse(
                SpawnRefusal::LaunchFailed,
                format!(
                    "{launcher} {failure}; yet the live session {id} holds the session name \
                     {}, so the controller {} stays recorded running",
                    session_name.0, controller_id.0.0
                ),
            );
        }
        Err(error) => {
            return refuse(
                SpawnRefusal::LaunchFailed,
                format!(
                    "{launcher} {failure}; the session list could not be read again \
                     ({error:#}), so the controller {} stays recorded running: stop it with \
                     conductor controller stop-controller once no session {} runs",
                    controller_id.0.0, session_name.0
                ),
            );
        }
    }
    record_stopped(
        state,
        controller_id,
        format!("{COMMAND}: the session did not start: {failure}"),
    )?;
    refuse(
        SpawnRefusal::LaunchFailed,
        format!("{launcher} {failure}; the controller is recorded stopped"),
    )
}

/// Records the controller `controller_id` stopped for `reason`.
fn record_stopped(
    state: Option<&Path>,
    controller_id: conductor_model::dispatch::ControllerId,
    reason: String,
) -> Result<()> {
    let mut generated = crate::controller::record(state)?;
    let outcome = generated
        .stop_controller(StopController {
            controller_id,
            reason,
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    generated.ports.store.check().context("keep the stop")?;
    match outcome {
        StopControllerOutcome::Stopped { .. } => Ok(()),
        StopControllerOutcome::NoSuchController { .. }
        | StopControllerOutcome::WrongState { .. } => {
            bail!("{COMMAND}: the controller it recorded could not be recorded stopped");
        }
    }
}

/// Answers a refusal of the confinement ([`confine::Refusal`]) as its `SpawnRefusal`.
fn refuse_confined(
    refuse: &dyn Fn(SpawnRefusal, String) -> Result<ExitCode>,
    refusal: Refusal,
) -> Result<ExitCode> {
    match refusal {
        Refusal::ProviderUnavailable(detail) => refuse(SpawnRefusal::ProviderUnavailable, detail),
        Refusal::MeasurementShort(detail) => refuse(SpawnRefusal::MeasurementShort, detail),
        Refusal::LaunchFailed { detail, .. } => refuse(SpawnRefusal::LaunchFailed, detail),
    }
}

/// The controller role and the files it starts with, each checked to be one ([`role`]).
struct ControllerRole<'r> {
    role: &'r Role,
    profile: &'r str,
    settings: &'r str,
    model: &'r str,
}

/// The launch of `repository`'s controller session in `checkout`, as the module documentation
/// gives it: the controller role's request, the harness arguments before the charter, and with a
/// config file the instance's name and the file's absolute path for the session.
fn controller_launch(
    config_flag: Option<&Path>,
    state_dir: &Path,
    controller: &ControllerRole<'_>,
    repository: &str,
    checkout: &Path,
    environment: &Environment,
) -> Result<Launch> {
    let active = config::active();
    let instance = &active.instance;
    let home = environment.home().context(COMMAND)?;
    let session_name = config::session_name(instance, repository);
    let role = controller.role;
    let request = SessionRequest {
        role: config::CONTROLLER.to_owned(),
        session_name: session_name.clone(),
        cwd: checkout.display().to_string(),
        limits: role.limits.clone(),
        writable: places::resolve(
            &role.writable,
            &Placement {
                home: home.to_owned(),
                tmpdir: confine::tmpdir(),
                start: checkout.to_owned(),
                repository: Some(repository.to_owned()),
                trees: PathBuf::from(&instance.checkouts.trees),
                records: PathBuf::from(&instance.records),
                cache: PathBuf::from(&instance.cache),
                harness: role.harness,
                controller: true,
            },
        ),
        network: role.network,
        forge_token_env: role.forge_token_env.clone(),
    };
    // The session's socket to the instance's broker (`story:tool-broker-read`): registered with
    // the controller role's grant before the session starts, and named in its environment. It is
    // the repository's lead, reporting to the director (`story:tool-build-commit-report`).
    let socket = crate::broker::register_session(
        state_dir,
        &instance.name,
        role,
        &session_name,
        checkout,
        crate::broker::placement(instance, config::CONTROLLER, Some(repository)),
    )
    .context(COMMAND)?;
    let mut env = vec![(
        conductor_broker::SOCKET_VARIABLE.to_owned(),
        socket.display().to_string(),
    )];
    if active.from_file {
        // Absolute, as the guard resolves it: the session runs in the checkout, where a relative
        // path would name a file of the repository.
        let (file, _) = config::locate(config_flag, environment).context(COMMAND)?;
        env.push((
            config::INSTANCE_VARIABLE.to_owned(),
            instance.name.0.clone(),
        ));
        env.push((
            config::CONFIG_VARIABLE.to_owned(),
            environment.cwd.join(file).display().to_string(),
        ));
    }
    Ok(Launch {
        request,
        session_prefix: instance.session_prefix.clone(),
        instance_limits: instance.confinement.limits.clone(),
        state: state_dir.to_owned(),
        program: CLAUDE.to_owned(),
        args: [
            "-n",
            &session_name.0,
            "--append-system-prompt-file",
            controller.profile,
            "--model",
            controller.model,
            "--permission-mode",
            "bypassPermissions",
            "--setting-sources",
            "project,local",
            "--settings",
            controller.settings,
        ]
        .map(str::to_owned)
        .to_vec(),
        env,
    })
}

/// The session `name` as its measurement under the state directory records it, or `None` when
/// none was written: a session no provider of this build started.
fn recorded_session(state_dir: &Path, name: &SessionName) -> Result<Option<StartedSession>> {
    Ok(confine::read_measurement(state_dir, name)?.map(|measurement| measurement.session))
}

/// Ends the controller session `session_name`, the one behind `controller pause-controller` and
/// `controller stop-controller`, through the provider its measurement names: the `systemd`
/// provider stops its unit (`systemctl --user stop`; nothing when the unit is gone already), the
/// `none` provider a session the session list shows live (`claude stop`, `claude rm`). The
/// measurement stays: it holds the id a resume starts from. A session with no measurement is left
/// as it is.
///
/// Answers the session's name and id when a session was stopped, `None` otherwise.
///
/// # Errors
///
/// The measurement or the session list cannot be read, or the provider did not stop the session.
pub fn end(
    state: Option<&Path>,
    repository: &str,
    session_name: &SessionName,
    providers: &dyn Providers,
) -> Result<Option<String>> {
    let state_dir = crate::state::dir(state)?;
    let Some(session) = recorded_session(&state_dir, session_name)? else {
        return Ok(None);
    };
    let systemd = session.provider == "systemd";
    if !systemd && live_session(&session_name.0)?.is_none() {
        return Ok(None);
    }
    let mut provider = providers.provider(if systemd {
        ConfinementProvider::Systemd
    } else {
        ConfinementProvider::None
    });
    let instance = &config::active().instance;
    let cwd = checkout(instance, repository).map_or_else(
        |_| instance.checkouts.root.clone(),
        |dir| dir.display().to_string(),
    );
    let request = SessionRequest {
        role: config::CONTROLLER.to_owned(),
        session_name: session_name.clone(),
        cwd,
        limits: Limits {
            memory_max: None,
            cpu_weight: None,
            tasks_max: None,
        },
        writable: Vec::new(),
        network: Network::Open,
        forge_token_env: Vec::new(),
    };
    provider
        .stop(&session, &request)
        .with_context(|| format!("stop the controller session {}", session_name.0))?;
    Ok(Some(format!(
        "{} ({})",
        session_name.0,
        session.session_id.as_deref().unwrap_or("with no id")
    )))
}

/// Resumes `repository`'s controller session `session_name` from its transcript through the
/// provider the instance's `confinement` names ([`confine::choose`]), with the launch `spawn`
/// starts it with, and writes its new measurement: the `systemd` provider starts a new unit of the
/// session's name running `claude --resume <id>` once the old unit is gone, the `none` provider
/// runs `claude --bg --resume <id>`. The session's id is the one its measurement records.
///
/// A session with no measurement, or one the session list shows live, is left as it is: there
/// is nothing to resume.
///
/// Answers the session's name and id, and the unit it runs in now, when a session was resumed.
///
/// # Errors
///
/// The measurement or the session list cannot be read; the measurement records no id; the
/// checkout or the controller role no longer holds; under `strict` the provider is not
/// available, or the resumed box measures short of the request and is stopped again; the
/// provider's resume fails.
pub fn resume(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    repository: &str,
    session_name: &SessionName,
    providers: &dyn Providers,
) -> Result<Option<String>> {
    let state_dir = crate::state::dir(state)?;
    let Some(session) = recorded_session(&state_dir, session_name)? else {
        return Ok(None);
    };
    if session.session_id.is_none() {
        bail!(
            "the measurement of the session {} records no id to resume it from: stop the \
             controller and spawn it again",
            session_name.0
        );
    }
    if live_session(&session_name.0)?.is_some() {
        return Ok(None);
    }
    let instance = &config::active().instance;
    let checkout = checkout(instance, repository).map_err(|detail| anyhow!("{detail}"))?;
    let (controller, profile, settings, model) =
        role(instance).map_err(|(_, detail)| anyhow!("{detail}"))?;
    let settings = pinned_settings(instance, settings, &state_dir, session_name)?;
    let environment = Environment::from_process().context(COMMAND)?;
    let launch = controller_launch(
        config_flag,
        &state_dir,
        &ControllerRole {
            role: controller,
            profile: &profile,
            settings: &settings,
            model: &model,
        },
        repository,
        &checkout,
        &environment,
    )?;
    let chosen = confine::choose(providers, &instance.confinement)
        .map_err(|refusal| anyhow!("{}", refusal_detail(refusal)))?;
    let started = chosen
        .resume(&session, &launch, &state_dir)
        .map_err(|refusal| anyhow!("{}", refusal_detail(refusal)))?;
    let id = started
        .session
        .session_id
        .as_deref()
        .unwrap_or("with no id");
    Ok(Some(match &started.session.unit {
        Some(unit) => format!("{} ({id}) in {unit}", session_name.0),
        None => format!("{} ({id})", session_name.0),
    }))
}

/// What a refusal of the confinement says, named.
fn refusal_detail(refusal: Refusal) -> String {
    match refusal {
        Refusal::ProviderUnavailable(detail) => format!("ProviderUnavailable: {detail}"),
        Refusal::MeasurementShort(detail) => format!("MeasurementShort: {detail}"),
        Refusal::LaunchFailed { detail, .. } => format!("LaunchFailed: {detail}"),
    }
}

/// Answers a refusal: one line naming its `SpawnRefusal`, and exit 1.
fn refused(answer: &SpawnRefused) -> Result<ExitCode> {
    bail!(
        "{COMMAND}: refused: {}: repository {:?}: {}",
        refusal_name(answer.refusal),
        answer.repository,
        answer.detail
    )
}

/// The refusal's name as the specification spells it. Exhaustive, so a refusal the specification
/// adds fails to compile here rather than rendering under a guessed name.
const fn refusal_name(refusal: SpawnRefusal) -> &'static str {
    match refusal {
        SpawnRefusal::InstanceNotPinned => "InstanceNotPinned",
        SpawnRefusal::AlreadyRunning => "AlreadyRunning",
        SpawnRefusal::SessionRunning => "SessionRunning",
        SpawnRefusal::SessionsUnread => "SessionsUnread",
        SpawnRefusal::NoControllerRole => "NoControllerRole",
        SpawnRefusal::NotClaude => "NotClaude",
        SpawnRefusal::NoProfile => "NoProfile",
        SpawnRefusal::NoSettings => "NoSettings",
        SpawnRefusal::NoCheckout => "NoCheckout",
        SpawnRefusal::RecordsCheckout => "RecordsCheckout",
        SpawnRefusal::UntrustedCheckout => "UntrustedCheckout",
        SpawnRefusal::NoCharter => "NoCharter",
        SpawnRefusal::LaunchFailed => "LaunchFailed",
        SpawnRefusal::ProviderUnavailable => "ProviderUnavailable",
        SpawnRefusal::MeasurementShort => "MeasurementShort",
    }
}

/// The physical path of `repository`'s checkout under `instance`'s checkouts root: `<repo>` or
/// `<group>/<repo>`, each step a plain name starting with a letter or digit, the directory holding
/// `.git` that the sessions collector reads as that repository ([`collect::repository_of`]), and
/// no `exclude` entry of the instance's sources leaving it out.
fn checkout(instance: &Instance, repository: &str) -> Result<PathBuf, String> {
    let steps: Vec<&str> = repository.split('/').collect();
    let plain = |step: &&str| {
        step.chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
            && step
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    if !matches!(steps.len(), 1 | 2) || !steps.iter().all(plain) {
        return Err(
            "is not <repo> or <group>/<repo>, each a name of letters, digits, '.', '_' and '-' \
             starting with a letter or digit"
                .to_owned(),
        );
    }
    let root = Path::new(&instance.checkouts.root);
    let trees = Path::new(&instance.checkouts.trees);
    let dir = root.join(repository);
    if !dir.join(".git").exists()
        || collect::repository_of(root, trees, Under::Checkouts, &steps).as_deref()
            != Some(repository)
    {
        return Err(format!(
            "{} is not a git checkout of the repository under the checkouts root",
            dir.display()
        ));
    }
    if collect::is_excluded(&steps, &collect::excluded_under(instance, root)) {
        return Err(
            "an exclude entry of the instance's sources leaves the repository out".to_owned(),
        );
    }
    let physical =
        fs::canonicalize(&dir).map_err(|error| format!("resolve {}: {error}", dir.display()))?;
    let physical_root =
        fs::canonicalize(root).map_err(|error| format!("resolve {}: {error}", root.display()))?;
    if !physical.starts_with(&physical_root) || physical == physical_root {
        return Err(format!(
            "{} resolves to {}, outside the checkouts root {}",
            dir.display(),
            physical.display(),
            physical_root.display()
        ));
    }
    Ok(physical)
}

/// Refuses a checkout that is the instance's records directory, lies inside it or holds it,
/// compared by physical paths (the records as written when they do not resolve): a session there
/// is conductor's to the guard, whatever its name.
fn outside_records(instance: &Instance, checkout: &Path) -> Result<(), String> {
    let written = Path::new(&instance.records);
    let records = fs::canonicalize(written).unwrap_or_else(|_| written.to_owned());
    if checkout.starts_with(&records) || records.starts_with(checkout) {
        return Err(format!(
            "the checkout {} is, lies in or holds the instance's records directory {}, where a \
             session is conductor's",
            checkout.display(),
            records.display()
        ));
    }
    Ok(())
}

/// The `controller` role, with its profile, settings and model, each file checked to be one.
fn role(instance: &Instance) -> Result<(&Role, String, String, String), (SpawnRefusal, String)> {
    let Some(role) = instance
        .roles
        .iter()
        .find(|role| role.role == config::CONTROLLER)
    else {
        return Err((
            SpawnRefusal::NoControllerRole,
            "the instance names no controller role in conductor config show".to_owned(),
        ));
    };
    let Role {
        harness,
        model,
        profile,
        settings,
        ..
    } = role;
    if *harness != RoleHarness::Claude {
        return Err((
            SpawnRefusal::NotClaude,
            "the controller role does not run on claude (conductor config show); spawn starts \
             claude only"
                .to_owned(),
        ));
    }
    let file = |value: &Option<CommandWord>, refusal: SpawnRefusal, what: &str| match value {
        Some(path) if Path::new(&path.0).is_file() => Ok(path.0.clone()),
        Some(path) => Err((
            refusal,
            format!("the controller role's {what} {} is not a file", path.0),
        )),
        None => Err((
            refusal,
            format!("the controller role names no {what} in conductor config show"),
        )),
    };
    let profile = file(profile, SpawnRefusal::NoProfile, "profile")?;
    let settings = file(settings, SpawnRefusal::NoSettings, "settings")?;
    // The settings select the instance the session's hooks act for (`story:instance-isolation`):
    // they must not pin another. Settings that pin none start pinned ([`pinned_settings`]).
    if config::active().from_file
        && let Some(problem) = config::pin_refusal(Path::new(&settings), &instance.name.0)
    {
        return Err((
            SpawnRefusal::InstanceNotPinned,
            format!("the controller role's settings {settings} {problem}"),
        ));
    }
    Ok((role, profile, settings, model.0.clone()))
}

/// The settings the controller session `session_name` starts with (`story:instance-isolation`):
/// under a config file, the role's `settings` when they pin the instance, else a copy of them
/// pinned to it under the state directory ([`config::session_settings`]); without one, the
/// role's `settings` as they are.
fn pinned_settings(
    instance: &Instance,
    settings: String,
    state_dir: &Path,
    session_name: &SessionName,
) -> Result<String> {
    if !config::active().from_file {
        return Ok(settings);
    }
    config::session_settings(
        Path::new(&settings),
        &instance.name.0,
        state_dir,
        &session_name.0,
    )
    .context(COMMAND)
}

/// Whether `~/.claude.json` under `home` marks `checkout`, a physical path, trusted:
/// `projects[<checkout>].hasTrustDialogAccepted` is `true`, as `conductor trust` sets it
/// ([`crate::trust`]). Reads the file and writes nothing.
fn trusted(home: &Path, checkout: &Path) -> Result<(), String> {
    let file = home.join(crate::trust::CLAUDE_JSON);
    let untrusted = || {
        format!(
            "{} is not marked trusted in {}; run conductor trust",
            checkout.display(),
            file.display()
        )
    };
    let Ok(text) = fs::read(&file) else {
        return Err(untrusted());
    };
    let document: Value = serde_json::from_slice(&text)
        .map_err(|error| format!("{} is not JSON: {error}", file.display()))?;
    let key = checkout.to_string_lossy();
    match document["projects"][&*key]["hasTrustDialogAccepted"] {
        Value::Bool(true) => Ok(()),
        _ => Err(untrusted()),
    }
}

/// The charter's text, refused when it is missing, not UTF-8, empty, or starts with `-`.
fn charter(file: &Path) -> Result<String, String> {
    let text = fs::read_to_string(file)
        .map_err(|error| format!("read the charter {}: {error}", file.display()))?;
    if text.trim().is_empty() {
        return Err(format!("the charter {} is empty", file.display()));
    }
    if text.starts_with('-') {
        return Err(format!(
            "the charter {} starts with '-', which claude would read as a flag",
            file.display()
        ));
    }
    Ok(text)
}

/// The session id `claude --bg` printed: the id of its `backgrounded · <id> · <name>` line, or the
/// output itself when it is one word (a harness that prints only the id).
#[must_use]
pub fn session_id(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|line| {
            let mut parts = line.split('·').map(str::trim);
            (parts.next() == Some("backgrounded"))
                .then(|| parts.next())
                .flatten()
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| stdout.trim().to_owned())
}
