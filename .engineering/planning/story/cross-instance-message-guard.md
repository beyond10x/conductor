---
format: aep.planning-md/3
id: story:cross-instance-message-guard
kind: story
status: archived
title: The guard keeps messages inside one instance
relations:
- decomposes: epic:multi-instance-host
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-10-08T22:02:39Z", actor: "human:timo", revision: 4}
---
## Why

The guard allows a controller's SendMessage to the name `conductor`, whichever instance owns it
(`spec:multi-instance-host`, finding 3).

## Acceptance

- A controller's SendMessage is allowed only to `<its prefix>-conductor` or its own sub-agents;
  conductor's only to sessions with its own prefix; a recipient with another instance's prefix is
  denied naming both instances.
- Tests in `crates/conductor/tests/guard_rules.rs` with two instances in one config.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`.
