---
format: aep.planning-md/3
id: story:gc-before-pause
kind: story
status: draft
title: conductor gc strips archive build output and names other holders before work pauses for disk
relations:
- depends_on: story:conductor-gc
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/dashboard/waste.rs
- confidence: cited
  path: crates/conductor/src/gc.rs
- confidence: cited
  path: crates/conductor/tests/gc.rs
revision: 2
---
## Why

Builds paused under the instance's disk floors for hours between 2026-10-10 and 2026-10-13. What
freed the space came from outside conductor: build output kept in worktree archives (later stripped
by `worktree prune-archives --strip-build-output`), a shared Go build cache regrowing to 25G, and
Flutter build output. `conductor gc` (wave 10) reclaims idle trees' caches only.

## Acceptance

- Before conductor pauses work for disk, `conductor gc` also strips build output from archives the
  worktree tool marks strippable and reports every other large holder it can measure (shared build
  caches, archives it may not touch) with its size and owner in one table; a test over a fixture
  archive root and a fixture cache holds both.

## Scope

`crates/conductor/src/gc.rs`, `crates/conductor/src/dashboard/waste.rs`,
`crates/conductor/tests/gc.rs`, `.agents/conductor.md`.
