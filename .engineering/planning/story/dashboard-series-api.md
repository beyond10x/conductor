---
format: aep.planning-md/3
id: story:dashboard-series-api
kind: story
status: implemented
title: '`/api/series` serves each chart''s series by day, repository and session'
relations:
- decomposes: epic:dashboard-charts
- depends_on: story:dashboard-history
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:48:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T19:48:59Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T02:15:05Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

One endpoint the charts read, with the KPIs' definitions.

## Acceptance

- `GET /api/series?series=<name>&by=<repository|session|workflow|rule>&days=<n>` answers rows of day, key and value for every series of the epic's table that has a source; a series without one answers 404 naming the story that adds it; endpoint tests per series over fixtures.

## Scope

`crates/conductor/src/dashboard*`, `crates/conductor/tests/dashboard_series.rs`.
