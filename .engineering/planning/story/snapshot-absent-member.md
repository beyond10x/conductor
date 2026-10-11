---
format: aep.planning-md/3
id: story:snapshot-absent-member
kind: story
status: draft
title: A workspace member whose checkout is absent is skipped, and the snapshot completes
scope:
- confidence: cited
  path: crates/conductor/src/collect/blockers.rs
- confidence: cited
  path: crates/conductor/src/collect/specifications.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
revision: 2
---
## Why

A workspace member whose checkout is missing fails every snapshot: `blockers::read` runs `git
fetch` in it (`crates/conductor/src/collect/blockers.rs:309-310`, `:378`), the error propagates,
and the driver fails the whole snapshot at the first collector error
(`crates/conductor/src/snapshot.rs:197-210`). `specifications::read` calls `blockers::read` first
(`collect/specifications.rs:202`) and fails the same way. That held the instance blind for about two
days (2026-10-08 to 2026-10-10). `aep plan workspace members --format json` already reports such a
member as `"state": "absent"`, with exit 0.

## Acceptance

- The blockers and specifications collectors skip a member aep reports `absent`, and the snapshot
  completes with the other members' rows. Test: three members, one absent; the snapshot completes
  with the two members' blockers and specifications.
- Each skipped member is named on one line of standard error (`member <name>: checkout absent,
  skipped`). Test.
- A member that is present but whose `git fetch` fails still fails the snapshot, as today
  (a typed per-member failure is `story:snapshot-member-failures`).

## Scope

`crates/conductor/src/collect/blockers.rs`, `crates/conductor/src/collect/specifications.rs`,
`crates/conductor/tests/collect_stores.rs`.
