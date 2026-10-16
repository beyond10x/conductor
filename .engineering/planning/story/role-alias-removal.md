---
format: aep.planning-md/3
id: story:role-alias-removal
kind: story
status: draft
title: One release later, the old role names are refused and their links, task aliases and session fallbacks are gone
relations:
- decomposes: epic:role-names
- depends_on: story:role-name-aliases
- depends_on: story:role-names-in-files
- depends_on: story:worker-agents
- depends_on: story:message-levels
- depends_on: story:profile-slimming
- depends_on: story:maintainer-reads-instances
- depends_on: story:maintainer-host-config
scope:
- confidence: cited
  path: .agents
- confidence: cited
  path: .claude
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/rules.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/session.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
revision: 3
---
# Story: role-alias-removal

## Outcome

One release after the new role names ship, the old names are gone: the config refuses them, and no
old profile link, settings link, task alias or session-name fallback is left.

## Context

The contract step of the rename: `docs/design/roles-and-surfaces.md` § 7 step 1 keeps the old
names as aliases for one release. Stories `role-name-aliases` and `role-names-in-files` add the
aliases; this story removes them once the release that carried them is out and every instance's
config has been moved, which `conductor config validate` shows by printing no alias note.

## Domain relations

- Old role name → new role name, one-to-one, lives one release — inferable from
  `docs/design/roles-and-surfaces.md` § 7 step 1, as declared in `spec/domains/config.yaml` by
  story `role-name-aliases`.

## Acceptance

1. Spec first: `spec/domains/config.yaml` drops the alias table; `conductor config validate` on a
   role `controller` exits 1 naming its YAML path and the new name, such as
   `instances[0].roles[2].role: controller is no role; it is lead now`. Test:
   `crates/conductor/tests/config.rs`.
2. `.agents/conductor.md`, `conductor-dev.md`, `repo-controller.md`, their adapters, the old
   settings links and the installed old profile links are gone; `Taskfile.yml` has no
   `conductor:*` or `dev:*` alias. Tests: `crates/conductor/tests/profiles.rs`,
   `crates/conductor/tests/taskfile.rs`.
3. The guard and `session start|stop` no longer take `<prefix>-conductor` for the director. Tests:
   `crates/conductor/tests/guard_rules.rs`, `crates/conductor/tests/session.rs`.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model` (generated), `crates/conductor/src/config.rs`,
`crates/conductor/src/session.rs`, `crates/conductor/src/guard/rules.rs`, `crates/conductor/src/rules.rs`,
`.agents/`, `.claude/`, `Taskfile.yml`, the tests named above, `docs/config.md`.

## Out of Scope

- Removing the `conductor` binary's session commands (epic `agent-tools`).
- Moving the maintainer out of `instances[].roles` (decision-blocker `maintainer-host-config`).
