//! `conductor spawn` (`story:spawn`): starts a repository's controller session, and the session
//! end behind `controller stop-controller`.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `spawn`, its input `conductor.dispatch.SpawnController`; each refusal is a
//! `conductor.dispatch.SpawnRefused`, named by its `conductor.dispatch.SpawnRefusal` on one line of
//! standard error, exit 1.
//!
//! It runs the controller start form of the conductor profile (`.agents/conductor.md`), as one
//! program with its arguments and never through a shell, with its working directory the checkout
//! `<checkouts.root>/<repository>`:
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
//! `CONDUCTOR_CONFIG`, so the controller and its guard run as the instance that started it.
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
//!
//! A launch that succeeds appends one `sent` line to the records' dispatch log
//! ([`crate::records`]): the repository as `repo`, the controller's id, the session name as `to`,
//! that session id and the charter revision 1.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{CommandWord, Harness as RoleHarness, Instance, Role};
use conductor_model::dispatch::obligations::{StartControllerBehavior, StopControllerBehavior};
use conductor_model::dispatch::{
    ControllerAlreadyRunning, DispatchLogEvent, SpawnController, SpawnRefusal, SpawnRefused,
    StartController, StartControllerOutcome, StopController, StopControllerOutcome,
};
use conductor_model::observation::{Harness, RepositoryName};
use serde_json::Value;

use crate::cli::{SpawnArgs, StopControllerArgs};
use crate::collect::{self, Under};
use crate::config::{self, Environment};
use crate::not_implemented;

const COMMAND: &str = "spawn";

/// The program `spawn` starts, found on `PATH`.
const CLAUDE: &str = "claude";

/// How long the session list and the launch may each take before they are stopped.
const BOUND: Duration = Duration::from_secs(120);

/// `conductor spawn`: starts the controller session of `--repository` and records the controller
/// `Running`, as the module documentation says. Prints what `claude --bg` printed.
///
/// # Errors
///
/// `--repository` is missing, the environment or the store cannot be read, the record is not
/// kept, standard output cannot be written, or a refusal the module documentation names.
pub fn spawn(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &SpawnArgs,
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
    let (profile, settings, model) = match role(instance) {
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
        outcome
    };
    let controller_id = match outcome {
        StartControllerOutcome::Started { controller_started } => controller_started.controller_id,
        StartControllerOutcome::AlreadyRunning {
            error: ControllerAlreadyRunning { .. },
        } => {
            return refuse(
                SpawnRefusal::AlreadyRunning,
                "the repository already has a running or paused controller (controller \
                 controllers)"
                    .to_owned(),
            );
        }
    };

    let mut launch = Command::new(CLAUDE);
    launch
        .arg("--bg")
        .arg("-n")
        .arg(&session_name.0)
        .arg("--append-system-prompt-file")
        .arg(profile)
        .arg("--model")
        .arg(model)
        .args(["--permission-mode", "bypassPermissions"])
        .args(["--setting-sources", "project,local"])
        .arg("--settings")
        .arg(settings)
        .arg(&charter)
        .current_dir(&checkout);
    if active.from_file {
        // Absolute, as the guard resolves it: the session runs in the checkout, where a relative
        // path would name a file of the repository.
        let (file, _) = config::locate(config_flag, &environment).context(COMMAND)?;
        launch
            .env(config::INSTANCE_VARIABLE, &instance.name.0)
            .env(config::CONFIG_VARIABLE, environment.cwd.join(file));
    }
    let failure = match run_launch(&mut launch) {
        Ok(output) if output.status.success() => {
            let mut err = io::stderr().lock();
            err.write_all(&output.stderr)
                .context("write what claude said to standard error")?;
            io::stdout()
                .lock()
                .write_all(&output.stdout)
                .context("write the session id to standard output")?;
            let mut line = crate::records::dispatch_line(DispatchLogEvent::Sent, &repository)?;
            line.controller_id = Some(controller_id.0.0.clone());
            line.to = Some(session_name.0.clone());
            line.session_id = Some(session_id(&String::from_utf8_lossy(&output.stdout)));
            line.charter_revision = Some(1);
            crate::records::append_dispatch(&line).with_context(|| {
                format!(
                    "{COMMAND}: the session started and controller {} is recorded",
                    controller_id.0.0
                )
            })?;
            return Ok(ExitCode::SUCCESS);
        }
        Ok(output) => exited(&output),
        Err(error) => format!("{error:#}"),
    };
    // A launch can fail, or be stopped at the bound, after the session started: the session list,
    // read again, decides whether the controller is recorded stopped.
    match live_session(&session_name.0) {
        Ok(None) => {}
        Ok(Some(id)) => {
            return refuse(
                SpawnRefusal::LaunchFailed,
                format!(
                    "{CLAUDE} --bg {failure}; yet the live session {id} holds the session name \
                     {}, so the controller {} stays recorded running",
                    session_name.0, controller_id.0.0
                ),
            );
        }
        Err(error) => {
            return refuse(
                SpawnRefusal::LaunchFailed,
                format!(
                    "{CLAUDE} --bg {failure}; the session list could not be read again \
                     ({error:#}), so the controller {} stays recorded running: stop it with \
                     conductor controller stop-controller once no session {} runs",
                    controller_id.0.0, session_name.0
                ),
            );
        }
    }
    let reason = format!("{COMMAND}: the session did not start: {failure}");
    let mut generated = crate::controller::record(state)?;
    let outcome = generated
        .stop_controller(StopController {
            controller_id,
            reason,
        })
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    generated.ports.store.check().context("keep the stop")?;
    match outcome {
        StopControllerOutcome::Stopped { .. } => {}
        StopControllerOutcome::NoSuchController { .. }
        | StopControllerOutcome::WrongState { .. } => {
            bail!("{COMMAND}: the controller it recorded could not be recorded stopped");
        }
    }
    refuse(
        SpawnRefusal::LaunchFailed,
        format!("{CLAUDE} --bg {failure}; the controller is recorded stopped"),
    )
}

/// Ends the controller session `args.controller_id` names.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented): no command ends a session yet.
pub fn end(_args: &StopControllerArgs) -> Result<ExitCode> {
    not_implemented("spawn end")
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

/// The `controller` role's profile, settings and model, each file checked to be one.
fn role(instance: &Instance) -> Result<(String, String, String), (SpawnRefusal, String)> {
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
    Ok((profile, settings, model.0.clone()))
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

/// The id of a live session the Claude session list (`claude agents --json`) names `name`: an
/// entry with a `pid`, as the sessions collector counts one live, `id` else `sessionId`.
fn live_session(name: &str) -> Result<Option<String>> {
    let shown = format!("{CLAUDE} agents --json");
    let output = collect::run(Command::new(CLAUDE).args(["agents", "--json"]), BOUND)
        .with_context(|| format!("read the Claude session list `{shown}`"))?;
    if !output.status.success() {
        bail!("`{shown}` {}", exited(&output));
    }
    let Value::Array(entries) = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON"))?
    else {
        bail!("`{shown}` printed no JSON array");
    };
    Ok(entries
        .iter()
        .find(|entry| !entry["pid"].is_null() && entry["name"].as_str() == Some(name))
        .map(|entry| {
            entry["id"]
                .as_str()
                .or_else(|| entry["sessionId"].as_str())
                .unwrap_or("with no id")
                .to_owned()
        }))
}

/// Runs the launch `command` to its end within [`BOUND`], with no standard input, and answers its
/// exit status and output. Its standard output and error go to files of this process's own, not
/// to pipes: `claude --bg` leaves a session running, which may keep what it inherited open, and a
/// pipe would not reach its end before that session does. The files are removed afterwards. A
/// launch still running after the bound is killed, and the answer is an error.
fn run_launch(command: &mut Command) -> Result<Output> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let base = format!("conductor-spawn.{}.{stamp}", std::process::id());
    let files = ["out", "err"].map(|what| std::env::temp_dir().join(format!("{base}.{what}")));
    let ran = (|| -> Result<Output> {
        let mut streams = Vec::with_capacity(files.len());
        for file in &files {
            streams.push(
                File::options()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(file)
                    .with_context(|| format!("create {}", file.display()))?,
            );
        }
        let stderr = streams.pop().map(Stdio::from).unwrap_or_else(Stdio::null);
        let stdout = streams.pop().map(Stdio::from).unwrap_or_else(Stdio::null);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .with_context(|| format!("start `{CLAUDE}`"))?;
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .with_context(|| format!("wait for `{CLAUDE}`"))?
            {
                break status;
            }
            if started.elapsed() >= BOUND {
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "gave no answer within {} s and was stopped",
                    BOUND.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(20));
        };
        let [out, err] = &files;
        Ok(Output {
            status,
            stdout: fs::read(out).with_context(|| format!("read {}", out.display()))?,
            stderr: fs::read(err).with_context(|| format!("read {}", err.display()))?,
        })
    })();
    for file in &files {
        let _ = fs::remove_file(file);
    }
    ran
}

/// How a program that did not succeed ended, with what it wrote to standard error on one line.
fn exited(output: &Output) -> String {
    let said: Vec<&str> = std::str::from_utf8(&output.stderr)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let how = output.status.code().map_or_else(
        || "was ended by a signal".to_owned(),
        |code| format!("exited {code}"),
    );
    if said.is_empty() {
        how
    } else {
        format!("{how}: {}", said.join("; "))
    }
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
