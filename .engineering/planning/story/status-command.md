---
format: aep.planning-md/3
id: story:status-command
kind: story
status: draft
title: conductor status prints the board, live sessions, open dispatches and waits
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor tried `conductor status` in four sessions, the last after
`story:status-board` shipped as `snapshot publish-board` and `task status`, and got "unrecognized
subcommand". The operator asked for status twice, and each time conductor assembled the answer from
five to ten commands. The board needs a fresh snapshot and holds neither live sessions nor open
dispatches nor waits.

## Acceptance

- Spec first: a `Status` view (newest complete snapshot id and age, its deviations, live sessions,
  open dispatches, registered waits, pending requests) is declared in
  `spec/domains/observation.yaml`; `ess specify validate --path spec` exits 0.
- `conductor status` prints those six parts from the store without starting a snapshot and returns
  within 2 s on a fixture store; with no complete snapshot it says so and still prints the other
  parts. Tests in `crates/conductor/tests/status.rs`.
- The guard allows `conductor status` to a controller (a view); a case in
  `crates/conductor/tests/guard_rules.rs` holds it.
- `.agents/conductor.md` answers an operator's status question with `conductor status`;
  `crates/conductor/tests/profiles.rs` holds the sentence.

## Notes

- After `story:records-from-commands` and `story:watch-waits`: without them, the dispatch and wait
  parts are empty.
- Shares `crates/conductor/src/status.rs` and its tests with `story:plan-hygiene` and
  `story:watch-goal-health`.
- `conductor --version` is `story:version-flag` (implemented).

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/status.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`, `crates/conductor/src/guard/rules.rs`,
`crates/conductor/tests/status.rs`, `crates/conductor/tests/guard_rules.rs`,
`.agents/conductor.md`, `crates/conductor/tests/profiles.rs`, `website/docs/reference/cli.md`.
