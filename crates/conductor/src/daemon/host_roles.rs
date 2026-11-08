//! The host roles under the daemon (`story:host-roles-by-daemon`, design architecture-next § 10):
//! the supervisor and the teacher, one session each on the control host, started and kept running
//! by conductord from `globals.roles`.
//!
//! - A host role runs when `globals.roles` lists it (decision H1: no `teacher.enabled` key; the
//!   teacher runs when the file lists it). Each pass of the reconciler starts a listed role whose
//!   unit does not run, and stops the unit of a host role the file no longer lists
//!   ([`super::reconcile::Daemon::pass`]); a listed role whose unit stopped makes a pass due
//!   (`LostRuntime`).
//! - The daemon starts one through [`super::units::Units::start_host_role`] with a [`HostStart`]:
//!   `<binary> --config <file> --state-dir <daemon dir>/host session start --role <role>` in the
//!   directory of the config file, at the placement [`PLACEMENT`]. That is the seam a placement
//!   adapter replaces: on the systemd path ([`super::units::Systemd`]) the command runs as it is,
//!   and `session start` puts the session's unit in [`crate::systemd::HOST_SLICE`].
//! - `POST /v1/roles/{role}/start` asks the daemon for a listed host role now ([`ask`]); the
//!   daemon's loop takes what was asked ([`take_asked`]) at its next tick. `conductor daemon
//!   start-role --role <role>` is its client, which `task supervisor:start` and `task
//!   teacher:start` run.
//! - `POST /v1/ask` delivers the question to the supervisor's session once it runs ([`Supervisor`],
//!   [`deliver`]).
//!
//! The daemon records no `Reconciliation` for a host role: that record names an instance, and a
//! host role belongs to none. Its actions are answered as [`super::reconcile::Done`] rows whose
//! instance is `host:<role>` and printed by the daemon.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use anyhow::{Result, anyhow};
use conductor_model::config::{Config, Role, SessionName};

use crate::systemd;

/// The supervisor: keeps every instance running and shipping (`.agents/conductor-supervisor.md`).
pub const SUPERVISOR: &str = "supervisor";

/// The teacher: reviews the supervisor from outside (`.agents/conductor-teacher.md`).
pub const TEACHER: &str = "teacher";

/// The host roles, in the order the daemon starts them.
pub const HOST_ROLES: [&str; 2] = [SUPERVISOR, TEACHER];

/// The placement a host role runs at: the control host.
pub const PLACEMENT: &str = "host";

/// The host roles' state directory under the daemon's directory: their sessions' files, sockets
/// and the deliveries to them.
pub const DIR: &str = "host";

/// Whether `role` names a host role.
#[must_use]
pub fn is_host_role(role: &str) -> bool {
    HOST_ROLES.contains(&role)
}

/// The unit a host role's session runs in: `conductor_<role>.service`, its session named after
/// the role, as a global role's is.
#[must_use]
pub fn unit(role: &str) -> String {
    systemd::unit_name(&SessionName(role.to_owned()))
}

/// The host roles' state directory under the daemon's directory `daemon_dir`.
#[must_use]
pub fn state_dir(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join(DIR)
}

/// The host roles `config` lists in `globals.roles`, in [`HOST_ROLES`]'s order.
#[must_use]
pub fn listed(config: &Config) -> Vec<String> {
    HOST_ROLES
        .iter()
        .filter(|role| {
            config
                .globals
                .roles
                .iter()
                .any(|named| named.role == **role)
        })
        .map(|role| (*role).to_owned())
        .collect()
}

/// The role `role` of `globals.roles`, when it is a host role the file lists.
///
/// # Errors
///
/// `role` is no host role, or `globals.roles` does not list it; the message names
/// `globals.roles`.
pub fn global_role(config: &Config, role: &str) -> Result<Role, String> {
    if !is_host_role(role) {
        return Err(format!(
            "{role:?} is no host role: the host roles are {}",
            HOST_ROLES.join(" and ")
        ));
    }
    config
        .globals
        .roles
        .iter()
        .find(|named| named.role == role)
        .cloned()
        .ok_or_else(|| {
            format!("globals.roles lists no role {role}: list it there to run the {role}")
        })
}

/// A host role as `session start` and `session stop` take it: the role `globals.roles` of this
/// process's config file lists ([`global_role`]), with the file's directory, where its session
/// starts, and the file's `globals.limits`, the limits of [`systemd::HOST_SLICE`].
#[derive(Debug, Clone)]
pub struct Found {
    /// The role.
    pub role: Role,
    /// The directory of the config file.
    pub dir: PathBuf,
    /// `globals.limits`.
    pub limits: conductor_model::config::Limits,
}

/// The host role `role` of this process's config file ([`crate::config::config_file`], else the
/// one [`crate::config::locate`] finds).
///
/// # Errors
///
/// No config file is read, or [`global_role`] refuses.
pub fn found(role: &str) -> Result<Found, String> {
    let environment =
        crate::config::Environment::from_process().map_err(|error| format!("{error:#}"))?;
    let loaded = crate::config::load(crate::config::config_file(), &environment)
        .map_err(|error| format!("{error:#}"))?;
    if !loaded.read {
        return Err(format!(
            "no config file is read, so globals.roles lists no role {role}"
        ));
    }
    let path = environment.cwd.join(&loaded.path);
    let named = global_role(&loaded.config, role)?;
    Ok(Found {
        role: named,
        dir: path
            .parent()
            .map_or_else(|| environment.cwd.clone(), Path::to_owned),
        limits: loaded.config.globals.limits,
    })
}

/// The first prompt of a host role's session.
#[must_use]
pub fn prompt(role: &str) -> &'static str {
    if role == TEACHER {
        "Start of a fresh teacher session. Read host/teacher-handoff.md if it is there and the \
         supervisor's profile, run your routine once, report blockers only, then wait for an event."
    } else {
        "Start of a fresh supervisor session. Read host/supervisor-handoff.md if it is there, run \
         your routine once, report blockers only, then wait for an event."
    }
}

/// The profile a host role that names none starts with: `profiles/conductor-<role>.md` beside the
/// config file, which `task profiles` installs. No `~/.claude/agents/` adapter is read.
#[must_use]
pub fn default_profile(config_dir: &Path, role: &str) -> PathBuf {
    config_dir
        .join("profiles")
        .join(format!("conductor-{role}.md"))
}

/// One host role as the daemon asks the service manager to start it: the seam a placement
/// adapter takes over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostStart {
    /// The role, [`SUPERVISOR`] or [`TEACHER`].
    pub role: String,
    /// The placement it runs at, [`PLACEMENT`] in this release.
    pub placement: String,
    /// The unit its session runs in ([`unit`]): what the daemon watches.
    pub unit: String,
    /// Where the command runs: the directory of the config file.
    pub cwd: PathBuf,
    /// `NAME=value` for the command, set over the daemon's own environment.
    pub env: Vec<String>,
    /// `<binary> --config <file> --state-dir <dir> session start --role <role>`.
    pub command: Vec<String>,
}

/// The supervisor's session as `POST /v1/ask` reaches it: the host roles' state directory and
/// its session name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supervisor {
    /// The state directory its session's files are under ([`state_dir`]).
    pub state: PathBuf,
    /// Its session name, [`SUPERVISOR`].
    pub name: String,
}

/// Delivers `text` to the supervisor's session through the `claude-code` kind
/// ([`crate::agent::Kind::deliver`]).
///
/// # Errors
///
/// The kind refuses: the session is not reached, or the delivery cannot be kept.
pub fn deliver(supervisor: &Supervisor, text: &str) -> Result<()> {
    use crate::agent::Kind as _;
    let host = crate::confine::Host;
    let context = crate::agent::Context {
        config: None,
        state: Some(&supervisor.state),
        providers: &host,
    };
    let handle = crate::agent::Handle {
        name: crate::agent::AgentName(supervisor.name.clone()),
        kind: crate::agent::AgentKindName(crate::agent::claude_code::NAME.to_owned()),
        id: None,
    };
    crate::agent::claude_code::ClaudeCode
        .deliver(&context, &handle, text)
        .map_err(|refused| anyhow!("{refused}"))
}

/// The host roles asked for through `POST /v1/roles/{role}/start`, until the loop takes them.
static ASKED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Asks the daemon's loop for the host role `role` at its next tick.
pub fn ask(role: &str) {
    ASKED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(role.to_owned());
}

/// The host roles asked for since the last call, which it empties.
#[must_use]
pub fn take_asked() -> BTreeSet<String> {
    std::mem::take(&mut *ASKED.lock().unwrap_or_else(PoisonError::into_inner))
}
