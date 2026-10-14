---
format: aep.planning-md/3
id: story:context-handover-per-repo
kind: story
status: draft
title: A repository can set its own context hand-over threshold
relations:
- decomposes: epic:conductor-autonomy
revision: 1
---
## Why

Conductor's ess velocity study (2026-10-13): ess controllers ran past 300k context in 34 sessions, and ess used 58 % of all input tokens; one instance-wide `thresholds.context_handover` fits no repository that heavy.

## Acceptance

- `repositories[].context_handover` (declared first in `spec/domains/config.yaml`) overrides `thresholds.context_handover` for that repository; the watch's `context:` line and conductor's hand-over use it; config and watch tests.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/config.rs`, `crates/conductor/src/watch.rs`, tests, `docs/config.md`.
