---
format: aep.planning-md/3
id: story:instance-records-start
kind: story
status: active
title: Conductor's tasks run conductor in the instance's records directory
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/conductor/tests/taskfile.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 3}
---
## Why

The guard gives conductor its rules only when its working directory is the instance's `records`
directory (`config show`: `instances[].records`), and the design keeps an instance's records out
of the product checkout. `Taskfile.yml` still starts, attaches and restarts conductor in the
product checkout, `task trust` trusts only checkouts under `checkouts.root`, and a session started
in another directory cannot find the product's `.claude/agents/` adapters. So an instance whose
records are not the product checkout cannot be run with the shipped tasks.

## Acceptance

- `task conductor:start`, `conductor:attach`, `conductor:restart` run conductor with the working
  directory the active instance's `records` (read from `conductor config show --format json`), the
  agent `conductor`, the model and settings of the `conductor` role, defaulting to this checkout's
  `.claude/conductor-settings.json`; `CONDUCTOR_PROMPT` reads the hand-over in that directory.
- `task trust` also marks the instance's `records` directory as a trusted workspace.
- `task agents:link` links `.claude/agents/{conductor,repo-controller,conductor-dev}.md` of this
  checkout into `~/.claude/agents/` (a symlink each; an existing different file is refused, not
  overwritten), so a session started in another directory finds them.
- A test that reads `Taskfile.yml` holds each of these (`crates/conductor/tests/watch.rs` already
  reads the Taskfile; add to the closest existing test file or a new `tests/taskfile.rs`).
- `README.md` and `AGENTS.md` are updated by the coordinator, not this unit.

## Scope

`Taskfile.yml` tasks `conductor:start`, `conductor:attach`, `conductor:restart`, `trust`, new
`agents:link` (cited); a Taskfile test (inferred).
