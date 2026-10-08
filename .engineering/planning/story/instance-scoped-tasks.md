---
format: aep.planning-md/3
id: story:instance-scoped-tasks
kind: story
status: draft
title: Every task acts on the active instance's sessions only
relations:
- decomposes: epic:multi-instance-host
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 7
---
## Why

`task conductor:restart` stops any background session named `conductor`, another instance's too
(`specification:multi-instance-host`, finding 2); the dashboard port is one for all (finding 5).

## Acceptance

- Every task resolves sessions by the active instance's full names (story
  `instance-session-names`) and refuses when a name resolves to more than one session.
- `instances[].dashboard_port` (spec first; default 7313 for the first instance; refused when two
  instances share one); the dashboard tasks use it, and their unit or pid file is per instance.
- Tests: two instances in one config; `conductor:restart` of one leaves the other's fake session
  running; the second dashboard binds its own port.

## Scope

`Taskfile.yml`, `spec/domains/config.yaml`, `crates/conductor/src/config.rs`,
`crates/conductor/tests/taskfile.rs`, `crates/conductor-model/`. After `instance-session-names` (shared `config.yaml`,
`config.rs`).
