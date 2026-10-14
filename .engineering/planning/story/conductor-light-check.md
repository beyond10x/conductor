---
format: aep.planning-md/3
id: story:conductor-light-check
kind: story
status: draft
title: A restarted conductor arms its light check itself
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

Conductor's session cron jobs die with each restart (six restarts on 2026-10-14); its light check
every 30 minutes (status, free disk, blocked dispatches, requests) had to be re-armed by hand.

## Acceptance

- `.agents/conductor.md`'s session-start list arms the light check (minutes 28 and 58) beside the
  cycle and the daily brief, and `task conductor:restart`'s start prompt names it; tests in
  `profiles.rs` and `taskfile.rs` hold both.

## Scope

`.agents/conductor.md`, `Taskfile.yml`, `crates/conductor/tests/profiles.rs`, `crates/conductor/tests/taskfile.rs`.
