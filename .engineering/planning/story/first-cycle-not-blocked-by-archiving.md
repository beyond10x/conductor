---
format: aep.planning-md/3
id: story:first-cycle-not-blocked-by-archiving
kind: story
status: draft
title: The first cycle starts controllers without waiting for worktree archiving
summary: archiving runs off the cycle critical path with an n-of-N progress line
revision: 1
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

## Proposal

- Archiving runs off the cycle's critical path: the first snapshot and the first dispatch do not
  wait for it.
- Archiving prints one progress line per pass, `archive n of N`, and a tree with nothing to strip
  costs no fixed wait.
- The memory peak of a pass is bounded and recorded.

## Acceptance

- On a fixture instance with 200 archived trees, the first cycle completes its snapshot and starts
  its first controller before archiving has finished.
- The journal shows `archive n of N` lines.
