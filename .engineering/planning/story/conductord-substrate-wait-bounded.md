---
format: aep.planning-md/3
id: story:conductord-substrate-wait-bounded
kind: story
status: draft
title: conductord bounds its wait on a silent local substrate daemon
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
revision: 2
---
## Outcome

conductord's reconcile pass ends within a stated bound when the local substrate daemon takes a
connection and never answers, and records that the daemon did not answer.

## Why

`Sdk::block_on` in `crates/conductor/src/placement.rs` passes no bound for a local substrate
daemon, so conductord waits without limit on a socket that accepts and stays silent, and the one
loop that reconciles every instance stops with it. `conductor doctor` got a 5 s bound for the same
question in `story:box-crash-loop-reads-failed`; the adversary of that story (wave 50, pass 1,
finding 7) measured the wait in doctor and noted that conductord's own wait is older than that
unit. Also left from that review: a config change during a wait starts every waiting instance,
not only the changed one.

## Acceptance

Given a local substrate socket that accepts a connection and never answers, when conductord runs
a reconcile pass that asks it, then the pass returns within the bound, the reconciliation of the
asking instance is recorded as refused naming the socket and the bound, and the other instances
of the pass are reconciled.
