---
format: aep.planning-md/3
id: story:convoy-test-leaks-no-process
kind: story
status: draft
title: The build-queue tests leave no process behind
relations:
- decomposes: epic:process-supervision
revision: 2
---
# Story: convoy-test-leaks-no-process

## Outcome

The build-queue tests leave no process behind when they end, whether they pass or fail.

## Context

On 2026-10-17 at 10:0xZ, `worktree finish conductor-w39-convoy` refused with `worktree-in-use`. Process 1975323, `sh -c while [ ! -e stop ]; do sleep 0.02; done`, had been reparented to init 3 h 14 min earlier. Its working directory was `target/tmp/build_queue_convoy/no-change/work` (deleted). It came from `crates/conductor/tests/build_queue_convoy.rs`, case `no-change`: the case's script waits for a `stop` file, and the test removes the case directory without writing `stop` or killing the command. conductor-dev stopped it by pid. This breaks rules item 143: finished work leaves nothing behind.

## Acceptance

1. Every case in `build_queue_convoy.rs` that starts a waiting command ends it on every path: it writes `stop`, and a drop guard kills the run's process group if the case panics.
2. A test helper asserts that, after each case, no process has its working directory under the case's directory. It reads `/proc/*/cwd`, on Linux only.
3. The same helper runs in `build_run.rs` for every case that starts a waiting command.

## Out of Scope

A general process reaper for all tests.

## Ambiguities

None.
