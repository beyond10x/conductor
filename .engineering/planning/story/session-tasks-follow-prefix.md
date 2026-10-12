---
format: aep.planning-md/3
id: story:session-tasks-follow-prefix
kind: story
status: implemented
title: The dashboard's context line names the configured hand-over threshold
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T17:33:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-12T17:33:44Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-12T17:57:16Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

The dashboard prints "context: <name> over 300k tokens" (`crates/conductor/src/dashboard.rs:1096`)
whatever `thresholds.context_handover` is configured to (`crates/conductor/src/config.rs`), so an
instance with a lower or higher hand-over threshold reads a number that is not the one it acts on.
Found by the docs overhaul, 2026-10-12.

The same review found that `task sessions`, `task conductor:attach` and `task dev:attach` match the
bare names `conductor` and `conductor-dev` and miss a prefixed instance's sessions. Session-name
resolution in the Taskfile tasks is owned by story:instance-scoped-tasks (epic:multi-instance-host),
so this story no longer claims it. The missing-program refusal wording moved to
story:doctor-refusal-wording.

## Acceptance

- The dashboard's context line names the configured `thresholds.context_handover` instead of a
  fixed "300k"; a test in `crates/conductor/tests/dashboard.rs` (beside
  `the_page_shows_the_watch_usage_limit_and_context_lines`) holds it.

## Scope

`crates/conductor/src/dashboard.rs`, `crates/conductor/tests/dashboard.rs`.
