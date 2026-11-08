//! The unit control conductord drives (`story:daemon-reconcile`), behind [`Units`] so a test runs
//! the daemon against a fake and never against systemd. [`Systemd`] is the user manager: an
//! instance runtime is a transient unit started through `systemd-run --user`
//! ([`systemd::start_service`]); the daemon itself is the installed unit `conductord.service`,
//! enabled and started with `systemctl --user enable --now`.

use std::process::Command;

use anyhow::{Context as _, Result, bail};

use super::host_roles::HostStart;
use crate::systemd::{self, Service};

/// What the daemon asks of the service manager.
pub trait Units {
    /// Starts `service` as a transient unit.
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached.
    fn start(&mut self, service: &Service) -> Result<()>;

    /// Stops the unit `unit`.
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached.
    fn stop(&mut self, unit: &str) -> Result<()>;

    /// Whether the unit `unit` runs: active, activating or reloading.
    ///
    /// # Errors
    ///
    /// The manager cannot be reached.
    fn active(&mut self, unit: &str) -> Result<bool>;

    /// Whether the unit `unit` answers: it runs and is not starting again after a failure. An
    /// upgraded runtime that never gets there is rolled back (`story:instance-upgrade`).
    ///
    /// # Errors
    ///
    /// The manager cannot be reached.
    fn answers(&mut self, unit: &str) -> Result<bool> {
        self.active(unit)
    }

    /// Reads the unit files again, then enables and starts the installed unit `unit`.
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached.
    fn enable(&mut self, unit: &str) -> Result<()>;

    /// Disables and stops the installed unit `unit`, then reads the unit files again.
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached.
    fn disable(&mut self, unit: &str) -> Result<()>;

    /// Starts the host role `start` names at its placement (`story:host-roles-by-daemon`): the
    /// seam a placement adapter takes over. [`Systemd`] runs its `session start` command, which
    /// puts the session in `conductor_host.slice`; a manager that starts no host role refuses.
    ///
    /// # Errors
    ///
    /// The manager refuses, or the command fails.
    fn start_host_role(&mut self, start: &HostStart) -> Result<()> {
        bail!(
            "this service manager starts no host role {} at {}",
            start.role,
            start.placement
        )
    }

    /// [`Units::enable`] on the system manager: the daemon of a box with no local operator
    /// (`story:daemon-remote`).
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached; a manager with no system scope refuses.
    fn enable_system(&mut self, unit: &str) -> Result<()> {
        bail!("this service manager installs no system unit {unit}")
    }

    /// [`Units::disable`] on the system manager (`story:daemon-remote`).
    ///
    /// # Errors
    ///
    /// The manager refuses or cannot be reached; a manager with no system scope refuses.
    fn disable_system(&mut self, unit: &str) -> Result<()> {
        bail!("this service manager removes no system unit {unit}")
    }
}

/// The user's systemd manager.
#[derive(Debug, Default, Clone, Copy)]
pub struct Systemd;

impl Systemd {
    fn systemctl(args: &[&str]) -> Result<()> {
        Self::systemctl_in("--user", args)
    }

    /// `systemctl <scope> <args>`: `--user` or `--system`.
    fn systemctl_in(scope: &str, args: &[&str]) -> Result<()> {
        let output = Command::new("systemctl")
            .arg(scope)
            .args(args)
            .output()
            .with_context(|| format!("run systemctl {scope} {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "systemctl {scope} {} exited {}: {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}

impl Units for Systemd {
    fn start(&mut self, service: &Service) -> Result<()> {
        systemd::start_service(service)
    }

    fn stop(&mut self, unit: &str) -> Result<()> {
        systemd::stop_unit(unit)
    }

    fn active(&mut self, unit: &str) -> Result<bool> {
        let (_, active) = systemd::unit_state(unit)?;
        Ok(matches!(
            active.as_str(),
            "active" | "activating" | "reloading"
        ))
    }

    fn answers(&mut self, unit: &str) -> Result<bool> {
        // A runtime that fails and waits to be started again is `activating`, never `active`.
        let (_, active) = systemd::unit_state(unit)?;
        Ok(active == "active")
    }

    fn start_host_role(&mut self, start: &HostStart) -> Result<()> {
        let Some((program, args)) = start.command.split_first() else {
            bail!("the host role {} has no command", start.role);
        };
        let mut command = Command::new(program);
        command.args(args).current_dir(&start.cwd);
        for set in &start.env {
            if let Some((name, value)) = set.split_once('=') {
                command.env(name, value);
            }
        }
        let output = command
            .stdin(std::process::Stdio::null())
            .output()
            .with_context(|| format!("run {}", start.command.join(" ")))?;
        if !output.status.success() {
            bail!(
                "{} exited {}: {}",
                start.command.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }

    fn enable(&mut self, unit: &str) -> Result<()> {
        Self::systemctl(&["daemon-reload"])?;
        Self::systemctl(&["enable", "--now", unit])
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        Self::systemctl(&["disable", "--now", unit])?;
        Self::systemctl(&["daemon-reload"])
    }

    fn enable_system(&mut self, unit: &str) -> Result<()> {
        Self::systemctl_in("--system", &["daemon-reload"])?;
        Self::systemctl_in("--system", &["enable", "--now", unit])
    }

    fn disable_system(&mut self, unit: &str) -> Result<()> {
        Self::systemctl_in("--system", &["disable", "--now", unit])?;
        Self::systemctl_in("--system", &["daemon-reload"])
    }
}
