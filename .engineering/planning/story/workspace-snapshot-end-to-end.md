---
format: aep.planning-md/3
id: story:workspace-snapshot-end-to-end
kind: story
status: draft
title: A workspace conductor config workspace writes is read by both snapshot collectors in one test
scope:
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
revision: 2
---
## Why

`conductor config workspace` (wave 04) needed three fixes after merge, each found by a failed live
snapshot (conductor's NEED N-20261012-01, -02, -03): a records member written as `..`, a checkout
with no `origin` remote, and a checkout whose directory differs from its forge name. Each fix was
tested alone; no test ran the blockers and specifications collectors over a file the command wrote.

## Acceptance

- One test writes a workspace with `conductor config workspace` over a fixture instance holding a
  records repository whose directory differs from its origin's name, a local-only checkout and a
  dotted checkout name, then runs the blockers and specifications collectors over it through
  `snapshot::take`; the snapshot completes, with one specification row per listed repository.
  It fails on each of the three earlier defects when that fix is reverted (logged).

## Scope

`crates/conductor/tests/collect_stores.rs` or a new `crates/conductor/tests/workspace_snapshot.rs`.
