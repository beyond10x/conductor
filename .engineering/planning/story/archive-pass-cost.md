---
format: aep.planning-md/3
id: story:archive-pass-cost
kind: story
status: draft
title: An archive pass costs no fixed wait per tree and a bounded memory peak
summary: about 12 s per archived tree and a 6.1 GB peak over about 225 trees
revision: 2
---
## Why

Observed 2026-11-07 on an instance whose checkouts held about 225 archived managed trees: gc's
archive pass printed one `archive … nothing-to-strip|tree-still-present` line about every 12 s,
and the run unit's memory peak was 6.1 GB. Wave 43 moved gc beside the loop's ticks
(story:first-cycle-not-blocked-by-archiving) but did not change what one pass costs. The only
fixed wait found in the code is a 5 s grace for leftover output after `worktree` exited (`run` in
`crates/conductor/src/gc/archives.rs`); whether the rest is `worktree` itself is not measured.

## Acceptance

- The time per archived tree with nothing to strip is measured and recorded on the story, and a
  tree with nothing to strip costs no fixed wait in conductor.
- The memory peak of an archive pass over 200 archived trees is measured and bounded.
- A gc still running when the loop stops is either finished at the next start or logged as lost.
