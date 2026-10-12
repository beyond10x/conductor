---
format: aep.planning-md/3
id: story:agents-link-user-scope
kind: story
status: draft
title: task agents:link says it serves an interactive session started with the user's own settings
relations:
- decomposes: epic:operating-loop-hardening
- depends_on: story:restart-current-profile
- depends_on: story:instance-scoped-tasks
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 5
---
## Why

`task agents:link` (`Taskfile.yml:163`) links the adapters into `~/.claude/agents/` "so a session
started in another directory finds them". Every session of an instance starts with
`--setting-sources project,local`, which loads no user-level agent
(story:agent-needs-user-setting-source, wave 05), so the link serves none of them. Found by the
wave 05 implementor of that story. `AGENTS.md:177` already says the link serves none of those
sessions; only the Taskfile description still carries the old claim.

## Acceptance

- The `agents:link` description in `Taskfile.yml` no longer says "so a session started in another
  directory finds them" and says it serves an interactive session started with the user's own
  settings; a test in `crates/conductor/tests/taskfile.rs` holds the description.

## Notes

- Ordered after story:restart-current-profile, which also changes `Taskfile.yml` and
  `crates/conductor/tests/taskfile.rs`.

## Scope

`Taskfile.yml`, `crates/conductor/tests/taskfile.rs`.
