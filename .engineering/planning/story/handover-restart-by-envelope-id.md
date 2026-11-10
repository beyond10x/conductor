---
format: aep.planning-md/3
id: story:handover-restart-by-envelope-id
kind: story
status: draft
title: The follow-up restart and the idle check read the enveloped session id, not the first session of that name
scope:
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 4
---
## Why

Found by the implementor of story:context-watch-by-envelope in wave 49 (2026-11-10), by reading;
not reproduced. The `context:` line and the hand-over delivery are gated by the session id the
controller's envelope records (`measured_is_enveloped`, crates/conductor/src/run/handlers.rs:216).
The follow-up restart is not: `handovers()` at crates/conductor/src/run/handlers.rs:310-315
restarts on "context over 1.25x" through `Watch::context_of(session)`
(crates/conductor/src/watch.rs:849), which takes the first live session with that name, and
`Watch::idle` picks a session the same way. Two live sessions sharing the controller's session
name would let the other one's context trigger the restart. It fires only after a delivery that
the envelope gated.

The run test of the gate (crates/conductor/tests/run.rs:998) uses 314k tokens against a 200k
threshold, so it holds the restart branch only; no test holds a foreign line between 1.0x and
1.25x of the threshold, the delivery branch.

## Acceptance

- The follow-up restart and the idle check read the context of the session id the controller's
  envelope records, never of the first live session with its name; a test with two live
  sessions of one name, the foreign one over 1.25x, sees no restart.
- A test holds that a `context:` line of a session the envelope does not record, between 1.0x
  and 1.25x of the threshold, delivers no hand-over.
