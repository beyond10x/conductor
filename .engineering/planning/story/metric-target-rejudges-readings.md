---
format: aep.planning-md/3
id: story:metric-target-rejudges-readings
kind: story
status: draft
title: A metric target set after a day's compute judges that day's readings
summary: readings before the target are kept unjudged, so kpi-readings shows no row
revision: 2
---
## Why

Observed 2026-11-07 on two instances: `conductor metric compute --day 2026-11-06`, run after
`metric set-metric-target`, printed `<id>: already recorded, kept as it is` for the readings of
that day recorded before the target was set, and `metric kpi-readings` shows no 2026-11-06 row.
On one instance K5.p90 read 44.342 against a target of 4 and was never shown as `met=false`.

Verified in the code: a reading the store holds is kept as it is
(`crates/conductor/src/metric.rs:752`, `RecordMetricReadingOutcome::AlreadyRecorded`), and the
target is judged only when a reading is recorded (`metric.rs:22`); `KpiReadings` lists only
readings with `defined(target)` (`spec/domains/improvement.yaml:844`), and its comment says a KPI
whose newest reading predates its target "has no row until its next reading".

## Acceptance

- After `set-metric-target`, `metric compute --day <d>` for a day already computed judges that
  day's readings of the metric against the target, and `metric kpi-readings` lists the KPI with
  its target and `met`.
- Or `set-metric-target` itself judges the metric's newest reading, so `kpi-readings` lists it at
  once.
- A test sets a target after a day's compute and reads one `kpi-readings` row with `met=false`
  for a value that misses it.
