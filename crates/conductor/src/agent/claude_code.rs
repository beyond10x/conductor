//! The `claude-code` kind (§ 7): today's launcher behind the trait, byte for byte in what it
//! writes. `start` and `stop` are `conductor session start` and `session stop` ([`session::start`],
//! [`session::stop`]): the settings file and `tmux.conf`, one tmux server, the profile appended,
//! the envelope, the terminal log, the hand-over and the guard hook are theirs, unchanged.
//! `deliver` is `session deliver`'s typing and record ([`session::deliver_text`]); `attach`
//! answers the session's tmux server on its launcher socket, and the terminal attach of
//! `conductor agent attach` is `session attach`'s ([`crate::attach::attach`]). Its reports reach
//! conductor through the mailbox (`conductor message send`), not through the kind.
//!
//! What the harness reads at start is the kind's packaging, written into the agent's own state
//! from the binary (`story:roles-in-the-binary`): the role's profile, appended to its system
//! prompt from `<state>/sessions/<session name>.profile.md` ([`write_profile`]), and the settings
//! the director and a controller start with when their role names none ([`settings`]), the files
//! under `crates/conductor/kinds/claude-code/`. No adapter is committed for a role and none is
//! written under `~/.claude/agents/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use conductor_model::agent::{AgentKindName, AgentNotReachable, Capability};

use super::{Context, Handle, Kind, PtyStream, Refused, Report, Slot};
use crate::cli::{AttachArgs, SessionRoleArgs};
use crate::config;
use crate::confine;
use crate::session;

/// The settings the director (`conductor`) starts with when its role names none: the guard as a
/// PreToolUse hook and the lease claim as a SessionStart hook.
pub const CONDUCTOR_SETTINGS: &str =
    include_str!("../../kinds/claude-code/conductor-settings.json");

/// The settings a controller starts with when its role names none: the guard as a PreToolUse hook.
pub const CONTROLLER_SETTINGS: &str =
    include_str!("../../kinds/claude-code/controller-settings.json");

/// The settings the binary carries for `role` when it names none: the director's and the
/// controller's (by either name of each); `None` for every other role, which starts with empty
/// settings.
#[must_use]
pub fn settings(role: &str) -> Option<&'static str> {
    match config::role_name(role)? {
        config::BuiltInRole::Director => Some(CONDUCTOR_SETTINGS),
        config::BuiltInRole::Lead => Some(CONTROLLER_SETTINGS),
        config::BuiltInRole::Maintainer => None,
    }
}

/// The file the session `session`'s appended system prompt is written to:
/// `<state>/sessions/<session>.profile.md`.
#[must_use]
pub fn profile_file(state: &Path, session: &str) -> PathBuf {
    state.join("sessions").join(format!("{session}.profile.md"))
}

/// Writes the profile the binary carries for `role` ([`crate::role::profile`]) to the session's
/// profile file ([`profile_file`]) and answers its path; `None`, writing nothing, for a role the
/// binary carries none for.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_profile(state: &Path, session: &str, role: &str) -> Result<Option<PathBuf>> {
    let Some(text) = crate::role::profile(role) else {
        return Ok(None);
    };
    let file = profile_file(state, session);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(&file, text).with_context(|| format!("write {}", file.display()))?;
    Ok(Some(file))
}

/// Writes the settings the binary carries for `role` ([`settings`]) as they are, pinned to no
/// instance, to the session's settings copy ([`config::pinned_copy`]) and answers its path: for a
/// session started without a config file. `None`, writing nothing, for a role it carries none for.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_settings(state: &Path, session: &str, role: &str) -> Result<Option<PathBuf>> {
    let Some(text) = settings(role) else {
        return Ok(None);
    };
    let file = config::pinned_copy(state, session);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(&file, text).with_context(|| format!("write {}", file.display()))?;
    Ok(Some(file))
}

/// The settings the binary carries for `role` as JSON, or an empty object for a role it carries
/// none for: the base a session's pinned copy is written from.
///
/// # Errors
///
/// The carried text is no JSON (a defect of the build).
pub fn settings_value(role: &str) -> Result<serde_json::Value> {
    settings(role).map_or_else(
        || Ok(serde_json::json!({})),
        |text| serde_json::from_str(text).context("the settings the binary carries are no JSON"),
    )
}

/// The kind's name.
pub const NAME: &str = "claude-code";

/// The `claude-code` kind.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeCode;

/// `conductor session start --role <role>` as it is: the role as the caller wrote it, so its
/// refusals and the note on an old name read as they do today.
///
/// # Errors
///
/// What [`session::start`] answers.
pub fn start_role(context: &Context<'_>, role: &str) -> Result<ExitCode> {
    session::start(
        context.config,
        context.state,
        &SessionRoleArgs {
            role: Some(role.to_owned()),
        },
        context.providers,
    )
}

/// `conductor session stop --role <role>` as it is.
///
/// # Errors
///
/// What [`session::stop`] answers.
pub fn stop_role(context: &Context<'_>, role: &str) -> Result<ExitCode> {
    session::stop(
        context.state,
        &SessionRoleArgs {
            role: Some(role.to_owned()),
        },
        context.providers,
    )
}

/// `conductor session attach --role <role> [--instance <i>] [--watch]` as it is: this terminal on
/// the session's tmux.
///
/// # Errors
///
/// What [`crate::attach::attach`] answers.
pub fn attach_here(
    context: &Context<'_>,
    instance: Option<String>,
    role: &str,
    watch: Option<bool>,
) -> Result<ExitCode> {
    crate::attach::attach(
        context.config,
        context.state,
        &AttachArgs {
            name: None,
            role: Some(role.to_owned()),
            instance,
            watch,
        },
    )
}

/// The role of the active instance whose session name is `name`.
fn role_named(name: &str) -> Option<String> {
    let instance = &config::active().instance;
    let mut roles: Vec<String> = instance
        .roles
        .iter()
        .map(|role| role.role.clone())
        .collect();
    if let Some(maintainer) = config::host_maintainer(instance) {
        roles.push(maintainer.role);
    }
    roles
        .into_iter()
        .find(|role| super::slot(instance, role).is_ok_and(|slot| slot.name.0 == name))
}

impl Kind for ClaudeCode {
    fn name(&self) -> AgentKindName {
        AgentKindName(NAME.to_owned())
    }

    fn binary(&self) -> String {
        confine::CLAUDE.to_owned()
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Interactive]
    }

    fn start(&self, context: &Context<'_>, slot: &Slot) -> Result<Handle, Refused> {
        start_role(context, &slot.role)?;
        Ok(Handle {
            name: slot.name.clone(),
            kind: self.name(),
            id: None,
        })
    }

    fn deliver(&self, context: &Context<'_>, handle: &Handle, text: &str) -> Result<(), Refused> {
        session::deliver_text(context.state, &handle.name.0, text, None)?;
        Ok(())
    }

    fn attach(
        &self,
        context: &Context<'_>,
        handle: &Handle,
        watch: bool,
    ) -> Result<PtyStream, Refused> {
        let sessions = crate::state::dir(context.state)?.join("sessions");
        let socket = sessions.join(format!("{}.tmux", handle.name.0));
        if fs::symlink_metadata(&socket).is_err() {
            return Err(Refused::AgentNotReachable(
                AgentNotReachable {
                    name: handle.name.clone(),
                },
                format!(
                    "its launcher socket {} is absent, so nothing was opened",
                    socket.display()
                ),
            ));
        }
        Ok(PtyStream::Tmux {
            socket,
            session: handle.name.0.clone(),
            watch,
        })
    }

    fn reports(&self, _context: &Context<'_>, _handle: &Handle) -> Result<Vec<Report>, Refused> {
        Ok(Vec::new())
    }

    fn stop(&self, context: &Context<'_>, handle: &Handle) -> Result<(), Refused> {
        let Some(role) = role_named(&handle.name.0) else {
            return Err(Refused::Failed(anyhow::anyhow!(
                "agent {}: no role of the instance has that session name",
                handle.name.0
            )));
        };
        stop_role(context, &role)?;
        Ok(())
    }
}
