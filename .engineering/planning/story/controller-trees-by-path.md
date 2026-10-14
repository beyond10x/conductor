---
format: aep.planning-md/3
id: story:controller-trees-by-path
kind: story
status: draft
title: A controller works in managed trees by absolute path
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

An outside user's trial of conductor 0.12.0 (2026-10-14, macOS, a local source, four goals,
hands-off) found the guard denies a controller `EnterWorktree`, while `.agents/repo-controller.md`
tells it to work in managed trees without saying how; the controller found out by being denied and
then worked in the trees by absolute path.

## Acceptance

- `.agents/repo-controller.md` says a controller works in a managed tree by its absolute path and
  never through `EnterWorktree`, which the guard denies; `crates/conductor/tests/profiles.rs` holds
  the sentence, and a case in `crates/conductor/tests/guard_rules.rs` holds the denial it names.

## Scope

`.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`, `crates/conductor/tests/guard_rules.rs`.
