---
format: aep.planning-md/3
id: story:dev-restart-and-idle
kind: story
status: draft
title: conductor-dev restarts itself from its hand-over, and an idle conductor-dev is seen
summary: task dev:restart detached; watch flags 2 h without wave progress
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

The operator, 2026-10-14 ~22:2xZ: "came here after hours, finding yourself inactive - not running - you that is not acceptable at all". conductor-dev moved no wave from ~19:00Z to 22:30Z: it decided to hand over at 200k, but nothing restarts conductor-dev, and it had no job that woke it between messages (rules item 135).

## Acceptance

- `task dev:restart` stops the active instance's conductor-dev and starts a new one that reads its newest hand-over in `<state>/conductor-dev/handoff/`, detached from the session it stops (`systemd-run --user` or a background process), so conductor-dev can run it on itself; its start prompt arms the recurring work tick; a Taskfile test reads the command lines.
- The watch prints `idle conductor-dev <hours>h <wave>` when conductor-dev's transcript shows no commit or tool call on an open wave's branch for 2 hours, and conductor's profile answers it by messaging conductor-dev; a test with a fixture transcript and clock.

## Scope

`Taskfile.yml`, `crates/conductor/src/watch.rs`, `.agents/conductor.md`, `.agents/conductor-dev.md`, tests.
