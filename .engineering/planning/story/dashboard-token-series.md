---
format: aep.planning-md/3
id: story:dashboard-token-series
kind: story
status: draft
title: Token consumption, idle and waiting time are charted per repository and session
relations:
- decomposes: epic:dashboard-charts
- depends_on: story:kpi-transcripts
- depends_on: story:dashboard-charts-ui
revision: 1
---
## Why

Token consumption was the operator's first example; idle and waiting time are the largest costs the retrospective measured.

## Acceptance

- With `story:kpi-transcripts`' instrument, the series `tokens_in`, `tokens_out`, `idle`, `waiting` are served by `/api/series` and charted on the Metrics page by repository and session; tests over fixture observations.

## Scope

`crates/conductor/src/dashboard*`, `crates/conductor/dashboard/src/**`, tests.
