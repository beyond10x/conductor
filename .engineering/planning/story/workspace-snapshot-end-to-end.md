---
format: aep.planning-md/3
id: story:workspace-snapshot-end-to-end
kind: story
status: implemented
title: A workspace conductor config workspace writes is read by both snapshot collectors in one test
scope:
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T12:33:13Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T12:33:13Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T12:33:18Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
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
