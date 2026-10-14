---
format: aep.planning-md/3
id: story:health-kpis
kind: story
status: draft
title: 'Efficiency KPIs: merged work per token, build wait, context overrun, short cache'
relations:
- decomposes: epic:controller-health
- depends_on: story:health-command
revision: 1
---
## Why

The KPI board (K1-K12) has no efficiency measure: nothing shows merged work per token, build
waiting or context overrun, which the first health report found to be the instance's largest costs.

## Acceptance

- Spec first: KPIs K14 merged pull requests per 100M input tokens (daily), K15 median build-queue
  wait, K16 share of turns over `thresholds.context_handover`, K17 share of cache writes with the
  5-minute TTL, each with a target in config, computed from stored health reports; `/api/kpis`
  serves them and the KPI page draws them with their trend. Tests in
  `crates/conductor/tests/dashboard_kpis.rs` and Vitest hold them.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/dashboard/series.rs`,
`crates/conductor/src/rollup.rs`, `crates/conductor/dashboard/src/views/kpi/`,
`crates/conductor/tests/dashboard_kpis.rs`.
