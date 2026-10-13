---
format: aep.planning-md/3
id: story:gc-before-pause
kind: story
status: draft
title: conductor gc strips archive build output and names other holders before work pauses for disk
relations:
- depends_on: story:conductor-gc
- decomposes: epic:conductor-autonomy
- depends_on: story:operator-items-age
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/dashboard/waste.rs
- confidence: cited
  path: crates/conductor/src/gc.rs
- confidence: cited
  path: crates/conductor/tests/gc.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 5
---
## Why

Builds paused under the instance's disk floors for hours between 2026-10-10 and 2026-10-13. What
freed the space came from outside conductor: build output kept in worktree archives (later stripped
by `worktree prune-archives --strip-build-output`), a shared Go build cache regrowing to 25G, and
Flutter build output. `conductor gc` (wave 10) reclaims idle trees' caches only.

## Acceptance

- `conductor gc` lists, beside idle trees, the strip of build output from worktree archives
  (`worktree prune-archives --strip-build-output`), and `conductor gc --apply` runs it; a test with
  a fixture `worktree` on `PATH` reads the strip row in the dry run's output and records that
  `--apply` called it and the dry run did not.
- `conductor gc` prints one table of the holders the waste panel (`dashboard/waste.rs`) measures
  but gc does not reclaim, now including the Go build cache (`~/.cache/go-build`), each with its
  path and size; a test with a fixture `HOME` holding a `.cache/go-build` directory reads its row.
- `.agents/conductor.md` tells conductor to run `conductor gc --apply` before it pauses work for
  disk; a test in `crates/conductor/tests/profiles.rs` holds that sentence.

## Notes

- Flutter build output is left out of this story; it becomes a follow-up if it recurs.

- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with the other profile
  stories of `epic:conductor-autonomy`; it depends on `story:operator-items-age` so it lands after it
  on those files.

## Scope

`crates/conductor/src/gc.rs`, `crates/conductor/src/dashboard/waste.rs`,
`crates/conductor/tests/gc.rs`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
