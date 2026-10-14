---
format: aep.planning-md/3
id: story:kpi-spec
kind: story
status: draft
title: The KPIs, their daily observation and friction findings are specified first
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

The nouns of the loop — `Kpi` (id, definition, target, instrument), `KpiObservation` (kpi, day, value, sources), `FrictionFinding` (signature, evidence ids, kpi, state Open → Filed → Verified | Closed) — have no typed home. (`docs/design/self-improvement.md`.)

## Acceptance

- `spec/domains/improvement.yaml` declares them with the commands `RecordKpi`, `RaiseFinding`, `FileFinding`, `VerifyFinding` and the views `Kpis`, `Findings`; `ess specify validate --path spec` exits 0 with no UNMAPPED marker.

## Scope

`spec/domains/improvement.yaml`, `spec/system.yaml`, `spec/ess-inputs.yaml`, `spec/components.yaml`, `crates/conductor/cli.yaml`.
