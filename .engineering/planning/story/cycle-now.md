---
format: aep.planning-md/3
id: story:cycle-now
kind: story
status: draft
title: An operator asks a running runtime for a cycle now
summary: A request through the instance API (and the CLI) makes the loop run its next cycle at once instead of at the next cadence tick
tags:
- from-finding
relations:
- decomposes: epic:conductor-run
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: inferred
  path: crates/conductor/src/dashboard.rs
- confidence: inferred
  path: crates/conductor/src/daemon/api.rs
revision: 1
---
## Why

Agentbox friction log F21 (2026-11-15). After a runtime start or an upgrade, open dispatches
waited for the next cycle tick. That was up to 2 h at `cadence.cycle: 2h`, and no cycle ran at
the 11:06 start. `conductor run --once` cannot run one meanwhile, because the runtime holds
`<state>/run/lock` (`crates/conductor/src/run.rs:6`, `:1428`). The operator's workaround was
`cadence.cycle: 30m`, which makes every cycle more frequent to make one sooner. Read at
v1.13.0, a cycle runs only when `cycle-last` is `cadence.cycle` old (`due`, `run.rs:1502-1508`,
called at `run.rs:409`).

## Outcome

A write route of the instance API, proxied by conductord and called by a CLI verb, marks the
next cycle due. The loop runs the cycle at its next tick, within seconds, under its own lock.

## Acceptance

- With `cadence.cycle: 2h` and a cycle that ran a minute ago, the request makes the loop run a
  cycle at its next tick and record it in run-ticks. An agent's bearer is refused as other
  operator writes are. A run test and an API test hold both.
