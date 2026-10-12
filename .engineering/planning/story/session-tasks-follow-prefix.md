---
format: aep.planning-md/3
id: story:session-tasks-follow-prefix
kind: story
status: draft
title: Session tasks and the dashboard follow the instance's session names and thresholds
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 2
---
## Why

`task sessions`, `task conductor:attach` and `task dev:attach` match the bare names `conductor` and
`conductor-dev` (`Taskfile.yml`), so they miss the sessions of an instance with a
`session_prefix`, whose names are `<prefix>-conductor` and `<prefix>-conductor-dev`. The dashboard
also prints "over 300k" whatever `thresholds.context_handover` is, and the missing-program refusal
reads "a snapshot's collectors starts it". Found by the docs overhaul, 2026-10-12.

## Acceptance

- The three tasks take the session names from `conductor config show` (the role's `session_name`)
  and find a prefixed instance's sessions; tests in `tests/taskfile.rs`.
- The dashboard's context line names the configured threshold; test.
- The refusal reads "a snapshot's collectors start it".

## Scope

`Taskfile.yml`, `crates/conductor/tests/taskfile.rs`, `crates/conductor/src/dashboard/`,
`crates/conductor/src/doctor.rs`.
