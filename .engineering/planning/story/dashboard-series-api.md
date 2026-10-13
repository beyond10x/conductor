---
format: aep.planning-md/3
id: story:dashboard-series-api
kind: story
status: draft
title: '`/api/series` serves each chart''s series by day, repository and session'
relations:
- decomposes: epic:dashboard-charts
- depends_on: story:dashboard-history
revision: 1
---
## Why

One endpoint the charts read, with the KPIs' definitions.

## Acceptance

- `GET /api/series?series=<name>&by=<repository|session|workflow|rule>&days=<n>` answers rows of day, key and value for every series of the epic's table that has a source; a series without one answers 404 naming the story that adds it; endpoint tests per series over fixtures.

## Scope

`crates/conductor/src/dashboard*`, `crates/conductor/tests/dashboard_series.rs`.
