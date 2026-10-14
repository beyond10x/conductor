---
format: aep.planning-md/3
id: story:kpi-board
kind: story
status: draft
title: The board and the dashboard show each KPI against its target with its trend
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

Measured KPIs help only where they are seen. (`docs/design/self-improvement.md`.)

## Acceptance

- `STATUS.md` gains a KPI section (value, target, 7-day trend, past-target marked); the dashboard's health view shows the same from `/api/kpis`; a status and a Vitest test read them.

## Scope

`crates/conductor/src/status.rs`, `crates/conductor/src/dashboard*`, `crates/conductor/dashboard/src/`, tests.
