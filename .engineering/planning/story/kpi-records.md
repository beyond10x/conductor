---
format: aep.planning-md/3
id: story:kpi-records
kind: story
status: draft
title: K1, K2, K4, K5, K6 and K12 are computed from the store, the records and the forge snapshot
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

The baseline of `docs/design/self-improvement.md` § 2 was computed by hand; the loop needs it daily. (`docs/design/self-improvement.md`.)

## Acceptance

- `conductor kpi compute --day <date>` records one `KpiObservation` per KPI for that day; over a fixture of records and snapshot rows it reproduces the definitions of § 2 (a test per KPI with a hand-counted expected value).

## Scope

`crates/conductor/src/kpi.rs` (new), `crates/conductor/tests/kpi.rs`.
