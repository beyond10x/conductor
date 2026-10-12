---
format: aep.planning-md/3
id: story:agents-link-user-scope
kind: story
status: draft
title: task agents:link says what it serves, since project and local sources read no user agent
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 2
---
## Why

`task agents:link` (`Taskfile.yml:163`) links the adapters into `~/.claude/agents/` "so a session
started in another directory finds them", and `AGENTS.md` § Sessions says the same. Every session of
an instance starts with `--setting-sources project,local`, which loads no user-level agent
(story:agent-needs-user-setting-source, wave 05), so the link serves none of them. Found by the
wave 05 implementor of that story.

## Acceptance

- `task agents:link` is removed, or its description and `AGENTS.md` say what it still serves (an
  interactive session the operator starts with his own settings); a test in
  `crates/conductor/tests/taskfile.rs` holds whichever is chosen.

## Scope

`Taskfile.yml`, `AGENTS.md`, `crates/conductor/tests/taskfile.rs`.
