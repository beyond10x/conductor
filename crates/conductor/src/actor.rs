//! Who issues a command (`conductor.operator.Actor`, design `docs/design/architecture-next.md`
//! § 3 "Actors"): every command input carries its actor, a kind and the fields of that kind.
//!
//! A process records every command under one actor, [`chosen`], which [`crate::cli::Cli::run`]
//! fixes once ([`choose`]): the leaf's `--actor` when it was given; else the run daemon itself
//! (`conductor run`), a [`daemon`]; else the session the command runs in, an [`agent`] of the
//! instance, where the provider that started the session set `CONDUCTOR_SESSION`; else the
//! operator at the terminal, a [`human`] named by the Unix user. A caller that did not fix it (a
//! library test) records as the operator.
//!
//! The tie of a kind to its field (`Human` with `operator`, `Agent` with `agent`, `Daemon` with
//! neither) is the decoder's to check ([`parse`]): the specification leaves it to the handler.

use std::sync::RwLock;

use anyhow::{Result, anyhow, bail};
use conductor_model::agent::{AgentKindName, AgentName};
use conductor_model::config::{Instance, InstanceName, RoleName};
use conductor_model::operator::{Actor, ActorKind, AgentActor, OperatorName};
use serde_json::{Map, Value};

/// The variable a provider sets in the session it starts: the session's name.
pub const SESSION_VARIABLE: &str = "CONDUCTOR_SESSION";

/// The agent kind every session conductor starts runs as in v1 (`conductor.agent`, § 7).
pub const KIND: &str = "claude-code";

/// The role a session is named as when no role of its instance gives it that session name: a
/// repository's controller, whose sessions are named per repository.
const CONTROLLER: &str = "controller";

/// The name a human actor carries when the Unix user is not known.
const OPERATOR: &str = "operator";

/// The actor this process fixed with [`choose`], if it did.
static CHOSEN: RwLock<Option<Actor>> = RwLock::new(None);

/// A person at the terminal or on the dashboard: the operator, named by the Unix user (design
/// § 5.3: "on a single host with no Identity, one operator is implied by the Unix user").
#[must_use]
pub fn human() -> Actor {
    let name = std::env::var("USER")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| OPERATOR.to_owned());
    operator(&name)
}

/// The operator `name`, a human actor.
#[must_use]
pub fn operator(name: &str) -> Actor {
    Actor {
        kind: ActorKind::Human,
        operator: Some(OperatorName(name.to_owned())),
        agent: None,
    }
}

/// The program running as the session `name` of `instance`, in the role whose session name it is,
/// else a controller's.
#[must_use]
pub fn agent(instance: &Instance, name: &str) -> Actor {
    let role = instance
        .roles
        .iter()
        .find(|role| {
            role.session_name
                .as_ref()
                .is_some_and(|session| session.0 == name)
        })
        .map_or_else(|| CONTROLLER.to_owned(), |role| role.role.clone());
    Actor {
        kind: ActorKind::Agent,
        operator: None,
        agent: Some(AgentActor {
            instance: InstanceName(instance.name.0.clone()),
            role: RoleName(role),
            name: AgentName(name.to_owned()),
            kind: AgentKindName(KIND.to_owned()),
        }),
    }
}

/// Conductor acting on its own account: the run daemon's ticks and what they cause.
#[must_use]
pub fn daemon() -> Actor {
    Actor {
        kind: ActorKind::Daemon,
        operator: None,
        agent: None,
    }
}

/// The actor of a process that names none: the calling session as an [`agent`] of the active
/// instance where `CONDUCTOR_SESSION` names one ([`crate::session::actor_of`]), else the operator
/// ([`human`]).
#[must_use]
pub fn default_actor() -> Actor {
    match std::env::var(SESSION_VARIABLE) {
        Ok(session) if !session.trim().is_empty() => crate::session::actor_of(session.trim()),
        _ => human(),
    }
}

/// Fixes the actor of this process: `flag`, the leaf's `--actor`, when given; else the run
/// daemon ([`daemon`]) when `run_daemon`; else [`default_actor`].
///
/// # Errors
///
/// `flag` is not an actor ([`parse`]).
pub fn choose(flag: Option<&str>, run_daemon: bool) -> Result<()> {
    let actor = match flag {
        Some(text) => parse(text).map_err(|error| anyhow!("--actor: {error}"))?,
        None if run_daemon => daemon(),
        None => default_actor(),
    };
    *CHOSEN
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(actor);
    Ok(())
}

/// The actor every command this process records carries: the one [`choose`] fixed, else the
/// operator ([`human`]). A caller that fixed none (a library test) reads no session and no
/// instance for it.
#[must_use]
pub fn chosen() -> Actor {
    CHOSEN
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .unwrap_or_else(human)
}

/// Reads an actor from one JSON object: `kind` (`Human`, `Agent` or `Daemon`), and the field of
/// that kind, `operator` (a name) for `Human`, `agent` (`instance`, `role`, `name`, `kind`) for
/// `Agent`, neither for `Daemon`. A key it does not declare is refused.
///
/// # Errors
///
/// The text is not such an object, naming what is wrong.
pub fn parse(text: &str) -> Result<Actor> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| anyhow!("not a JSON object: {error}"))?;
    let Value::Object(map) = value else {
        bail!("not a JSON object");
    };
    only(&map, &["kind", "operator", "agent"], "")?;
    let kind = match map.get("kind").and_then(Value::as_str) {
        Some("Human") => ActorKind::Human,
        Some("Agent") => ActorKind::Agent,
        Some("Daemon") => ActorKind::Daemon,
        Some(other) => bail!("kind {other:?} is not Human, Agent or Daemon"),
        None => bail!("kind is missing: write Human, Agent or Daemon"),
    };
    let operator = match map.get("operator") {
        None | Some(Value::Null) => None,
        Some(Value::String(name)) if !name.trim().is_empty() => Some(OperatorName(name.clone())),
        Some(other) => bail!("operator {other} is not a name"),
    };
    let agent = match map.get("agent") {
        None | Some(Value::Null) => None,
        Some(Value::Object(agent)) => {
            only(agent, &["instance", "role", "name", "kind"], "agent.")?;
            let field = |key: &str| -> Result<String> {
                agent
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|text| !text.trim().is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| anyhow!("agent.{key} is missing or not text"))
            };
            Some(AgentActor {
                instance: InstanceName(field("instance")?),
                role: RoleName(field("role")?),
                name: AgentName(field("name")?),
                kind: AgentKindName(field("kind")?),
            })
        }
        Some(other) => bail!("agent {other} is not an object"),
    };
    match (kind, &operator, &agent) {
        (ActorKind::Human, Some(_), None)
        | (ActorKind::Agent, None, Some(_))
        | (ActorKind::Daemon, None, None) => {}
        (ActorKind::Human, _, _) => bail!("a Human actor names its operator, and no agent"),
        (ActorKind::Agent, _, _) => bail!("an Agent actor names its agent, and no operator"),
        (ActorKind::Daemon, _, _) => bail!("a Daemon actor names neither operator nor agent"),
    }
    Ok(Actor {
        kind,
        operator,
        agent,
    })
}

/// Refuses a key of `map` outside `keys`, naming it under `prefix`.
fn only(map: &Map<String, Value>, keys: &[&str], prefix: &str) -> Result<()> {
    match map.keys().find(|key| !keys.contains(&key.as_str())) {
        Some(key) => bail!("{prefix}{key} is not a key of an actor"),
        None => Ok(()),
    }
}

/// An actor as the store keeps it beside a record: the object [`parse`] reads.
#[must_use]
pub fn to_json(actor: &Actor) -> Value {
    let mut map = Map::new();
    let kind = match actor.kind {
        ActorKind::Human => "Human",
        ActorKind::Agent => "Agent",
        ActorKind::Daemon => "Daemon",
    };
    map.insert("kind".to_owned(), Value::from(kind));
    if let Some(operator) = &actor.operator {
        map.insert("operator".to_owned(), Value::from(operator.0.clone()));
    }
    if let Some(agent) = &actor.agent {
        map.insert(
            "agent".to_owned(),
            serde_json::json!({
                "instance": agent.instance.0,
                "role": agent.role.0,
                "name": agent.name.0,
                "kind": agent.kind.0,
            }),
        );
    }
    Value::Object(map)
}

/// The actor of a stored record, its `actor` member ([`to_json`]). A record the store kept before
/// commands carried an actor has none, and reads as conductor's own, the [`daemon`]'s: v0 knew
/// no other issuer of a record than conductor itself.
///
/// # Errors
///
/// The member is there and is not an actor.
pub fn stored(body: &Value) -> Result<Actor, String> {
    match body.get("actor") {
        None | Some(Value::Null) => Ok(daemon()),
        Some(value) => parse(&value.to_string()).map_err(|error| format!("actor: {error}")),
    }
}

/// Why a command only a person issues refused `actor` (`conductor.operator.ActorNotHuman`).
#[must_use]
pub fn not_human(actor: &Actor) -> String {
    format!(
        "the actor {} is not a person, and only a human actor issues this command (pass --actor \
         with kind Human for the operator); nothing changed",
        written(actor)
    )
}

/// How the records projection writes an actor (design § 3): `human:<name>`,
/// `agent:<instance>/<role>/<name>` or `daemon`.
#[must_use]
pub fn written(actor: &Actor) -> String {
    match (actor.kind, &actor.operator, &actor.agent) {
        (ActorKind::Human, Some(operator), _) => format!("human:{}", operator.0),
        (ActorKind::Agent, _, Some(agent)) => {
            format!(
                "agent:{}/{}/{}",
                agent.instance.0, agent.role.0, agent.name.0
            )
        }
        (ActorKind::Human, None, _) => "human:".to_owned(),
        (ActorKind::Agent, _, None) => "agent:".to_owned(),
        (ActorKind::Daemon, _, _) => "daemon".to_owned(),
    }
}
