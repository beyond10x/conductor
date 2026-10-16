---
format: aep.planning-md/3
id: story:metrics-naming
kind: story
status: draft
title: Metrics and KPIs have one meaning, and the dashboard lists every metric
summary: A KPI is a metric with a target; one Metrics section lists all of them
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

The operator, 2026-10-14 ~23:2xZ: "one goal is simlpy having a good UI for conductor, that means all the metrics implemented, not just a few - also metric vs kpi ? its a bit missleading - get your nomen right". The dashboard has a Metrics page and a KPI page side by side; `docs/design/self-improvement.md` § 2 lists K1-K13 and T1-T4 as KPIs, most of them "to measure"; the stories that compute them (kpi-spec, kpi-records, kpi-transcripts, kpi-per-task, kpi-board, kpi-verify, health-kpis, instance-usage-metrics) are drafts.

## Acceptance

- One vocabulary, written once in `docs/design/self-improvement.md` § 2 and used by the CLI, the board, the dashboard and the site: a **metric** is one measured quantity with its unit, instrument and series over time; a **KPI** is a metric the operator set a target for, shown with its target and whether it meets it. Every KPI is a metric; not every metric is a KPI.
- The dashboard has one top-level Metrics section: KPIs first (value, target, met or missed, trend, source), then every other metric; the separate KPI page goes; every metric the design names is listed, and one not measured yet says so with the story that will measure it; a Vitest test checks that the page lists every metric of the design's table.
- No label, route or command says "KPI" for a figure that has no target; a test greps the dashboard app and the CLI help.

## Scope

`docs/design/self-improvement.md`, `crates/conductor/dashboard/src/`, `crates/conductor/src/dashboard.rs`, `website/docs/`, tests.
