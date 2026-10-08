---
format: aep.planning-md/3
id: story:watch-disk-reading
kind: story
status: draft
title: The watch's free-disk reading matches df
revision: 1
---
## Why

Observed in a live run: a conductor disk dispatch, raised from the watch, read 2118 MiB free; the
controller measured 56997 MiB 12 s later, and six further samples read 56969 to 57142 MiB. Two
workers were stopped for nothing. Cause unknown. The watch runs `df -B1 --output=avail /`
(`crates/conductor/src/config.rs:233`, read in `crates/conductor/src/watch.rs` `disk`).

## Acceptance

Find where the 2118 MiB came from (watch log, statvfs field, path read); the watch reads available
blocks of `/` as `df --output=avail` does, and a reading that differs from the previous one by
more than 20G is read again before it is reported.
