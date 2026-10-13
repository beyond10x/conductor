---
format: aep.planning-md/3
id: story:watch-goal-health
kind: story
status: draft
title: The watch reports a goal whose run or timer has stalled
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/goal.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/direction.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor learned that a goal's seven-day measurement run had stopped two days
earlier: an upstream change had disabled its timer, and the run had produced no data for four and a
half days. No watch line, snapshot or cycle saw it; it came to light in a survey the operator asked
for, and the goal's read moved five days. The fix taken was a hand-written per-cycle
`systemctl --user --failed | grep …` that each new conductor session must remember.

## Acceptance

- Spec first: a goal may carry probes, declared in `spec/domains/direction.yaml`: a user unit or
  timer that must be active, or a file whose modification time must be newer than a duration; `goal
  propose-goal` and a new `goal set-probe` take them; `ess specify validate --path spec` exits 0.
- `conductor watch run` checks each probe of each confirmed goal every `cadence.watch` and prints
  `goal stalled: <goal> <probe> <age>` once when it fails, and `goal healthy again: <goal> <probe>`
  once when it recovers; a test in `crates/conductor/tests/watch.rs` with a fake unit state and a
  file aged past its duration reads both lines once each.
- The snapshot's board lists a stalled goal probe under Deviations; a test in
  `crates/conductor/tests/status.rs` holds it.
- Design § 8's probe table has the row.

## Notes

- After `story:watch-pr-checks` on `crates/conductor/src/watch.rs` and its test file.
- Shares `crates/conductor/src/status.rs` with `story:plan-hygiene` and `story:status-command`.

## Scope

`spec/domains/direction.yaml`, `crates/conductor-model`, `crates/conductor/src/goal.rs`,
`crates/conductor/src/watch.rs`, `crates/conductor/src/status.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/watch.rs`, `crates/conductor/tests/status.rs`, `docs/design/conductor.md`,
`website/docs/reference/cli.md`.
