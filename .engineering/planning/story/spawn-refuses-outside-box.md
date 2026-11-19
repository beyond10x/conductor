---
format: aep.planning-md/3
id: story:spawn-refuses-outside-box
kind: story
status: active
title: conductor spawn refuses outside a boxed instance's box, naming instance cycle
summary: A controller of a boxed instance never starts on the host
refs:
- provider: github
  reference: beyond10x/conductor#216
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:33:49Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:33:49Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #216. `conductor spawn` run on the host for an instance at `placement: systemd` started the
controller as a host `claude --bg` session, outside the instance's box. Spawn checks the
placement nowhere: it chooses the instance's `confinement` provider
(`crates/conductor/src/spawn.rs:193`), which on the host is `none`, and launches. The operator's
rule is that a boxed instance's sessions stay in its box. The runtime in the box starts its
controllers there, and `conductor instance cycle` asks it for a cycle now; no route starts a
controller in the box from the host.

## Outcome

- `conductor spawn` for an instance whose placement opens a substrate box, run outside that box,
  refuses `OutsideBox` (a new `SpawnRefusal`, and the same `WorkStartRefusal`) before anything is
  recorded or launched, naming `conductor instance cycle` and `controller controllers`.
- Inside the box (the runtime, its loop and its work call) spawn starts as before.

## Acceptance (fail on main)

- `tests/box_session.rs` `on_the_host_a_boxed_instance_starts_no_session_outside_its_box`: on the
  host, spawn for a boxed instance exits 1 with `spawn: refused: OutsideBox` and
  `conductor instance cycle`; the harness is not called and nothing is recorded.
- The existing box tests that spawn in the box still pass.

## Boundaries

- No route from the host into the box's runtime for a start; the operator asks for a cycle.
- `session start --role director` is issue #220's story.

## Note (implementation)

The refusal applies to an instance whose file writes its `placement` (`conductor.config/2`). A
`/1` file and the built-in instance name their placement after the confinement's provider
(`systemd` by default), whose own sessions start where the command runs; refusing those would
stop every legacy start (`tests/confine.rs`, `tests/spawn.rs`). `story:director-boxed-start` uses
the same check (`crate::spawn::outside_box`).
