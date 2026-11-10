---
format: aep.planning-md/3
id: story:build-run-shared-target-for-any-command
kind: story
status: draft
title: A cargo started by any command under build run builds in the shared target
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
revision: 2
---
## Why

Seen at the close of wave 49 (2026-11-10): `conductor build run -- task check` and `conductor build run -- zsh <script>` built in the tree's own `target/` (18 GB) beside the repository's shared target, although build.rs:94 says a run's command gets `CARGO_TARGET_DIR`. Not reproduced in a test; which of the two (the variable not set for a command that is not `cargo`, or a child that drops it) is not known.

## Acceptance

- A `cargo` started by any command under `conductor build run` (a task runner, a shell script) builds in the repository's shared target; a test runs a script that prints `CARGO_TARGET_DIR` and sees the shared directory.
