---
format: aep.planning-md/3
id: story:gc-archive-profile
kind: story
status: draft
title: conductor gc lists archive build output from a checkout's profile
relations:
- informed_by: story:gc-before-pause
scope:
- confidence: cited
  path: crates/conductor/src/dashboard/waste.rs
- confidence: cited
  path: crates/conductor/tests/gc.rs
revision: 2
---
## Why

`conductor gc --dry-run` (v0.6.0, 2026-10-13 09:40Z, reported by conductor) shows the row "build
output in worktree archives … not measured": `archive_strip` in
`crates/conductor/src/dashboard/waste.rs` passes `sources.root`, the records directory, as
`worktree prune-archives --scope profile --repo`, and `worktree` refuses it with
`workspace-not-activated: no active profile contains <records>`. The records directory is in no
worktree profile; the instance's checkouts are.

## Acceptance

- `conductor gc --dry-run` passes `--repo` a repository under the config's `checkouts.root` (the
  first checkout the workspace names), not the records directory; a test in
  `crates/conductor/tests/gc.rs` with a fixture `worktree` on `PATH` records the `--repo` argument
  of the `prune-archives` call and finds the checkout's path in it.

## Notes

- Not urgent for disk: `~/.local/state/worktree/archives` read 5.2G at the report.

## Scope

`crates/conductor/src/dashboard/waste.rs`, `crates/conductor/tests/gc.rs`.
