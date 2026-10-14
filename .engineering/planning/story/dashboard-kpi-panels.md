---
format: aep.planning-md/3
id: story:dashboard-kpi-panels
kind: story
status: implemented
title: 'Every conductor KPI has a panel: value against target, trend and source, or not measured yet'
relations:
- decomposes: epic:dashboard-charts
- depends_on: story:dashboard-series-api
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:52:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T19:52:27Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T02:15:05Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator asked for every conductor KPI to be visible in the dashboard: K1–K12 of
`docs/design/self-improvement.md` § 2, and the per-task KPIs (tokens per task in, out and cache;
time per task; cost per task as a list-price equivalent). Showing them is approved; the loop that acts
on them (findings, stories from findings, the week-later check) still waits for the design's approval.

## What

- `GET /api/kpis` answers one row per KPI: id, title, definition, unit, target, current value, the
  series its trend reads (`/api/series` name), and its source; a KPI with no source answers
  `measured: false` with the story that adds it, and no value.
- A KPI page (`#/kpis`) and a KPI strip on the overview: one panel per KPI with the value against its
  target (past target marked), a trend line from its series, and its source. A KPI not measured yet
  shows "not measured yet" and the story id; never an empty chart or a zero.

Measured now, from records, store and forge snapshot: K1 operator touches per merged PR, K2
unnecessary escalations, K4 intent → merged, K5 merged → released, K6 rework, K10 guard denials, K12
defects shipped. Not measured yet: K3, K7, K8, K9, K11 and the per-task tokens, time and cost
(`story:kpi-transcripts`, and the per-task story to come).

## Acceptance

- `crates/conductor/tests/dashboard_series.rs` (or `dashboard_kpis.rs`): `/api/kpis` over fixtures
  answers the measured KPIs with values matching their definitions and the others with
  `measured: false` and a story id.
- Vitest: each panel renders a measured KPI with value, target and trend, and a not-measured KPI with
  "not measured yet" and its story, and renders no chart for it.

## Scope

`crates/conductor/src/dashboard*`, `crates/conductor/tests/`, `crates/conductor/dashboard/src/**`,
`dist/`.
