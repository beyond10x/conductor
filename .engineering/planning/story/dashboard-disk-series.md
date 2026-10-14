---
format: aep.planning-md/3
id: story:dashboard-disk-series
kind: story
status: draft
title: The dashboard charts disk peak against reserved
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

`epic:dashboard-charts` lists disk "peak against reserved" per build, but wave 27 shipped no series
for it: the ledger records a run's peak (`host record-peak`) and its reservation, and nothing rolls
them up.

## Acceptance

- `/api/series` serves `disk-peak-vs-reserved` per day and repository from the host ledger's runs;
  a test in `crates/conductor/tests/dashboard_series.rs` over two runs holds it, and the Metrics page
  draws it.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/rollup.rs`,
`crates/conductor/src/dashboard/series.rs`, `crates/conductor/tests/dashboard_series.rs`,
`crates/conductor/dashboard/src/views/Metrics.vue`.
