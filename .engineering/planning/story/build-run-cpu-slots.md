---
format: aep.planning-md/3
id: story:build-run-cpu-slots
kind: story
status: draft
title: Builds take a CPU slot from the host ledger, so the host runs as many as its cores serve
revision: 1
---
# Story: build-run-cpu-slots

## Outcome

Compiling on the host is scheduled centrally: at most as many builds run at once as the cores serve, each with a known job count. A unit's `task check` takes its own compile time, not that time multiplied by every other build running beside it.

## Context

The operator asked through a controller on 2026-10-15, with measurements from that day:
- one `task check` in a unit tree took 60–74 min;
- the dev build alone took 8m45s and the test build 10m28s;
- 141 crates compile into a 2.3–2.6G `target/` per tree;
- the load average was 21.6 on 20 cores.

Every controller and every implementor runs its own builds and gates in its own tree, and the ledger (design § 9) reserves disk only. A shared or larger sccache was ruled out by the operator.

## Acceptance

- `conductor build run` takes a CPU slot as well as its disk reservation.
  - The host has `thresholds.build_slots` slots, by default cores / 8, at least 1.
  - The run's command gets `CARGO_BUILD_JOBS` set to cores / slots, unless the command sets it itself.
- Slots are granted in arrival order, with the same backfill rule as disk (story:build-run-backfill). A run releases its slot when it ends or is stopped (story:build-run-cancel-group).
- `conductor host grants` shows the slot held by each run.
- `--light` (fmt, a clippy of one crate, docs-check) takes no slot.
- Tests with a fake core count:
  - three runs with 2 slots give two running and one queued;
  - a stopped run frees its slot;
  - `CARGO_BUILD_JOBS` is set in the command's environment.
- Design § 9 states that compiling is scheduled by the host ledger, not per controller.

## Out of Scope

A shared build service, and a shared target directory per repository. Either becomes a story only if slots are measured as not enough.

## Ambiguities

None.

## Evidence (conductor, 2026-10-15)

thalamus's 14 released grants that day: 418 min queued against 97 min running, so 81 % of build wall time was queue wait. The highest peak was 1.63G against a 4G reservation. The queue wait is mostly the disk ledger's head-of-line hold (story:build-run-backfill), so measure again after that ships, before sizing the slots.
