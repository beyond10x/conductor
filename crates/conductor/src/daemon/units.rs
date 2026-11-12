//! The unit control conductord drives (`story:daemon-reconcile`), behind [`Units`] so a test runs
//! the daemon against a fake and never against systemd. [`Systemd`] is the user manager: an
//! instance runtime is a transient unit started through `systemd-run --user`
//! ([`systemd::start_service`]); the daemon itself is the installed unit `conductord.service`,
//! enabled and started with `systemctl --user enable --now`. Where no user manager is, as in a
//! container ([`has_user_manager`]), [`Children`] runs each runtime as a child process of the
//! daemon ([`Manager`]).

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

    /// The pod supervisor, where this manager is one (design: instance pods): the reconciler
    /// starts, stops, observes and routes to each runtime through it instead of through the
    /// calls above, since a pod is made from its instance and has an address and a state a unit
    /// lacks ([`super::pods`]). None for every other manager.
    fn pods(&self) -> Option<&super::pods::Pods> {
        None
    }

    /// [`Units::pods`], to change.
    fn pods_mut(&mut self) -> Option<&mut super::pods::Pods> {
        None
    }
}

/// Whether a user systemd manager is there (`story:container-start-without-prompt`): its private
/// socket `<runtime_dir>/systemd/private` is there (`$XDG_RUNTIME_DIR`), or `systemd-run` is in a
/// directory of `search` (`$PATH`). A container with neither has none.
#[must_use]
pub fn has_user_manager(runtime_dir: Option<&OsStr>, search: Option<&OsStr>) -> bool {
    let socket = runtime_dir
        .filter(|dir| !dir.is_empty())
        .is_some_and(|dir| {
            Path::new(dir)
                .join("systemd/private")
                .symlink_metadata()
                .is_ok()
        });
    socket
        || search.is_some_and(|search| {
            std::env::split_paths(search)
                .filter(|dir| !dir.as_os_str().is_empty())
                .any(|dir| dir.join("systemd-run").is_file())
        })
}

/// The service manager the daemon starts runtimes with: the user's systemd manager where there
/// is one ([`has_user_manager`]), else the daemon itself, each runtime a child process of it.
#[derive(Debug, Clone)]
pub enum Manager {
    /// The user's systemd manager.
    Systemd(Systemd),
    /// Child processes of the daemon.
    Children(Children),
    /// One pod per instance in a cluster, where `daemon.kubernetes` is set (design: instance
    /// pods, [`super::pods`]).
    Pods(Box<super::pods::Pods>),
}

impl Manager {
    /// The manager of a host whose `$XDG_RUNTIME_DIR` is `runtime_dir` and whose `$PATH` is
    /// `search`.
    #[must_use]
    pub fn chosen(runtime_dir: Option<&OsStr>, search: Option<&OsStr>) -> Self {
        if has_user_manager(runtime_dir, search) {
            Self::Systemd(Systemd)
        } else {
            Self::Children(Children::default())
        }
    }

    fn units(&mut self) -> &mut dyn Units {
        match self {
            Self::Systemd(systemd) => systemd,
            Self::Children(children) => children,
            Self::Pods(pods) => pods.as_mut(),
        }
    }
}

impl Units for Manager {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.units().start(service)
    }

    fn stop(&mut self, unit: &str) -> Result<()> {
        self.units().stop(unit)
    }

    fn active(&mut self, unit: &str) -> Result<bool> {
        self.units().active(unit)
    }

    fn answers(&mut self, unit: &str) -> Result<bool> {
        self.units().answers(unit)
    }

    fn enable(&mut self, unit: &str) -> Result<()> {
        self.units().enable(unit)
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        self.units().disable(unit)
    }

    fn start_host_role(&mut self, start: &HostStart) -> Result<()> {
        self.units().start_host_role(start)
    }

    fn enable_system(&mut self, unit: &str) -> Result<()> {
        self.units().enable_system(unit)
    }

    fn disable_system(&mut self, unit: &str) -> Result<()> {
        self.units().disable_system(unit)
    }

    fn pods(&self) -> Option<&super::pods::Pods> {
        match self {
            Self::Pods(pods) => Some(pods.as_ref()),
            Self::Systemd(_) | Self::Children(_) => None,
        }
    }

    fn pods_mut(&mut self) -> Option<&mut super::pods::Pods> {
        match self {
            Self::Pods(pods) => Some(pods.as_mut()),
            Self::Systemd(_) | Self::Children(_) => None,
        }
    }
}

/// How long [`Children::stop`] waits for a child to end on `SIGTERM` before it kills it.
const STOP_BOUND: Duration = Duration::from_secs(10);

/// Runtimes as child processes of the daemon, where no user manager is
/// (`story:container-start-without-prompt`). A child runs the service's command in its working
/// directory with the daemon's environment and the service's variables, its standard input
/// closed and its output the daemon's; [`crate::instance::PARENT_VARIABLE`] names the daemon's
/// process, so the runtime ends once the daemon is gone, however it ended. The slice, its limits
/// and the unit's properties are the user manager's and are not applied: a child that exits is
/// started again by the reconciler (`LostRuntime`, under the failed-start rules), not by a
/// restart property. A clone shares the children: the daemon's signal handler stops them
/// ([`Children::stop_all`]).
#[derive(Debug, Default, Clone)]
pub struct Children {
    running: Arc<Mutex<BTreeMap<String, Child>>>,
}

impl Children {
    fn held(&self) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, Child>>> {
        self.running
            .lock()
            .map_err(|_| anyhow::anyhow!("the daemon's children are held by a thread that failed"))
    }

    /// Stops every child, as [`Units::stop`] does each.
    pub fn stop_all(&self) {
        let units: Vec<String> = self
            .held()
            .map(|held| held.keys().cloned().collect())
            .unwrap_or_default();
        for unit in units {
            let _ = self.clone().stop(&unit);
        }
    }
}

/// Ends `child`: `SIGTERM`, then `SIGKILL` once [`STOP_BOUND`] passed.
fn end(child: &mut Child) -> Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    if let Ok(pid) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid),
            nix::sys::signal::Signal::SIGTERM,
        );
    }
    let asked = Instant::now();
    while asked.elapsed() < STOP_BOUND {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.kill()?;
    child.wait()?;
    Ok(())
}

impl Units for Children {
    fn start(&mut self, service: &Service) -> Result<()> {
        let mut held = self.held()?;
        if let Some(child) = held.get_mut(&service.unit)
            && child.try_wait()?.is_none()
        {
            bail!(
                "the runtime {} runs already, as process {}",
                service.unit,
                child.id()
            );
        }
        let Some((program, args)) = service.command.split_first() else {
            bail!("the runtime {} has no command", service.unit);
        };
        let mut command = crate::instance::command(program);
        command.args(args).current_dir(&service.cwd);
        // `NAME` alone passes this process's value, which a child inherits.
        for set in &service.env {
            if let Some((name, value)) = set.split_once('=') {
                command.env(name, value);
            }
        }
        let child = command
            .env(
                crate::instance::PARENT_VARIABLE,
                std::process::id().to_string(),
            )
            .stdin(Stdio::null())
            .spawn()
            .with_context(|| {
                format!(
                    "start the runtime {} as a child: {}",
                    service.unit,
                    service.command.join(" ")
                )
            })?;
        held.insert(service.unit.clone(), child);
        Ok(())
    }

    fn stop(&mut self, unit: &str) -> Result<()> {
        let child = self.held()?.remove(unit);
        match child {
            Some(mut child) => end(&mut child)
                .with_context(|| format!("stop the runtime {unit}, process {}", child.id())),
            None => Ok(()),
        }
    }

    fn active(&mut self, unit: &str) -> Result<bool> {
        let mut held = self.held()?;
        match held.get_mut(unit) {
            Some(child) => Ok(child
                .try_wait()
                .with_context(|| format!("look at the runtime {unit}"))?
                .is_none()),
            None => Ok(false),
        }
    }

    fn enable(&mut self, unit: &str) -> Result<()> {
        bail!("no user service manager here enables the unit {unit}")
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        bail!("no user service manager here disables the unit {unit}")
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
