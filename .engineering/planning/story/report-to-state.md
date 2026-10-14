---
format: aep.planning-md/3
id: story:report-to-state
kind: story
status: draft
title: A parsed report moves its dispatch without a model deciding
relations:
- decomposes: epic:deterministic-steps
revision: 1
---
## Why

Conductor reads each controller report and records the dispatch state by hand.

## Acceptance

- `message receive-message` with a parsed `[REPORT <repo> <id>] started|blocked|unblocked|done|failed …` records the matching `dispatch report-*` in the same command; a test per verb reads the dispatch state back; conductor's profile drops the step.

## Scope

`crates/conductor/src/message.rs`, `crates/conductor/src/dispatch.rs`, tests, `.agents/conductor.md`.
