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
//! written under `~/.claude/agents/`. Where the instance has standing rules
//! (story:instance-rules-text), the same file holds them after the profile, framed
//! ([`standing_rules_block`]); a role with a profile file of its own gets a copy of it with them
//! ([`write_own`]), and the file it names is not changed.

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

/// The tools the director never calls, which its start denies so their definitions leave its
/// first-turn context (`story:conductor-context-budget`): the list its settings deny too.
pub const DIRECTOR_DENIED_TOOLS: [&str; 7] = [
    "Artifact",
    "Workflow",
    "ScheduleWakeup",
    "EnterWorktree",
    "AskUserQuestion",
    "ReportFindings",
    "ShareOnboardingGuide",
];

/// The tools the maintainer never calls: the director's but `Workflow` and `EnterWorktree`, which
/// a wave may call.
pub const MAINTAINER_DENIED_TOOLS: [&str; 5] = [
    "Artifact",
    "ScheduleWakeup",
    "AskUserQuestion",
    "ReportFindings",
    "ShareOnboardingGuide",
];

/// The tools a session of `role` starts without (`--disallowedTools`), by either name of the
/// role: the director's and the maintainer's; none for every other role.
#[must_use]
pub fn denied_tools(role: &str) -> &'static [&'static str] {
    match config::role_name(role) {
        Some(config::BuiltInRole::Director) => &DIRECTOR_DENIED_TOOLS,
        Some(config::BuiltInRole::Maintainer) => &MAINTAINER_DENIED_TOOLS,
        Some(config::BuiltInRole::Lead) | None => &[],
    }
}

/// The file the session `session`'s appended system prompt is written to:
/// `<state>/sessions/<session>.profile.md`.
#[must_use]
pub fn profile_file(state: &Path, session: &str) -> PathBuf {
    state.join("sessions").join(format!("{session}.profile.md"))
}

/// The line before the instance's standing rules in a session's prompt file
/// (story:instance-rules-text), which says what they are and names the length of the fence around
/// them.
#[must_use]
pub fn standing_rules_heading(fence: usize) -> String {
    format!(
        "The standing rules of this instance follow, verbatim, between the two lines of {fence} \
         backticks; they hold for every session of the instance."
    )
}

/// The block of the instance's standing rules `rules` in a session's prompt file
/// (story:instance-rules-text): [`standing_rules_heading`], a blank line, and `rules` in a fenced
/// block whose fence is longer than any run of backticks they hold ([`crate::spawn::fenced`], the
/// frame of the work call's brief), so no line of them closes the block.
#[must_use]
pub fn standing_rules_block(rules: &str) -> String {
    standing_rules_block_after(rules, 0)
}

/// [`standing_rules_block`] with a fence longer than `longer_than` backticks as well.
fn standing_rules_block_after(rules: &str, longer_than: usize) -> String {
    let (fence, block) = crate::spawn::fenced_longer(rules, longer_than);
    format!("{}\n\n{block}", standing_rules_heading(fence))
}

/// The fenced code block `text` leaves open at its end, as CommonMark reads fences: the
/// character and the length of its opening fence; `None` where every fenced block it opens is
/// closed.
fn open_fence(text: &str) -> Option<(char, usize)> {
    let mut open: Option<(char, usize)> = None;
    for line in text.lines() {
        let rest = line.trim_start_matches(' ');
        if line.len() - rest.len() > 3 {
            continue;
        }
        let Some(mark) = rest.chars().next().filter(|c| *c == '`' || *c == '~') else {
            continue;
        };
        let run = rest.chars().take_while(|c| *c == mark).count();
        match open {
            None if run >= 3 && !(mark == '`' && rest[run..].contains('`')) => {
                open = Some((mark, run));
            }
            Some((held, length))
                if held == mark && run >= length && rest[run..].trim().is_empty() =>
            {
                open = None;
            }
            _ => {}
        }
    }
    open
}

/// The text of a session's prompt file: `base`, the profile it starts from, where it starts from
/// one, and after it, where the instance has standing rules, their block
/// ([`standing_rules_block`]); `None` where it has neither. Rules that are empty or only white
/// space are none ([`crate::config::standing_rules_given`]): without standing rules it is `base`,
/// byte for byte. After a profile, the rules' fence is longer than any run of backticks in the
/// profile too, and a fenced block the profile leaves open is closed before the heading line, so
/// the heading and the rules stand outside it.
#[must_use]
pub fn prompt_text(base: Option<&str>, rules: Option<&str>) -> Option<String> {
    let rules = rules.and_then(crate::config::standing_rules_given);
    match (base, rules) {
        (base, None) => base.map(str::to_owned),
        (None, Some(rules)) => Some(standing_rules_block(rules)),
        (Some(base), Some(rules)) => {
            let base = base.trim_end();
            let close = open_fence(base)
                .map(|(mark, length)| format!("\n{}", mark.to_string().repeat(length)))
                .unwrap_or_default();
            let longest = base.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            Some(format!(
                "{base}{close}\n\n{}",
                standing_rules_block_after(rules, longest)
            ))
        }
    }
}

/// Writes [`prompt_text`] of `base` and `rules` to the session's profile file ([`profile_file`])
/// and answers its path; `None`, writing nothing, where there is neither.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_prompt(
    state: &Path,
    session: &str,
    base: Option<&str>,
    rules: Option<&str>,
) -> Result<Option<PathBuf>> {
    let Some(text) = prompt_text(base, rules) else {
        return Ok(None);
    };
    let file = profile_file(state, session);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(&file, text).with_context(|| format!("write {}", file.display()))?;
    Ok(Some(file))
}

/// Writes the profile the binary carries for `role` ([`crate::role::profile`]), followed by the
/// block of the instance's standing rules `rules` where it has them, to the session's profile
/// file ([`profile_file`]) and answers its path; `None`, writing nothing, for a role the binary
/// carries none for and an instance without standing rules.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_profile(
    state: &Path,
    session: &str,
    role: &str,
    rules: Option<&str>,
) -> Result<Option<PathBuf>> {
    write_prompt(state, session, crate::role::profile(role), rules)
}

/// Writes the role's own profile file `own`, followed by the block of the instance's standing
/// rules `rules`, to the session's profile file ([`profile_file`]) and answers its path: a copy in
/// the session's state, so the file the role names is not changed.
///
/// # Errors
///
/// `own` cannot be read, or the directory or the file cannot be written.
pub fn write_own(state: &Path, session: &str, own: &Path, rules: &str) -> Result<PathBuf> {
    let base = fs::read_to_string(own).with_context(|| format!("read {}", own.display()))?;
    let file = write_prompt(state, session, Some(&base), Some(rules))?;
    Ok(file.unwrap_or_else(|| profile_file(state, session)))
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
        let _ = start_role(context, &slot.role)?;
        Ok(Handle {
            name: slot.name.clone(),
            kind: self.name(),
            id: None,
        })
    }

    fn deliver(&self, context: &Context<'_>, handle: &Handle, text: &str) -> Result<(), Refused> {
        let _ = session::deliver_text(context.state, &handle.name.0, text, None)?;
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
        let _ = stop_role(context, &role)?;
        Ok(())
    }
}
