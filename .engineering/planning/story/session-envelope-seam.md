---
format: aep.planning-md/3
id: story:session-envelope-seam
kind: story
status: draft
title: A session starts only in the envelope its role requests
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-envelope-model
- depends_on: story:session-box-spike
- depends_on: story:spawn
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/confine.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 4
---
## Why

The ADR's provider seam (architecture-decision-record:external-confinement; metaharness `crates/metaharness/src/process.rs:99-193`): a request,
a provider that starts the harness in it, a measurement at the boundary, and a refusal when they do
not match.

## Acceptance

- A `confine` module with a `Provider` trait: from a role's request it builds the start command and
  reads back a measurement; a `none` provider builds the plain command and measures nothing.
- Spec first, in `crates/conductor/cli.yaml`: `session start --role <role>` starts conductor's or
  conductor-dev's session through the instance's provider.
- Tests through `session start` with a fake provider (`crates/conductor/tests/confine.rs`), one per
  case:
  - `strict`, provider unavailable: exit non-zero, the reason printed, no session started;
  - `strict`, measurement short of the request (a limit or a writable path missing): the session is
    stopped, exit non-zero;
  - `report`, provider unavailable: the session starts and `unboxed` with the reason is recorded;
  - a full measurement is written to the instance's state, and `conductor snapshot sessions` shows
    it.
- `controller start-controller` (`story:spawn`) starts a controller through the same function: a
  test with the fake provider sees one call per start, with the controller role's request.
- `conductor doctor` lists each provider with `available` or the reason it is not; each provider
  supplies its own probe. Test with two fake providers, one of each (`crates/conductor/tests/doctor.rs`).
- `Taskfile.yml` `conductor:start` and `dev:start`, and the controller start and resume commands in
  `.agents/conductor.md`, call these commands instead of `claude --bg`. A test reads them
  (`crates/conductor/tests/taskfile.rs`). The `.agents/conductor.md` diff is merged only after an
  `approval-record` from the operator `decides` this story.

## Scope

`crates/conductor/cli.yaml`, `crates/conductor/src/confine/mod.rs` (new),
`crates/conductor/src/spawn.rs`, `crates/conductor/src/doctor.rs`, `crates/conductor/src/lib.rs`,
`Taskfile.yml`, `.agents/conductor.md`, `crates/conductor/tests/confine.rs` (new),
`crates/conductor/tests/doctor.rs`, `crates/conductor/tests/taskfile.rs`. After
`session-envelope-model` (its data), `session-box-spike` (how a session is kept in a box),
`story:spawn` (`spawn.rs`), and `instance-session-names` (the same start lines).
