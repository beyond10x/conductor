---
format: aep.planning-md/3
id: story:collector-failure-stays-its-own
kind: story
status: draft
title: A collector that still fails fails only its own rows
summary: One failing collector fails the whole snapshot today; the snapshot should keep every other collector rows
refs:
- provider: github
  reference: beyond10x/conductor#241
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/src/repository.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/snapshot.rs
revision: 2
---
## Why

Issue #241 asked that a collector that still fails after its retries fail only its own rows.
`story:forge-rate-limit-retried`, which supersedes `story:collector-retries-rate-limit`, retries
rate-limited forge reads but keeps a failing collector failing the whole snapshot; this story takes
the requirement it left out.

On main:

- In the repositories collector, the first error of any one repository's read is the collector's
  error, and no further repository is started once one has failed
  (`crates/conductor/src/collect/repositories.rs:1904-1946`, `each`).
- The snapshot driver runs the collectors in order and, at the first that fails, records its reason
  and stops: no later collector runs (`crates/conductor/src/snapshot.rs:239-290`, the `break` at
  `:288`). The snapshot then ends `Failed` (`crates/conductor/src/snapshot.rs:302-311`).
- `repository activity` and `metric compute` read only the newest `Complete` snapshot
  (`crates/conductor/src/repository.rs:163-171`, `newest_complete`), so the rows the collectors
  before the failing one recorded are not read either.

## Outcome

- A collector that still fails fails only its own rows: the snapshot runs every other collector and
  keeps their rows, and the failing collector's run is recorded `Failed` with its reason.
- The views read those rows, and say which collector of that snapshot failed.

## Acceptance (tests that fail on the current code)

- `tests/snapshot.rs` `a_failing_collector_leaves_the_later_collectors_rows_recorded`.
- `tests/snapshot.rs` `the_views_read_the_other_collectors_rows_of_a_snapshot_one_collector_failed`.

## Boundaries

- The retry of a rate-limited forge read is `story:forge-rate-limit-retried`.
- Whether one repository's failed read keeps the other repositories' rows of the same collector is
  decided in this story's design, not assumed.
- `task docs-generate` rewrites `website/docs/status.md` and `website/data/status.json`; those are
  regenerated after merge, not merged by hand.
