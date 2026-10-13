---
format: aep.planning-md/3
id: story:gc-archive-profile
kind: story
status: implemented
title: conductor gc lists archive build output from a checkout's profile
relations:
- informed_by: story:gc-before-pause
scope:
- confidence: cited
  path: crates/conductor/src/dashboard/waste.rs
- confidence: cited
  path: crates/conductor/tests/gc.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:00Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:01Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`conductor gc --dry-run` (v0.6.0, 2026-10-13 09:40Z, reported by conductor) shows the row "build
output in worktree archives … not measured": `archive_strip` in
`crates/conductor/src/dashboard/waste.rs` passes `sources.root`, the records directory, as
`worktree prune-archives --scope profile --repo`, and `worktree` refuses it with
`workspace-not-activated: no active profile contains <records>`. The apply path passes the same
`--repo` (`waste.rs:341`).

## Acceptance

- `conductor gc --dry-run` and `conductor gc` pass `--repo` a git checkout under the config's
  `checkouts.root` (the first in name order), not the records directory; a test in
  `crates/conductor/tests/gc.rs` with a fixture `worktree` on `PATH` records the `--repo` argument
  of both `prune-archives` calls and finds the checkout's path in each.

## Notes

- Not urgent for disk: `~/.local/state/worktree/archives` read 5.2G at the report.

## Scope

`crates/conductor/src/dashboard/waste.rs`, `crates/conductor/tests/gc.rs`.
