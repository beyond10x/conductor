//! The v1 specification is the generated model (`story:spec-v1`): the seven domains design
//! `docs/design/architecture-next.md` § 3 adds are inputs of the specification and modules of
//! `crates/conductor-model`, every command input carries the actor that issued it, and an actor
//! is a kind (`Human`, `Agent`, `Daemon`) with the fields of that kind (§ 3 "Actors", decision 14).
//!
//! No case here runs the binary or reads the instance of the process: each reads files of this
//! repository, or names a generated type.

use std::fs;
use std::path::PathBuf;

use conductor_model::agent::{AgentKindName, AgentName};
use conductor_model::config::{InstanceName, RoleName};
use conductor_model::decision::{DecisionClass, DecisionId, RecordConductorDecision};
use conductor_model::operator::{Actor, ActorKind, AgentActor, OperatorName};
use conductor_model::primitives::Timestamp;

/// The domains v1 adds, by the last segment of their name: each is `spec/domains/<name>.yaml` and
/// the generated module `conductor_model::<name>`.
const NEW_DOMAINS: [&str; 7] = [
    "instance",
    "placement",
    "agent",
    "store",
    "secret",
    "operator",
    "token",
];

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = repository().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn the_seven_new_domains_are_specification_inputs_and_generated_modules() {
    let inputs = read("spec/ess-inputs.yaml");
    let listed: Vec<&str> = inputs
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .collect();
    let lib = read("crates/conductor-model/src/lib.rs");
    let mut problems = Vec::new();
    for domain in NEW_DOMAINS {
        let file = format!("domains/{domain}.yaml");
        if !listed.contains(&file.as_str()) {
            problems.push(format!("spec/ess-inputs.yaml does not list `{file}`"));
        }
        if !lib
            .lines()
            .any(|line| line.trim() == format!("pub mod {domain};"))
        {
            problems.push(format!(
                "crates/conductor-model/src/lib.rs declares no module `{domain}`"
            ));
        }
        let module = repository().join(format!("crates/conductor-model/src/{domain}.rs"));
        if !module.is_file() {
            problems.push(format!("{} is not a file", module.display()));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The actor of a decision conductor records, read from the generated input of
/// `conductor.decision.RecordConductorDecision`, a command of a v0 domain.
fn actor_of(input: &RecordConductorDecision) -> &Actor {
    &input.actor
}

/// The kind of an actor, read by destructuring: `Actor` is a struct with a field `kind`.
fn kind_of(actor: &Actor) -> ActorKind {
    let Actor { kind, .. } = actor;
    *kind
}

/// The name of each kind. The match names every variant and has no wildcard, so a variant added
/// to or removed from `ActorKind` stops this file from compiling.
fn kind_name(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Human => "Human",
        ActorKind::Agent => "Agent",
        ActorKind::Daemon => "Daemon",
    }
}

#[test]
fn a_command_of_an_existing_domain_carries_its_actor() {
    let input = RecordConductorDecision {
        actor: Actor {
            kind: ActorKind::Agent,
            operator: None,
            agent: Some(AgentActor {
                instance: InstanceName("example-org".to_owned()),
                role: RoleName("controller".to_owned()),
                name: AgentName("example-org-controller".to_owned()),
                kind: AgentKindName("claude-code".to_owned()),
            }),
        },
        decision_id: DecisionId("DEC-20261108-01".to_owned()),
        decision_class: DecisionClass::C,
        question: "which tree".to_owned(),
        options: "A or B".to_owned(),
        choice: "A".to_owned(),
        reason: "reversible".to_owned(),
        evidence: "the brief".to_owned(),
        decided_at: Timestamp("2026-11-08T12:00:00Z".to_owned()),
        repository: None,
        decider_name: None,
        prompt_sha256: None,
    };
    let actor = actor_of(&input);
    assert_eq!(kind_name(kind_of(actor)), "Agent");
    assert_eq!(actor.operator, None);
    assert_eq!(
        actor.agent.as_ref().map(|agent| agent.role.0.as_str()),
        Some("controller")
    );
}

#[test]
fn an_actor_is_a_kind_human_agent_or_daemon() {
    let kinds = [ActorKind::Human, ActorKind::Agent, ActorKind::Daemon];
    let names: Vec<&str> = kinds.into_iter().map(kind_name).collect();
    assert_eq!(names, ["Human", "Agent", "Daemon"]);
    let operator = Actor {
        kind: ActorKind::Human,
        operator: Some(OperatorName("example-operator".to_owned())),
        agent: None,
    };
    let daemon = Actor {
        kind: ActorKind::Daemon,
        operator: None,
        agent: None,
    };
    assert_eq!(kind_name(kind_of(&operator)), "Human");
    assert_eq!(kind_name(kind_of(&daemon)), "Daemon");
}
