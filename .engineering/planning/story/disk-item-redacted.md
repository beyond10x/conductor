---
format: aep.planning-md/3
id: story:disk-item-redacted
kind: story
status: draft
title: A disk item names no personal path
relations:
- decomposes: epic:disk-protocol
revision: 1
---
## Why

Conductor reported on 2026-10-14 that the disk item `conductor build run` raises on a refusal lists
`/var/tmp` entries by name in its options, among them a directory carrying the user's login name and
another carrying a third party's project name. The item lands in the records' decisions log, and the
Gates scan of the records diff refuses it as a personal path.

## Acceptance

- A disk item's text and options name the `/var/tmp` entries outside the instance only as a count
  and a total size ("4 entries outside the instance, 9G"), never by name; a test in
  `crates/conductor/tests/build_run.rs` with a fixture `/var/tmp` directory holding an entry named
  after the user holds it.
- Every path under the home directory in a disk item is written with `~/`; the same test holds it.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/src/dashboard/waste.rs`,
`crates/conductor/tests/build_run.rs`.
