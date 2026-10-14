---
format: aep.planning-md/3
id: story:marked-by-session
kind: story
status: draft
title: A repository mark names the session that made it
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

In an outside user's trial `conductor repository mark-repository --marked-by` took only `Conductor`
or `Operator`, so a mark made by a controller or another session could not say who made it.

## Acceptance

- Spec first: a repository mark records an optional `session` (the marking session's name) beside
  `marked_by`, and `MarkedBy` gains `Controller` (`spec/domains/direction.yaml`);
  `ess specify validate --path spec` exits 0.
- `conductor repository mark-repository --marked-by controller --session <name>` records both and
  `repository repository-marks` shows them; a test in `crates/conductor/tests/repository_marks.rs`
  holds it.

## Scope

`spec/domains/direction.yaml`, `crates/conductor-model`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/repository.rs`, `crates/conductor/tests/repository_marks.rs`.
