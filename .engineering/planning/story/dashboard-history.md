---
format: aep.planning-md/3
id: story:dashboard-history
kind: story
status: draft
title: Daily rollups keep the series beyond the snapshot retention
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

Charts need days and weeks; the store keeps the newest snapshots only.

## Acceptance

- Each completed snapshot updates one rollup per day and series of the table in `epic:dashboard-charts` from data the store holds; rollups survive snapshot retention; a test over two days of fixture snapshots reads the rollups back. Once `story:kpi-records` lands, the KPI series read the KPI observations instead.

## Scope

`spec/domains/observation.yaml` (the rollup, spec first), `crates/conductor/src/snapshot.rs` or a new rollup module, tests.
