---
format: aep.planning-md/3
id: story:build-jobs-per-slot
kind: story
status: draft
title: Build run sets each build's parallelism from the cores and the build slots
relations:
- decomposes: epic:disk-protocol
revision: 2
---
# Story: build-jobs-per-slot

## Outcome

`conductor build run` sets each build's parallelism from the host's cores and its build slots, so the admitted builds together leave the desktop its spare cores. No session has to pass `CARGO_BUILD_JOBS`.

## Context

At 10:17Z on 2026-10-17, after 0.17.2 brought `thresholds.build_slots` (3), 3 builds were Running and 14 Requested. Load was 36 on 20 cores, because every build passed `CARGO_BUILD_JOBS=18` as rules item 142 said. `build run` already sets `CARGO_BUILD_JOBS` and `MAKEFLAGS=-j` to the cores less `thresholds.cpu_reserve`, unless the environment holds them (crates/conductor/src/build.rs, `story:build-run-spare-cores`). That ignores the slots, and an explicit variable overrides it. Interim: rules item 142 now passes 6 (fe70efb).

## Acceptance

1. `build run` sets `CARGO_BUILD_JOBS` and `MAKEFLAGS=-j` to `max(1, (cores - cpu_reserve) / build_slots)`, or to `cores - cpu_reserve` when `build_slots` is 0. Test.
2. A `CARGO_BUILD_JOBS` higher than that share is lowered to it, and `build run` prints one line saying so. A lower one is kept. Tests.
3. The profiles drop `env CARGO_BUILD_JOBS=…` from their command forms, and the profiles test pins the new form.
4. Rules item 142 is rewritten to match when this ships, a rules.md change conductor-dev makes.

## Out of Scope

Admission by measured load.

## Ambiguities

None.
