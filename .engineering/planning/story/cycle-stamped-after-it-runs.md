---
format: aep.planning-md/3
id: story:cycle-stamped-after-it-runs
kind: story
status: active
title: A cycle is stamped when it ends, so a restart during it reruns it
summary: a restart during a cycle lost one snapshot and 2 h of cycles
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T15:58:44Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T15:58:44Z", actor: "human:timo", revision: 3}
---
## Why

Host supervisor, 2026-11-07: `crates/conductor/src/run.rs:318-319` writes `cycle.last` before `cycle_tick` runs. A loop restart at 15:33Z (the 0.22.0 install) stopped the cycle started 15:30:22Z; `cycle.last` read 15:30:22Z with no cycle tick recorded since 13:28Z, so the next cycle would have come at 17:30Z: one snapshot lost and 2 h without a cycle. `daily.last` is already written after its tick ends (`story:daily-tick-runs`).

## Acceptance

- `cycle.last` is written once the cycle tick ended, failed or not; a loop stopped during a cycle runs it again at its next start. A test stops the loop inside a cycle (a seam that blocks) and reads a cycle tick at the next start.
- `conductor run --uninstall` (and a unit stop) names a cycle in progress on standard error, with its start time, before it stops the loop.
