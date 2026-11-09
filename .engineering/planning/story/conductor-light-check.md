---
format: aep.planning-md/3
id: story:conductor-light-check
kind: story
status: archived
title: A restarted conductor arms its light check itself
relations:
- decomposes: epic:loop-without-handwork
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:28:15Z", actor: "human:timo", revision: 3}
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

## Archived 2026-11-09 (draft sweep)

Targets .agents/conductor.md, deleted in commit fde22c979 (conductor session retired).
