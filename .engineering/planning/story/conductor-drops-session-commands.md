---
format: aep.planning-md/3
id: story:conductor-drops-session-commands
kind: story
status: draft
title: The conductor binary loses the session commands no session calls any more
relations:
- decomposes: epic:agent-tools
- depends_on: story:agent-run-grants
- depends_on: story:tool-director-verbs
scope:
- confidence: cited
  path: .agents
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/cli_tree.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: website/docs/reference
revision: 4
---
# Story: conductor-drops-session-commands

## Outcome

The `conductor` binary no longer compiles in any session command that `tool` replaces and that no session called within a measured window. This includes the director's commands. What remains is the operator's and the bridge's, so no session of an instance holds the admin binary.

## Context

docs/design/roles-and-surfaces.md § 6.1 ("the split is by binary, not by subcommand") and § 7 step 6 ("once no session calls them"). § 1 point 4: every session that can run `conductor` can run all of it. decision-blocker:director-tool-verbs (cleared 2026-10-16) gave the director its own verb group (story:tool-director-verbs), so its commands go too.

## Domain relations

- Command → Component, many-to-one: each command is accepted by exactly one component (`conductor-cli`, `conductor-broker` or `tool`), in `spec/components.yaml` under each component's `accepts.commands` (ess/1). This story moves entries between those lists.

## Acceptance

`conductor --help` lists no command that `tool` replaces, the director's included, and the CLI tree test fails if one comes back. The removal list matches a `conductor resource tool-calls` reading that shows zero session calls for each removed command.

1. **Evidence first.** `conductor resource tool-calls --since <t>` over at least 7 days gives each `conductor` subcommand's call count from sessions, director and leads included. A command is removed only when it has zero calls and its `tool` verb exists. The reading is recorded as evidence on this story.
2. **Spec first.** Each removed command leaves `conductor-cli`'s `accepts` in `spec/components.yaml` and is accepted by `conductor-broker` instead, and `crates/conductor/cli.yaml` drops its binding. This covers the session-side decision, dispatch, message, goal, controller and resource-request commands. Validate and regenerate.
3. `crates/conductor/tests/cli_tree.rs` asserts that the removed commands are absent and the operator commands are present (`init`, `setup`, `config`, `trust`, `upgrade`, `doctor`, `dashboard`, `gc`, `host`, `status`).
4. The guard rules that only policed removed commands (the `conductor` write rule and the `build run` prefix rule) are deleted with their tests. `tests/guard_latency.rs` shows the allow path did not grow.
5. Profiles and `rules.md` guidance that name a removed command name its `tool` verb instead. `task docs-generate` refreshes the reference pages.

## Scope

- `spec/components.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`, the handlers of removed commands under `crates/conductor/src/`
- `crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/cli_tree.rs`, `crates/conductor/tests/guard_rules.rs`
- `.agents/`, `website/docs/reference/**` (generated)

## Out of Scope

- Operator and bridge commands, which stay in `conductor`.
- Binaries pinned per instance (epic:multi-instance-host, I13).

## Ambiguities

None.
