---
format: aep.planning-md/3
id: story:cycle-command
kind: story
status: draft
title: '`conductor cycle` runs the cycle''s bookkeeping with no model call'
relations:
- decomposes: epic:deterministic-steps
revision: 1
---
## Why

Each cycle, conductor runs snapshot, board, reclaim, records check and KPI compute one prompt at a time.

## Acceptance

- `conductor cycle` runs them in order, bounded, and prints one summary (deviations, operator items, what changed); a systemd timer or cron line from `task cycle:install` runs it on `cadence.cycle`; conductor's profile reads the summary instead of running the steps; tests over a fixture instance.

## Scope

`crates/conductor/src/cycle.rs` (new), `Taskfile.yml`, `.agents/conductor.md`, tests.
