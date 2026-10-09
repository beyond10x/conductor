---
format: aep.planning-md/3
id: story:watch-tests-shared-isolation
kind: story
status: draft
title: The watch tests use the shared atomic config isolation
scope:
- confidence: cited
  path: crates/conductor/tests/adv_watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
revision: 2
---
## Why

Found by wave 03 U4 (report 2026-10-09), present before that wave:
- `crates/conductor/tests/watch.rs` and `crates/conductor/tests/adv_watch.rs` each have their own
  `isolate_active`, which writes one scratch config that every process of that test binary shares,
  without writing it atomically. Under nextest (one process per test) it races: 2 `watch.rs` tests
  failed with `version: missing`.
- Their scratch configs set neither `state` nor `records`, so both resolve under the real HOME.

`crates/conductor/tests/active/mod.rs` (`active::isolate`, from U4) writes a temp file and renames
it into place, and gives every path under the test directory.

## Acceptance

- `watch.rs` and `adv_watch.rs` use `active::isolate` and drop their own `isolate_active`.
- Their scratch configs name `state` and `records` under the test directory.
- `cargo nextest run -p conductor-cli --test watch --test adv_watch` passes 3 times in a row
  (logged), and the existing `cargo test` run still passes.

## Scope

`crates/conductor/tests/watch.rs`, `crates/conductor/tests/adv_watch.rs`.
