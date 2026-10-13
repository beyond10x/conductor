---
format: aep.planning-md/3
id: story:snapshot-duration
kind: story
status: draft
title: A snapshot records each collector's duration and completes in under five minutes
relations:
- informed_by: story:snapshot-member-failures
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/snapshot.rs
- confidence: cited
  path: docs/analysis/snapshot-duration.md
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On this instance `conductor snapshot start-snapshot` took 1949 s (snapshot 01a134b7, 2026-10-13
10:03Z start, v0.6.0), and two earlier snapshots failed on the caller's timeouts (1500 s and
900 s); a cycle is two hours and the snapshot is its first step (conductor's report,
2026-10-13). Nothing records which collector costs what, so the slow one cannot be named.

## Acceptance

- Spec first: each collector's run in a snapshot records its name, its start and its duration in
  milliseconds (`spec/domains/observation.yaml`), shown by `conductor snapshot snapshots` (or a
  `snapshot collectors` view); a test with fake collectors of known delay reads the durations.
- The snapshot of this instance completes in under 300 s, with the per-collector durations of that
  run recorded in `docs/analysis/snapshot-duration.md` before and after the change (the change
  named from the before-durations: parallel collectors, cached reads, or a narrower read).
- `conductor snapshot start-snapshot --budget <seconds>` fails a collector that passes the budget
  as a recorded member failure rather than the whole snapshot; a test with a fake slow collector
  holds it.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `website/docs/reference/cli.md`,
`crates/conductor/src/snapshot.rs`, `crates/conductor/src/collect/mod.rs`,
`crates/conductor/tests/snapshot.rs`, `docs/analysis/snapshot-duration.md` (new).
