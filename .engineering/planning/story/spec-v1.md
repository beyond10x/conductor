---
format: aep.planning-md/3
id: story:spec-v1
kind: story
status: implemented
title: the whole v1 system is modelled in the ESS specification before a handler is written
summary: new domains instance, placement, agent, store, secret, operator; config and improvement changed; host shrunk; the components of the two APIs; one scenario per new command; the model regenerated
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor-docs/src/config.rs
- confidence: cited
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/src
- confidence: inferred
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/dashboard/configure.rs
- confidence: cited
  path: crates/conductor/src/dashboard/instances.rs
- confidence: cited
  path: crates/conductor/src/dashboard/sessions.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/registry.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/run/unit.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/decision.yaml
- confidence: cited
  path: spec/domains/direction.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/domains/host.yaml
- confidence: cited
  path: spec/domains/improvement.yaml
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: spec/domains/operator.yaml
- confidence: cited
  path: spec/domains/placement.yaml
- confidence: cited
  path: spec/domains/secret.yaml
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: spec/domains/store.yaml
- confidence: cited
  path: spec/domains/token.yaml
- confidence: cited
  path: spec/domains/tool.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
- confidence: cited
  path: spec/system.yaml
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/config.md
revision: 45
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T04:59:22Z", actor: "human:timo", revision: 45, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-08: "we work spec-first ESS" and "the whole system as visible in the design/stories/pub docs as proposed needs to be modelled in ESS". `AGENTS.md` rule 1 already makes `spec/` the source of truth; design §3 names the domains. Every other story of the epic starts by reading its domain here, so this story goes first on `v1`.

## Acceptance

- `spec/domains/` gains `instance.yaml` (`Instance`, `Reconciliation`, `Reconcile`, `StartInstance`, `StopInstance`, `RestartInstance`, `UpgradeInstance`, `PurgeInstance`, views `Instances`, `Reconciliations`), `placement.yaml` (`Placement`, `Box`, `OpenBox`, `CloseBox`, `PlacementRefused` with substrate's reason names), `agent.yaml` (`AgentKind`, `Capabilities`, `Agent`, `StartAgent`, `DeliverToAgent`, `AttachAgent`, `StopAgent`, the events), `store.yaml` (`Backend` with `sqlite` and `postgres`, `Store` as the one URL of the deployment, `MigrateStore`, view `StoreStatus`; no sync, no restore), `secret.yaml` (`SecretName`, `Source`, `Slot`, `ResolveSecrets`, view `Secrets`) `operator.yaml` (`Actor` as a sum type, `human` holding an `Operator` and `agent` holding an `Agent` reference; `Operator`, `Grant`, `JoinInstance`, `LeaveInstance`, error `ActorNotHuman`, views `Operators`, `Actors`) and `token.yaml` (`TokenPool`, `PoolToken` with owner, seat and state, `ContributeToken`, `WithdrawToken`, `LeaseToken`, `ReleaseToken`, `MarkTokenLimited`, view `Tokens` without values); each with its `naming`, `types`, `entities`, `errors`, `events`, `commands` and `views` sections in the form the existing domains use.
- Every command in every domain carries an `actor: Actor`; every event and every record derived from one carries the same; a scenario per grant-bearing command shows the agent actor refused with `ActorNotHuman` and the human actor accepted (operator, 2026-11-08: "we kind of need to distinguish between who is an agent actor and who is a human actor").
- `config.yaml` gains `Level`, precedence, `EffectiveSettings`, `PutConfig`, `ConfigChanged` and loses `dashboard_port` and every `confinement.*` key; `improvement.yaml` gains `TuningGrant`; `host.yaml` loses `InstanceRegistry`, `RegisteredInstance`, `RegisteredFile`, `DashboardInstance`, `InstanceCollision`, `HostRoleName`, `HostConfig` and `HostRoles` and keeps grants, disk, build runs and reclaim; `session.yaml` is kept as the `claude-code` kind's behaviour and every one of its scenarios still validates.
- `spec/components.yaml` declares `conductord` (owns instance, placement, store, secret, operator, config), `conductor-instance` (owns observation, direction, decision, dispatch, agent, improvement, tool), `conductor-cli`, `conductor-dashboard` and `conductor-tui` with what each accepts; the global and instance APIs of design §5 are those components' accepted commands and views, and every write command names the operator actor.
- `spec/scenarios/` has at least one scenario per new command; `ess specify validate --path spec` exits 0; the conformance run executes them; `task regen` regenerates `crates/conductor-model` and `task check`'s byte comparison passes; `cargo build` of `crates/conductor` still succeeds against the regenerated model (handlers for new commands may exit 2 until their story lands).
- `docs/config.md` and the generated reference pages describe `conductor.config/2` from the specification.

## Scope

Derived 2026-11-08 by `story-scoper`; every line is **cited** (read from the story, the design or the tree) or **inferred**.

- **Primary surface:** `spec/` — cited (story Acceptance; design § 3; `AGENTS.md` rule 1)
- **New domains:** `spec/domains/instance.yaml`, `placement.yaml`, `agent.yaml`, `store.yaml`, `secret.yaml`, `operator.yaml`, `token.yaml` — cited
- **Changed domains:** `spec/domains/config.yaml` (gains `Level`, `EffectiveSettings`, `PutConfig`, `ConfigChanged`; loses `Instance.dashboard_port`, `Instance.confinement`, `ConfinementMode`, `ConfinementProvider`, `Confinement`, the `dashboard_port` invariants), `spec/domains/improvement.yaml` (`TuningGrant`), `spec/domains/host.yaml` (loses `RegisteredInstance`, `RegisteredFile`, `InstanceRegistry`, `DashboardInstance`, `InstanceCollision`, `HostRoleName`, `HostConfig`, `HostRoles`) — cited
- **Actor on every command and event:** `spec/domains/observation.yaml`, `direction.yaml`, `decision.yaml`, `dispatch.yaml`, `session.yaml`, `tool.yaml` — cited (story: "Every command in every domain carries an `actor: Actor`")
- **Also in `config.yaml`:** `ConfigInstance.dashboard_port`, `ProviderAvailability`, `DoctorReport.providers` — inferred (they depend on the removed key and provider)
- **Also in `host.yaml`:** `CollisionKey` — inferred (used only by `InstanceCollision`)
- **System files:** `spec/system.yaml`, `spec/ess-inputs.yaml`, `spec/components.yaml` — cited
- **Scenarios:** new files under `spec/scenarios/`, at least one per new command — cited; the 31 existing scenario files gain the actor on the commands they issue — inferred (depends on `ess` requiring every input field)
- **Generated:** `crates/conductor-model/` (whole tree) — cited (`task regen`)
- **Handlers that break on the removed host types:** `crates/conductor/src/registry.rs`, `crates/conductor/src/dashboard/instances.rs`, `crates/conductor/src/config.rs` — cited (`git grep`)
- **Handlers that break through the removed config fields:** `crates/conductor/src/config.rs`, `confine/mod.rs`, `run/unit.rs`, `session.rs`, `spawn.rs`, `dashboard.rs`, `dashboard/configure.rs`, `guard/rules.rs` — cited (`git grep`)
- **Indirect callers of the host roles:** `crates/conductor/src/run/handlers.rs`, `dashboard/sessions.rs`, `session.rs` — cited (`config::host_maintainer`, `config::host_file`)
- **Handlers that break on the actor field:** about 30 files under `crates/conductor/src/` build generated command structs — inferred
- **CLI tree:** `crates/conductor/src/cli.rs` — inferred (`tests/cli_tree.rs` requires it to equal the `cli:` block of `spec/components.yaml`)
- **Tests:** `crates/conductor/tests/` (config, instance names, dashboard instances, confinement, spawn, run, plus about 30 files that build command structs) — inferred
- **Docs generator:** `crates/conductor-docs/src/config.rs` (its example sets `dashboard_port` and `confinement:`) — cited
- **Documents:** `docs/config.md`; generated `website/docs/reference/config.md`, `website/docs/reference/cli.md` — cited
- **Confidence:** high for `spec/`, the generated tree and the host-type break sites; medium for the size of the handler and test fallout
- **Would collide with:** every unit touching `spec/`, `crates/conductor-model`, `config.rs`, the confinement path or any handler that builds a generated command; nothing else runs beside it, which fits the story going first on `v1`
- **Safety fact:** no handler uses a generated `ports::` module (`git grep 'conductor_model::ports'` matches nothing), so restructuring the components breaks no handler by itself; the breaks come from removed or changed types
