---
format: aep.planning-md/3
id: story:usage-conserve
kind: story
status: draft
title: Conductor conserves before the pool runs out and sessions stop at safe points
relations:
- decomposes: epic:usage-continuity
- depends_on: story:usage-probe
revision: 1
---
## Why

When the pool will run out before its next reset, sessions keep fanning out and stop mid-unit,
with uncommitted work and no hand-over (epic:usage-continuity, 2026-10-14).

## Acceptance

- When the forecast says the pool runs out before the earliest reset, the watch prints `usage:
  conserve until <reset>` once; `.agents/conductor.md` says that in conserve conductor starts no new
  wave or dispatch, lowers sub-agent width to one, and tells each controller to reach a safe point
  (local commit, hand-over written) and wait; `.agents/repo-controller.md` says how a controller
  answers it. Profile tests hold both; a watch test holds the line.
- Conserve ends on its own when the forecast clears (a reset, or a rotation to an account with
  headroom), printed once.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`, `.agents/conductor.md`,
`.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
