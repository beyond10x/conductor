---
format: aep.planning-md/3
id: story:watch-tree-sizes
kind: story
status: draft
title: The watch reports the largest managed trees by size
revision: 1
---
## Why

Observed in a live run: conductor ran 11 du/gc scans over all managed trees by hand in one day,
440 s in total; two hit the 120 s tool timeout. `conductor watch` gives free space on `/` only
(`crates/conductor/src/watch.rs`).

## Acceptance

Under the disk threshold, a watch line lists the 5 largest trees under the instance's trees root
with their size and age, measured at most once per hour and cached.
