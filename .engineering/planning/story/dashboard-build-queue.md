---
format: aep.planning-md/3
id: story:dashboard-build-queue
kind: story
status: draft
title: The dashboard shows the build queue and build times, per repository
revision: 1
---
# Story: dashboard-build-queue

## Outcome

The operator sees on the dashboard where build time goes: queued or running, for each repository, and how much disk is reserved beyond what builds use.

## Context

The operator's idea, relayed by a controller on 2026-10-15. Every host grant already carries requested_at, started_at, released_at, bytes, peak_bytes, repository and command. That day one repository's 14 builds spent 418 min queued against 97 min running, and peaked at 1.63G against 4G reserved.

These figures are metrics, so they appear in the one Metrics section of story:metrics-naming.

## Acceptance

- One endpoint, `/api/build-queue`, served from the memo, holds three groups.
  - **Queue now:** its length, each request's position and wait, and the oldest wait.
  - **Per repository, per day:** build count, queue minutes, run minutes, p50 and p95 of wait and of run, reserved bytes against peak bytes, and refused and killed runs with their shortfall.
  - **Slots:** once story:build-run-cpu-slots lands, slots in use and CPU wait against disk wait.
- The Metrics section has a Builds group that shows all three groups.
- Each figure is defined in `docs/design/self-improvement.md` § 2 as a metric with its unit.
- Tests:
  - Rust, over a fixture ledger with known grants: every figure matches a hand count.
  - Vitest: the page lists every figure.

## Out of Scope

Changing how the ledger schedules.

## Ambiguities

None.
