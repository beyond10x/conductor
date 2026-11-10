---
format: aep.planning-md/3
id: story:build-run-shared-target-for-any-command
kind: story
status: draft
title: A cargo started by any command under build run builds in the shared target
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
revision: 3
---
## Why

Seen at the close of wave 49 (2026-11-10): `conductor build run -- task check` and `conductor build run -- zsh <script>` built in the tree's own `target/` (18 GB) beside the repository's shared target, although build.rs:94 says a run's command gets `CARGO_TARGET_DIR`. Not reproduced in a test; which of the two (the variable not set for a command that is not `cargo`, or a child that drops it) is not known.

## Acceptance

- A `cargo` started by any command under `conductor build run` (a task runner, a shell script) builds in the repository's shared target; a test runs a script that prints `CARGO_TARGET_DIR` and sees the shared directory.

A trial instance reported the same on 2026-11-10 with conductor 1.0.1: units that ran cargo in their worktrees held 10 GB in three private `target/` directories and compiled a release for 98 minutes from an empty target, while the host ledger refused three reservations in 85 minutes at under 40 GB free. It asked that `build run --repository <name>` set `CARGO_TARGET_DIR` to the repository's shared target unless the caller sets one, or that the refusal name the private targets of the caller's own trees as reclaimable.
