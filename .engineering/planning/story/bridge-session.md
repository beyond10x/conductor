---
format: aep.planning-md/3
id: story:bridge-session
kind: story
status: draft
title: One bridge session per host reads every instance and writes none
relations:
- decomposes: epic:host-bridge
- depends_on: story:host-instances-list
- depends_on: story:maintainer-host-config
revision: 2
---
# Story: bridge-session

## Outcome

One bridge session runs per host under its own profile; it reads every instance and relays to its director, and the guard refuses it every write into an instance.

## Context

docs/design/roles-and-surfaces.md § 2 (the bridge, one per host), § 4 ("What the bridge never does: it never plans for an instance, decides for one, or carries one instance's data into another"), § 5b (the bridge is per host, shared by design) and § 7 step 1 ("add the bridge session as a profile over the existing CLI"). Epic acceptance 5 asks for profile rules and a guard test.

The bridge is a built-in host role: it starts in the host directory, outside every instance's trees. Configuring host roles (the maintainer's move out of `instances[].roles`) belongs to epic:role-names; this story adds no config key.

## Domain relations

- Host → Bridge session, one-to-one; the host owns it; the bridge may be absent, and every instance outlives it — inferable (docs/design/roles-and-surfaces.md § 2 and § 5b, operator-approved 2026-10-16; no ess/1 document declares it: this story's spec step adds the bridge to `spec/domains/session.yaml`).
- Bridge → Instance, one-to-many through the registry (story:host-instances-list), read and relay only, owning none — inferable (same sections; declared by the same spec step).

## Acceptance

1. Spec first: `spec/domains/session.yaml` names the `bridge` role and its session name `bridge`, unique on the host; `ess specify validate --path spec` exits 0 and the model crate is regenerated.
2. `conductor session start bridge` starts it once; a second start is refused while it runs; `crates/conductor/tests/session.rs` holds both.
3. `.agents/bridge.md` and `.claude/agents/bridge.md` ship. The profile says: name the route on the first line as `→ <instance>/<repository>: <one line>`; ask one question with options when a request is ambiguous; split a request that needs two instances into one per instance and join the answers; never plan, decide or carry one instance's data into another. `crates/conductor/tests/profiles.rs` holds each rule's line.
4. The guard refuses the bridge every write into an instance (`decision`, `dispatch`, `goal` and `snapshot` writes; edits under any records or trees root), and a message to one instance's director that quotes or attaches another instance's records. `crates/conductor/tests/guard_rules.rs` holds each case with instances `alpha` and `beta`.
5. The profile names no organization; `crates/conductor/tests/no_organization.rs` covers it.

## Scope

`spec/domains/session.yaml`, `crates/conductor-model`, `crates/conductor/src/session.rs`, `crates/conductor/src/guard/rules.rs`, `.agents/bridge.md`, `.claude/agents/bridge.md`, `crates/conductor/tests/session.rs`, `crates/conductor/tests/guard_rules.rs`, `crates/conductor/tests/profiles.rs`, `crates/conductor/tests/no_organization.rs`, `docs/design/conductor.md` (§§ 2–3).

## Out of Scope

Reaching the bridge (story:ask-bridge-route). Host role configuration and the role renames (epic:role-names). The bridge starting and stopping instances and upgrading one at a time (§ 5b, I13).

## Ambiguities

None.
