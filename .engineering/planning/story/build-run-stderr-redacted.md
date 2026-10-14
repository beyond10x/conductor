---
format: aep.planning-md/3
id: story:build-run-stderr-redacted
kind: story
status: draft
title: A refused build run prints no personal path
relations:
- decomposes: epic:disk-protocol
revision: 1
---
## Why

`story:disk-item-redacted` took personal paths out of the disk item, but a refused `conductor build
run` still prints each leftover path in full on standard error (`left: <path>`), and sessions quote
that line into their reports and records (wave 33, unit D's report).

## Acceptance

- A refused `build run` prints `/var/tmp` entries outside the instance as a count and a total size
  and the home directory as `~`, as the disk item does; a test in
  `crates/conductor/tests/build_run.rs` with a fixture entry named after the user holds it.

## Scope

`crates/conductor/src/build.rs`, `crates/conductor/src/host.rs`, `crates/conductor/tests/build_run.rs`.
