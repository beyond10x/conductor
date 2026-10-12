---
format: aep.planning-md/3
id: story:conductor-gc
kind: story
status: implemented
title: conductor gc reclaims what the disk panel calls reclaimable
relations:
- decomposes: epic:disk-and-gate
- depends_on: story:portable-tree-sizes
- supersedes: story:watch-tree-sizes
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/disk.rs
- confidence: cited
  path: crates/conductor/src/gc.rs
- confidence: cited
  path: crates/conductor/tests/gc.rs
- confidence: cited
  path: spec/domains
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T19:49:58Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T19:49:58Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T20:20:25Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator chose cleanup rules as the disk control (`approval-record:project-quota-c`). The
dashboard's disk-waste panel already measures what can be reclaimed
(`crates/conductor/src/dashboard/waste.rs:4-11`), but nothing reclaims it; the instance runs a
cleanup-only session by hand for it. `epic:operating-loop-hardening` deferred the command until a
specification exists.

## Acceptance

- `conductor gc --dry-run` lists exactly the rows the disk panel calls reclaimable, with their
  bytes, and `conductor gc` removes those and no tree a live session works in, printing the bytes
  freed; a test over a fixture trees root with one live and one idle tree holds both.

## Notes

Spec first: the command and its report in `spec/`. The guard declares it writing (`cli::LEAVES`).
After `story:portable-tree-sizes` (both change `crates/conductor/src/disk.rs`).

## Scope

`spec/domains/`, `crates/conductor-model/`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/disk.rs`, `crates/conductor/src/gc.rs` (new), `crates/conductor/tests/gc.rs`
(new).
