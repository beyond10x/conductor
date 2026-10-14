---
format: aep.planning-md/3
id: story:findings-to-stories
kind: story
status: draft
title: conductor-dev turns findings into draft stories
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

Findings must reach the plan to change anything. (`docs/design/self-improvement.md`.)

## Acceptance

- `.agents/conductor-dev.md`: at session start and daily, each open finding becomes a draft story citing it and the KPI it should move with the expected effect, and the finding is marked filed; a profile test holds it.

## Scope

`.agents/conductor-dev.md`, `crates/conductor/tests/profiles.rs`.
