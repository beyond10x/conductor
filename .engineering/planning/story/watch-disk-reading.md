---
format: aep.planning-md/3
id: story:watch-disk-reading
kind: story
status: implemented
title: The watch reads free disk again after a jump of more than 20G
scope:
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T20:25:58Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-12T20:25:58Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-12T20:30:03Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Observed in a live run: a conductor disk dispatch, raised from the watch, read 2118 MiB free; the
controller measured 56997 MiB 12 s later, and six further samples read 56969 to 57142 MiB. Two
workers were stopped for nothing.

The reading itself is no longer `df`: `story:portable-free-space` moved every reader to `statvfs`
(`crates/conductor/src/disk.rs:1-4`, `:25-38`), and the watch reads `/` through it
(`crates/conductor/src/watch.rs:250-251`). What is still open is that one odd reading is reported
as it stands. The watch's disk pass reads free disk once and compares it only with `disk_low` and
`disk_clear` (`crates/conductor/src/watch.rs:1071-1096`). It keeps no previous reading, so a
sudden jump is never checked.

## Acceptance

When a free-disk reading differs from the watch's previous reading by more than 20G, the watch
reads free disk again before it reports. A `disk low` line is written only when the second reading
is also under `disk_low`. A test in `crates/conductor/tests/watch.rs` (free disk 100G, then one
reading of 2G followed by 100G) shows no `disk low` line, and one where both readings are 2G shows
one.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
