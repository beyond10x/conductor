//! `conductor run --install` and `--uninstall` (`story:run-daemon`): the daemon as the user unit
//! `conductor_<instance>-run.service` ([`unit_name`]).
//!
//! Under the `systemd` provider (`confinement.provider`), `--install` starts it through `systemd-run
//! --user` as a session's unit is started ([`systemd::start_service`]): in the instance's slice,
//! restarted 30 s after a failure, in the state directory, with this process's `PATH`, the
//! instance's name and config file in `CONDUCTOR_INSTANCE` and `CONDUCTOR_CONFIG`, each role's
//! `forge_token_env` variable this process has, by name, and running this binary's `run`, with the
//! global `--state-dir` when it was given. It refuses while a unit of that name runs. Under the
//! `none` provider it starts nothing and prints the command to run under a supervisor of the
//! operator's. `--uninstall` stops the unit (`systemctl --user stop`), and says `not installed`
//! when systemd has no unit of that name loaded. `conductor doctor` reports the unit's state.

use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use conductor_model::config::{ConfinementProvider, Instance, SessionName};

use crate::config::{self, Environment};
use crate::confine::systemd::{self, Service};

/// The user unit `conductor run` runs in: `conductor_<instance>-run.service`, escaped as a
/// session's unit is ([`systemd::unit_name`]).
#[must_use]
pub fn unit_name(instance: &Instance) -> String {
    systemd::unit_name(&SessionName(format!("{}-run", instance.name.0)))
}

/// `conductor run --install`, as the module documentation says.
///
/// # Errors
///
/// The config file or this binary's path cannot be told, a unit of that name runs, or
/// `systemd-run` fails.
pub fn install(config_flag: Option<&Path>, state: Option<&Path>) -> Result<ExitCode> {
    const WHAT: &str = "run --install";
    let active = config::active();
    let instance = &active.instance;
    let unit = unit_name(instance);
    let program = std::env::current_exe().context("run --install: read the conductor binary")?;
    let mut env = Vec::new();
    if active.from_file {
        let environment = Environment::from_process().context(WHAT)?;
        let (file, _) = config::locate(config_flag, &environment).context(WHAT)?;
        env.push(format!(
            "{}={}",
            config::CONFIG_VARIABLE,
            environment.cwd.join(file).display()
        ));
        env.push(format!("{}={}", config::INSTANCE_VARIABLE, instance.name.0));
    }
    let mut command = vec![program.display().to_string()];
    if let Some(state) = state {
        command.push("--state-dir".to_owned());
        command.push(state.display().to_string());
    }
    command.push("run".to_owned());
    let mut out = io::stdout().lock();
    if instance.confinement.provider == ConfinementProvider::None {
        let mut words = env.clone();
        words.extend(command);
        writeln!(
            out,
            "confinement.provider is none: run this under a supervisor of your own:\n{}",
            words.join(" ")
        )
        .context(WHAT)?;
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(path) = std::env::var_os("PATH") {
        env.push(format!("PATH={}", path.to_string_lossy()));
    }
    for role in &instance.roles {
        for name in &role.forge_token_env {
            if std::env::var_os(name).is_some() && !env.contains(name) {
                env.push(name.clone());
            }
        }
    }
    let cwd = crate::state::dir(state)?;
    systemd::start_service(&Service {
        unit: unit.clone(),
        slice: systemd::slice_name(
            instance
                .session_prefix
                .as_ref()
                .map(|prefix| prefix.0.as_str()),
        ),
        limits: instance.confinement.limits.clone(),
        cwd,
        properties: ["Restart=on-failure", "RestartSec=30"]
            .map(str::to_owned)
            .to_vec(),
        env,
        command: command.clone(),
    })
    .context(WHAT)?;
    writeln!(out, "started {unit}: {}", command.join(" ")).context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor run --uninstall`, as the module documentation says.
///
/// # Errors
///
/// `systemctl` fails.
pub fn uninstall() -> Result<ExitCode> {
    const WHAT: &str = "run --uninstall";
    let unit = unit_name(&config::active().instance);
    let (load, _) = systemd::unit_state(&unit).context(WHAT)?;
    let mut out = io::stdout().lock();
    if load == "not-found" || load.is_empty() {
        writeln!(out, "{unit}: not installed").context(WHAT)?;
        return Ok(ExitCode::SUCCESS);
    }
    systemd::stop_unit(&unit).context(WHAT)?;
    writeln!(out, "stopped {unit}").context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}
