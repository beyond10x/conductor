---
format: aep.planning-md/3
id: story:build-run-short-lane
kind: story
status: draft
title: A short build does not wait behind long gates
relations:
- decomposes: epic:disk-protocol
revision: 2
---
# Story: build-run-short-lane

## Outcome

A short build does not wait behind long gates. One extra slot admits only commands the ledger knows to be short, so most runs start within about a minute.

## Context

Measured on 2026-10-17, 08:46–11:34Z:
- 75% of runs take under 2 min, yet they wait p50 12.5 min.
- 73% of slot-time went to 8 runs of 10 min or more.
- Long gates (30–90 min) hold 2 of the 3 slots while a stream of 1-minute tests shares the third.

The CPU is already near saturation, so this lane is narrow: one slot, few jobs.

## Acceptance

1. Spec first: `thresholds.short_lane_slots` (default 1; 0 turns the lane off) and `thresholds.short_lane_max` (default 3m) in `spec/domains/config.yaml`, validated with ess.
2. A request is eligible for the short lane when the ledger holds at least 3 earlier runs of the same repository and command in 14 days, and their p90 run time is at most `short_lane_max`. Test.
3. An eligible request that finds every regular slot taken goes to a free short-lane slot, ahead of longer requests and without breaking queue order among short ones. It runs with `CARGO_BUILD_JOBS` at most 4. Tests.
4. A short-lane run that outlives `2 × short_lane_max` is marked in the ledger, and the command loses short-lane eligibility for 24 h. It is not killed. Test.
5. `build run`'s queued line and `host grants` name the lane (`regular` or `short`). The dashboard's build queue shows it.
6. Queue order, backfill and the disk rules stay as they are for regular slots. The build_run and build_queue_convoy suites stay green.

## Out of Scope

Preemption of running builds.

## Ambiguities

None.
