---
format: aep.planning-md/3
id: story:goal-rank
kind: story
status: draft
title: Goals carry the operator's rank, and dispatch and the build queue follow it
relations:
- decomposes: initiative:self-improvement
- depends_on: story:build-run-priority
revision: 1
---
## Why

The operator, 2026-10-14, in conductor's session, on ranking the instance goals to order dispatch:
"yeah, goal ranking could be a nice feature, dispatch to conductor-dev". Cross-repository order
follows red signals and readiness only; when work competes for the working slots or the host disk
ledger, nothing says whose work goes first.

## Acceptance

- Spec first: a goal carries an optional rank (1 = first) with the date and the operator's words,
  set by `conductor goal rank-goal --goal-id <id> --rank <n> --words <text>`, refused unless the
  actor is the operator (class O); `goals` and `conductor status` show it; tests hold the refusal,
  the record and the views.
- Ordering: a red-CI target still comes first; among the rest, the snapshot's compare step and
  `conductor status` list repositories by the best rank of the goals they serve, and a disk request
  carries its repository's best rank so the host queue grants a higher-ranked request first after
  red-CI requests (with `story:build-run-priority`); tests hold both orders.
- `.agents/conductor.md`'s Dispatch step and the charter template say dispatch follows the goal
  rank after red signals, and that conductor proposes ranks to the operator with a recommendation;
  profile tests hold the sentences.

## Scope

`spec/domains/direction.yaml`, `spec/domains/host.yaml`, `crates/conductor-model`,
`crates/conductor/src/goal.rs`, `crates/conductor/src/status.rs`, `crates/conductor/src/host.rs`,
`crates/conductor/src/build.rs`, `crates/conductor/tests/`, `.agents/conductor.md`.
