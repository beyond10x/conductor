//! The unit control conductord drives (`story:daemon-reconcile`), behind [`Units`] so a test runs
//! the daemon against a fake and never against systemd. [`Systemd`] is the user manager: an
//! instance runtime is a transient unit started through `systemd-run --user`
//! ([`systemd::start_service`]); the daemon itself is the installed unit `conductord.service`,
//! enabled and started with `systemctl --user enable --now`.

use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::confine::systemd::{self, Service};

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
}

/// The user's systemd manager.
#[derive(Debug, Default, Clone, Copy)]
pub struct Systemd;

impl Systemd {
    fn systemctl(args: &[&str]) -> Result<()> {
        let output = Command::new("systemctl")
            .arg("--user")
            .args(args)
            .output()
            .with_context(|| format!("run systemctl --user {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "systemctl --user {} exited {}: {}",
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

    fn enable(&mut self, unit: &str) -> Result<()> {
        Self::systemctl(&["daemon-reload"])?;
        Self::systemctl(&["enable", "--now", unit])
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        Self::systemctl(&["disable", "--now", unit])?;
        Self::systemctl(&["daemon-reload"])
    }
}
