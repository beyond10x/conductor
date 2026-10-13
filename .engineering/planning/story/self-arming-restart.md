---
format: aep.planning-md/3
id: story:self-arming-restart
kind: story
status: draft
title: A restarted conductor arms its cycles and watch from the config, not the hand-over
relations:
- informed_by: story:restart-message
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 2
---
## Why

Conductor wrote 33 hand-overs between 2026-10-08 and 2026-10-13 (11, 5, 3, 1, 8, 5 per day), each a
restart at its context threshold; every new session re-arms its cron jobs, its lease and its watch by
hand from the hand-over's first section. `story:controller-start-reading` and
`story:restart-message` cover parts of this.

## Acceptance

- A restarted conductor session re-arms its scheduled cycles and its watch without reading them from
  the hand-over: `task conductor:restart` (or the session-start hook) passes them in the start
  prompt generated from the config (cadence, watch command), and the hand-over keeps only open
  work; a test over the Taskfile and the hook holds it.

## Scope

`Taskfile.yml`, `.claude/conductor-settings.json`, `.agents/conductor.md`,
`crates/conductor/tests/taskfile.rs`.
