---
format: aep.planning-md/3
id: story:first-cycle-not-blocked-by-archiving
kind: story
status: active
title: The first cycle starts controllers without waiting for worktree archiving
summary: archiving runs off the cycle critical path with an n-of-N progress line
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T09:28:05Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T09:28:05Z", actor: "human:timo", revision: 4}
---
## Why

Observed 2026-11-07 on a new instance whose checkouts held about 225 archived managed trees: the
first cycle of `conductor run` spent its time on worktree archiving before its snapshot completed.
Its journal printed one `archive 0 bytes <repository>/<tree> … nothing-to-strip|tree-still-present`
line about every 12 s from 04:58 to 05:05Z (inferred: up to about 45 min for all of them); after
7.7 min `conductor status` still said "snapshot: none is complete"; the unit's memory peak was
6.1 GB. Controllers start only in the cycle, after the snapshot (`cadence.cycle`,
`docs/design/conductor-run.md` lines 51-53), so none started; they were started by hand with
`conductor spawn`.

Observed again 2026-11-07 on a second instance: its loop, started 04:05Z, logged `archive …
nothing-to-strip` lines until 06:49:45Z, then waited on a child `worktree gc --repo <checkout> --id
<tree> --apply`; `conductor session run-ticks` holds no tick after 06:16:17Z, so that day's daily
tick did not run.


Cause: gc started by a `disk low` watch line ran inside the watch tick (`disk_low`,
`crates/conductor/src/run/handlers.rs`), and the loop is one thread, so no cycle and no daily tick
ran until gc, its archiving and its `worktree gc --apply` ended.

## Proposal

- gc started by a `disk low` line runs on its own thread beside the loop's ticks, at most one at a
  time; its `cleanup` line is written by the first watch tick after it ended.
- Archiving prints `archive n of N` on standard error, one line per archive.
- The fixed cost per archived tree and the memory peak of a pass are story:archive-pass-cost.

## Acceptance

- While the gc a `disk low` line started still runs, a cycle tick and a daily tick run, and a
  second `disk low` starts no second gc; once gc ended, one `cleanup` line is written with its
  outcome.
- The journal shows `archive n of N` lines on standard error.
