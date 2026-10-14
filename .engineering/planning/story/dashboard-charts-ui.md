---
format: aep.planning-md/3
id: story:dashboard-charts-ui
kind: story
status: active
title: A Metrics page and charts on the overview and repository pages
relations:
- decomposes: epic:dashboard-charts
- depends_on: story:dashboard-series-api
- depends_on: story:dashboard-polish
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:48:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T19:48:59Z", actor: "human:timo", revision: 3}
---
## Why

The operator asked for technical charts.

## Acceptance

- A Metrics page with a chart per series (lines for rates, stacked bars for counts, a filter by repository and range of 7, 30 or 90 days), small trend charts on the overview (merges, CI pass rate, operator touches) and per repository; charts drawn as inline SVG components (no chart library, inside the app's CSP); Vitest tests render each chart from fixture series.

## Scope

`crates/conductor/dashboard/src/**`, `dist/`.
