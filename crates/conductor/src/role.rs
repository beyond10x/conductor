//! The role profiles the binary carries (`story:roles-in-the-binary`): product data, one Markdown
//! file per role under `crates/conductor/roles/`, compiled in with `include_str!`, so a role
//! always starts from the text of the binary that starts it and nothing is copied to the host.
//! `conductor role show --role <role>` prints one (`conductor.agent.ShowRole`, over the binding's
//! `local` callable `role-show`); the `claude-code` kind writes one into the agent's own state
//! when it starts an agent ([`crate::agent::claude_code::write_profile`]).

use std::io::{self, Write as _};
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use conductor_model::agent::ShowRole;

use crate::cli::RoleShowArgs;
use crate::config::{self, BuiltInRole};

/// Each role the binary carries a profile for, by its name, with the profile's text.
pub const PROFILES: [(&str, &str); 4] = [
    ("maintainer", include_str!("../roles/maintainer.md")),
    ("supervisor", include_str!("../roles/supervisor.md")),
    ("teacher", include_str!("../roles/teacher.md")),
    ("controller", include_str!("../roles/controller.md")),
];

/// The name of the profile `role` starts from: a profile's own name, or the role an old name of
/// it names (`conductor-dev` the maintainer, `lead` the controller); `None` for a role the binary
/// carries no profile for, such as `director`.
#[must_use]
pub fn profile_name(role: &str) -> Option<&'static str> {
    if let Some((name, _)) = PROFILES.iter().find(|(name, _)| *name == role) {
        return Some(name);
    }
    match config::role_name(role)? {
        BuiltInRole::Maintainer => Some("maintainer"),
        BuiltInRole::Lead => Some("controller"),
        BuiltInRole::Director => None,
    }
}

/// The profile the binary carries for `role` ([`profile_name`]).
#[must_use]
pub fn profile(role: &str) -> Option<&'static str> {
    let name = profile_name(role)?;
    PROFILES
        .iter()
        .find(|(each, _)| *each == name)
        .map(|(_, text)| *text)
}

/// `conductor role show --role <role>`: prints the profile the binary carries for the role, byte
/// for byte; reads no config and no store.
///
/// # Errors
///
/// No role is given, or the binary carries no profile for it (one line, exit 1).
pub fn role_show(args: &RoleShowArgs) -> Result<ExitCode> {
    let input = ShowRole {
        role: args.role.clone(),
    };
    let Some(role) = input.role else {
        bail!("role show: --role is required");
    };
    let Some(text) = profile(&role) else {
        let names: Vec<&str> = PROFILES.iter().map(|(name, _)| *name).collect();
        bail!(
            "role show: the binary carries no profile for the role {role:?}; it carries {}",
            names.join(", ")
        );
    };
    io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .context("write the profile")?;
    Ok(ExitCode::SUCCESS)
}
