---
format: aep.planning-md/3
id: epic:dashboard-charts
kind: epic
status: draft
title: The dashboard is polished and charts how the loop runs
revision: 1
---
## Why

The operator asked for a more polished dashboard and technical charts: token consumption (input and
output), merges, and further series an engineer and the operator read to see how the loop runs.
Today the dashboard shows the current state only: no history, no charts.

## Outcome

A Metrics page and charts on the overview and repository pages, drawn from what conductor records,
with the same definitions as the KPIs of `docs/design/self-improvement.md` § 2 so charts and KPIs
agree; and the whole app polished (layout, typography, empty and loading states, dark mode, small
screens).

## Series and their sources

| series | by | source today |
|---|---|---|
| PRs opened, merged, closed unmerged per day | repository | forge snapshot: pull-requests, merged-pull-requests (K1's denominator, K6) |
| reverts per day | repository | forge snapshot (merged PRs titled revert) |
| CI runs passed / failed per day | repository, workflow | snapshot workflow-runs (newest per workflow per snapshot: daily, not per run, until the collector keeps every run) |
| releases and merged → released age | repository | snapshot releases, `repository shipped` (K5) |
| dispatch sent → done (median, p90) | repository | dispatch store (K4) |
| operator touches per day | instance | decision store (K1, K2) |
| guard denials per day | rule, role | guard decisions (K10) |
| disk reservations: granted, refused, waited; peak vs reserved | repository | host ledger |
| tokens in and out | repository, session | **no source yet**: `story:kpi-transcripts` (K11) |
| idle controller time, waiting | repository, session | **no source yet**: `story:kpi-transcripts` (K7, K8) |

History beyond the snapshot retention comes from daily rollups (`story:dashboard-history`), which
become the KPI observations once `story:kpi-records` lands.
