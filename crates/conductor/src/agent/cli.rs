//! `conductor agent start|deliver|attach|stop --instance <i> --role <r>` (`story:agent-kinds`):
//! each goes through the role's kind (`roles.<role>.kind`, `claude-code` by default). For
//! `claude-code` it is today's `session` command, byte for byte ([`super::claude_code`]);
//! `conductor session start` and `session stop` are aliases of [`start`] and [`stop`] for this
//! release, and go through the same path. A kind this binary does not implement is refused as
//! `UnknownAgentKind`, a kind that is not interactive as `CapabilityMissing` on `deliver` and
//! `attach`; each refusal is one line naming the error, exit 1.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};
use conductor_model::agent::{AgentKindName, Capability};

use super::claude_code;
use super::{Context, Kind, Refused, built_in_kind, find_role, slot};
use crate::cli::{AgentAttachArgs, AgentDeliverArgs, AgentSlotArgs, DeliverArgs};
use crate::config;

/// The kind of the role `role` of the active instance: its `kind`; `claude-code` for a role the
/// instance does not name, whose refusal is then `session start`'s own (`UnknownRole`).
fn kind_of(command: &str, role: &str) -> Result<Box<dyn Kind>> {
    let instance = &config::active().instance;
    let name = find_role(instance, role).map_or_else(
        || AgentKindName(claude_code::NAME.to_owned()),
        |found| found.kind,
    );
    match built_in_kind(&name.0) {
        Some(kind) => Ok(kind),
        None => bail!("{command}: {}", Refused::unknown_kind(&name)),
    }
}

fn is_claude_code(kind: &dyn Kind) -> bool {
    kind.name().0 == claude_code::NAME
}

fn role_of(command: &str, role: Option<&String>) -> Result<String> {
    match role {
        Some(role) => Ok(role.clone()),
        None => bail!("{command}: --role is required"),
    }
}

/// A refusal of `command`, one line.
fn refused(command: &str, refusal: Refused) -> anyhow::Error {
    match refusal {
        Refused::Failed(error) => error,
        refusal => anyhow::anyhow!("{command}: {refusal}"),
    }
}

/// `conductor agent start` (`command` `agent start`) and its alias `conductor session start`
/// (`session start`): the slot of `--role` started through its kind.
///
/// # Errors
///
/// `--role` is missing, the kind is not implemented, or the kind's refusal.
pub fn start(context: &Context<'_>, args: &AgentSlotArgs, command: &str) -> Result<ExitCode> {
    let role = role_of(command, args.role.as_ref())?;
    let kind = kind_of(command, &role)?;
    if is_claude_code(kind.as_ref()) {
        return claude_code::start_role(context, &role);
    }
    let found = slot(&config::active().instance, &role)
        .map_err(|detail| anyhow::anyhow!("{command}: {detail}"))?;
    let handle = kind
        .start(context, &found)
        .map_err(|refusal| refused(command, refusal))?;
    println!("started {}: kind {}", handle.name.0, handle.kind.0);
    Ok(ExitCode::SUCCESS)
}

/// `conductor agent stop` and its alias `conductor session stop`.
///
/// # Errors
///
/// `--role` is missing, the kind is not implemented, or the kind's refusal.
pub fn stop(context: &Context<'_>, args: &AgentSlotArgs, command: &str) -> Result<ExitCode> {
    let role = role_of(command, args.role.as_ref())?;
    let kind = kind_of(command, &role)?;
    if is_claude_code(kind.as_ref()) {
        return claude_code::stop_role(context, &role);
    }
    let found = slot(&config::active().instance, &role)
        .map_err(|detail| anyhow::anyhow!("{command}: {detail}"))?;
    let handle = super::Handle {
        name: found.name.clone(),
        kind: found.kind.clone(),
        id: None,
    };
    kind.stop(context, &handle)
        .map_err(|refusal| refused(command, refusal))?;
    println!("stopped {}", handle.name.0);
    Ok(ExitCode::SUCCESS)
}

/// The kind of `role`, refused unless it is interactive.
fn interactive(command: &str, role: &str) -> Result<Box<dyn Kind>> {
    let kind = kind_of(command, role)?;
    if !kind.capabilities().contains(&Capability::Interactive) {
        let name = slot(&config::active().instance, role).map_or_else(
            |_| conductor_model::agent::AgentName(role.to_owned()),
            |found| found.name,
        );
        bail!(
            "{command}: {}",
            Refused::capability_missing(&name, Capability::Interactive)
        );
    }
    Ok(kind)
}

/// `conductor agent deliver --role <r> --text <file|->`: for `claude-code`, `conductor session
/// deliver` to the role's session name.
///
/// # Errors
///
/// A flag is missing, the instance names no such role, the kind is not implemented or not
/// interactive, or `session deliver`'s refusal.
pub fn deliver(state: Option<&Path>, args: &AgentDeliverArgs) -> Result<ExitCode> {
    const COMMAND: &str = "agent deliver";
    let role = role_of(COMMAND, args.role.as_ref())?;
    let Some(text) = args.text.clone() else {
        bail!("{COMMAND}: --text is required");
    };
    let kind = interactive(COMMAND, &role)?;
    let found = slot(&config::active().instance, &role).map_err(|detail| {
        anyhow::anyhow!("{COMMAND}: refused: AgentNotFound: role {role:?}: {detail}")
    })?;
    if is_claude_code(kind.as_ref()) {
        return crate::session::deliver(
            state,
            &DeliverArgs {
                name: Some(found.name.0),
                text: Some(text),
            },
        );
    }
    bail!(
        "{COMMAND}: kind {} delivers through the instance API only",
        kind.name().0
    )
}

/// `conductor agent attach --role <r> [--watch]`: for `claude-code`, `conductor session attach
/// --role <r>` on the instance.
///
/// # Errors
///
/// `--role` is missing, the kind is not implemented or not interactive, or `session attach`'s
/// refusal.
pub fn attach(context: &Context<'_>, args: &AgentAttachArgs) -> Result<ExitCode> {
    const COMMAND: &str = "agent attach";
    let role = role_of(COMMAND, args.role.as_ref())?;
    let kind = interactive(COMMAND, &role)?;
    if is_claude_code(kind.as_ref()) {
        return claude_code::attach_here(context, args.instance.clone(), &role, args.watch);
    }
    bail!(
        "{COMMAND}: kind {} attaches through the instance API only",
        kind.name().0
    )
}
