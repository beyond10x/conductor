---
format: aep.planning-md/3
id: story:dashboard-fast
kind: story
status: draft
title: Every dashboard endpoint answers warm in under 200 ms
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

The operator, 2026-10-14: "big loading gaps ... find ways to improve snappiness". Measured on the
instance that day: `/api/waiting-on` 8.2 s, `/api/releases` 7.7 s, `/api/board` 5.8 s,
`/api/workflow-runs` 4.5 s, `/api/specifications` 3.7 s, `/api/series` 3.6 s, `/api/kpis` 3.4 s;
each request opens the store and recomputes from the newest snapshot, and every view polls every
20 s.

## Acceptance

- The server computes each endpoint's answer once per new snapshot (or store write) and serves it
  from memory with an `ETag`; a repeat request answers in under 50 ms and with 304 when unchanged;
  a test holds both, and a measurement on a fixture store of the instance's size holds every
  endpoint under 200 ms warm.
- The app loads the overview's panels in parallel and shows each as it arrives.

## Scope

`crates/conductor/src/dashboard.rs`, `crates/conductor/src/dashboard/`, `crates/conductor/dashboard/src/`,
`crates/conductor/tests/dashboard_*.rs`.
