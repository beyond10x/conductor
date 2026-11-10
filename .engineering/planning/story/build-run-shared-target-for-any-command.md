---
format: aep.planning-md/3
id: story:build-run-shared-target-for-any-command
kind: story
status: draft
title: A cargo started by any command under build run builds in the shared target
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
revision: 4
---
## Why

Two reports of 2026-11-10 disagree on what `conductor build run` should do with cargo's target directory, and neither is reproduced in a test.

- At the close of wave 49, `conductor build run -- task check` and `conductor build run -- zsh <script>` built in the tree's own `target/` (18 GB) beside the repository's shared target, although build.rs:94 says a run's command gets `CARGO_TARGET_DIR`. Which of the two it is (the variable not set for a command that is not `cargo`, or a child that drops it) is not known.
- A trial instance on conductor 1.0.1 reported units holding 10 GB in three private `target/` directories and a release compiled for 98 minutes from an empty target, while the host ledger refused three reservations in 85 minutes at under 40 GB free.
- The same instance then built two worktrees of one Rust workspace into one shared `CARGO_TARGET_DIR` and withdrew its first ask: cargo names workspace crates by relative path and decides freshness by modification time, so the tree with older sources linked the other tree's build of a workspace crate, and a test binary ran on code that did not hold the change under test. AGENTS.md says a run from another tree than the target's last marks its own files modified so cargo rebuilds; whether that held on that instance's version is not known.

What the instance asks for: a target per tree under the cache, outside the tree, counted by `conductor gc` and by the disk reservation and removed when the unit ends, with a shared cache for third-party dependencies as an option.

## Acceptance

- A test builds two trees of one workspace that differ in one workspace crate, one after the other under `conductor build run`, and each test binary holds its own tree's source. The design that makes it pass (a target per tree under the cache, or the shared target with a proof that the rebuild marking holds) is decided before the story is accepted.
- A `cargo` started by any command under `conductor build run` (a task runner, a shell script) builds in the directory the run chose and not in the tree's own `target/`; a test runs a script that prints `CARGO_TARGET_DIR`.
- `conductor gc` and the disk reservation count that directory.
