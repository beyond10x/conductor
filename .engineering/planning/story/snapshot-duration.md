---
format: aep.planning-md/3
id: story:snapshot-duration
kind: story
status: draft
title: A snapshot records each collector's duration and completes in under five minutes
relations:
- informed_by: story:snapshot-member-failures
- depends_on: story:setup-command
- depends_on: story:fetch-without-prompts
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/store/observations.rs
- confidence: cited
  path: crates/conductor/src/store/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/snapshot.rs
- confidence: cited
  path: docs/analysis/snapshot-duration.md
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 7
---
## Why

On this instance `conductor snapshot start-snapshot` took 1949 s (2026-10-13, 10:03Z start,
v0.6.0), and two earlier snapshots failed on the caller's timeouts (1500 s and 900 s); a cycle is
two hours and the snapshot is its first step (conductor's report, 2026-10-13). Nothing records
which collector costs what, so the slow one cannot be named.

## Acceptance

- Spec first: a collector run of a snapshot, `CollectorRunObservation` (snapshot id, collector
  name, start, duration in milliseconds, state `completed`, `failed` or `over budget`), is declared
  in `spec/domains/observation.yaml`, with a view `CollectorRuns` placed in the `snapshot` group of
  `spec/components.yaml`, so `conductor snapshot collector-runs --snapshot-id <id>` lists them; a
  test with fake collectors of known delay reads the durations in that view.
- `conductor snapshot start-snapshot --budget <seconds>` stops a collector that passes the budget,
  records its run as `over budget`, and completes the snapshot with the other collectors' rows; a
  test with a fake slow collector reads the `over budget` run and a completed snapshot.
- The snapshot of this instance completes in under 300 s, with the `snapshot collector-runs` of that
  run recorded in `docs/analysis/snapshot-duration.md` before and after the change (the change
  named from the before-durations: parallel collectors, cached reads, or a narrower read).

## Notes

- After `story:setup-command`: both regenerate `crates/conductor-model` and
  `website/docs/reference/cli.md` and edit `crates/conductor/src/cli.rs`.
- After `story:snapshot-member-failures` and `story:fetch-without-prompts`. With the first it shares
  `spec/components.yaml` (both add to the `snapshot` group's list), `spec/domains/observation.yaml`,
  `store/observations.rs` (`ENTITIES` grows by one in each), `store/snapshot.rs` (one storage port
  each), `snapshot.rs`, `cli.rs` and `reference/cli.md`; with the second, `collect/mod.rs`.
- `story:onboarding-run` comes after it on the model, `cli.rs` and `reference/cli.md`.
- `story:dashboard-health` serves these rows.

## Scope

`spec/domains/observation.yaml`, `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`, `website/docs/reference/cli.md`,
`crates/conductor/src/snapshot.rs`, `crates/conductor/src/store/observations.rs`,
`crates/conductor/src/store/snapshot.rs`, `crates/conductor/src/collect/mod.rs`,
`crates/conductor/tests/snapshot.rs`, `docs/analysis/snapshot-duration.md` (new).
