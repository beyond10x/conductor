//! The `claude-code` kind (§ 7): today's launcher behind the trait, byte for byte in what it
//! writes. `start` and `stop` are `conductor session start` and `session stop` ([`session::start`],
//! [`session::stop`]): the settings file and `tmux.conf`, one tmux server, the profile appended,
//! the envelope, the terminal log, the hand-over and the guard hook are theirs, unchanged.
//! `deliver` is `session deliver`'s typing and record ([`session::deliver_text`]); `attach`
//! answers the session's tmux server on its launcher socket, and the terminal attach of
//! `conductor agent attach` is `session attach`'s ([`crate::attach::attach`]). Its reports reach
//! conductor through the mailbox (`conductor message send`), not through the kind.

use std::fs;
use std::process::ExitCode;

use anyhow::Result;
use conductor_model::agent::{AgentKindName, AgentNotReachable, Capability};

use super::{Context, Handle, Kind, PtyStream, Refused, Report, Slot};
use crate::cli::{AttachArgs, SessionRoleArgs};
use crate::config;
use crate::confine;
use crate::session;

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
