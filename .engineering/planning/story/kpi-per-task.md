---
format: aep.planning-md/3
id: story:kpi-per-task
kind: story
status: draft
title: 'Per-task KPIs: LLM calls, tokens (in, out, cache), time and cost per dispatch'
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

The organisation measures deterministic engineering by model use per task; the dashboard's T1–T4 panels show 'not measured yet'.

## Acceptance

- A task is one dispatch (its controller session, resumed or not, and its subagents). The transcript reader records per dispatch: LLM calls (one per assistant response with usage), tokens in, out, cache read, cache write, wall time, and a list-price cost from a price table kept in the config (`kpi.prices`, per model); `/api/kpis` serves T1–T4 per repository; the four-week baseline is the first reading; tests over fixture transcripts.

## Scope

`crates/conductor/src/kpi/transcripts.rs`, `crates/conductor/src/dashboard*`, config spec (`kpi.prices`), tests.
