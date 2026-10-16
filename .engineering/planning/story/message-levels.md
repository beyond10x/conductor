---
format: aep.planning-md/3
id: story:message-levels
kind: story
status: draft
title: The guard refuses and records a message that skips a level
relations:
- decomposes: epic:role-names
- depends_on: story:role-name-aliases
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference
revision: 3
---
# Story: message-levels

## Outcome

The guard refuses and records a SendMessage that skips a level: a session messages only one level
up or one level down (host, instance, repository, unit).

## Context

Design: `docs/design/roles-and-surfaces.md` § 2, rules of the levels ("A message goes one level up
or down. A worker never messages a director, and a lead never messages the bridge"), approved by
the operator on 2026-10-16; `docs/design/conductor.md` § 3 (topology) and § 7 (the guard).
Today `crates/conductor/src/guard/rules.rs` lets a controller message only its instance's
conductor or an agent it started, and lets conductor message any session of the instances it
serves, the maintainer (`<prefix>-conductor-dev`) among them as a session of its instance.

Levels and who may message whom:

| sender (level) | may message |
|---|---|
| maintainer (host) | the operator; each director |
| director (instance) | the maintainer; its own instance's leads |
| lead (repository) | its own instance's director; the workers it started |
| worker (unit) | its lead |

## Domain relations

- Role → Level, many-to-one, the level owns nothing — inferable from
  `docs/design/roles-and-surfaces.md` § 2 (operator-approved); no `ess/1` document declares it, and
  this story declares it first.
- Director → Lead, one-to-many within one instance — inferable from `docs/design/conductor.md` § 3
  and the instance check of `crates/conductor/src/guard/rules.rs:1187` (inferred from code).

## Acceptance

1. Spec first: `spec/domains/config.yaml` declares `conductor.config.Level` (`host`, `instance`,
   `repository`, `unit`) and each role's level; the guard decision's reason names the sender's and
   the recipient's levels. `ess specify validate --path spec` and `task regen` pass.
2. The guard denies a lead's SendMessage to `<prefix>-maintainer` (or `<prefix>-conductor-dev`),
   with a reason naming both levels and the allowed recipients, and records the denial
   (`guard guard-decisions` shows it).
3. A director's SendMessage to its leads and to the maintainer is allowed; to a recipient two levels
   down (a worker of a lead, by its agent id, which the director did not start) it is denied.
4. Where the hook payload marks the call as a sub-agent's (a worker of the lead), the worker's
   SendMessage to anyone but its lead is denied; where the harness sends no such mark, the call is
   decided as the lead's, as today, and the story's test says which payload fields it relies on.
5. An allowed call still opens no store (AGENTS.md rule 6). Tests:
   `crates/conductor/tests/guard_rules.rs` with fixture payloads for each row of the table and each
   refusal; `crates/conductor/tests/guard_hook.rs` for the recorded denial.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model` (generated), `crates/conductor/src/guard.rs`,
`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`,
`crates/conductor/tests/guard_hook.rs`, `docs/design/conductor.md` § 7, `website/docs/reference/**`
(generated).

## Out of Scope

- The maintainer's own sends: it runs unguarded today, so its one-down rule is a profile rule until
  it runs with the guard.
- The bridge (epic `host-bridge`) and a team's sideways channel (epic `agent-tools`).
- Messages across instances, already denied (story `cross-instance-message-guard`).
