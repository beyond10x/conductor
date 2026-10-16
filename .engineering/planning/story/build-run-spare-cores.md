---
format: aep.planning-md/3
id: story:build-run-spare-cores
kind: story
status: draft
title: Builds leave the operator's desktop two cores and run at the lowest priority
revision: 1
---
# Story: build-run-spare-cores

## Outcome

The operator's desktop stays responsive while every build on the host runs: builds never take the last cores and always yield to interactive work.

## Context

The operator, 2026-10-15 ~23:5xZ: with all 20 CPUs busy, the UI lags; "can we somehow spare 2 cores or so, and not give them to the fleet". The fleet and the desktop share one login scope (`session-3.scope`), so a cgroup cap on the fleet alone needs the systemd provider (story:session-limits-systemd). The user manager delegates `cpu memory pids`, not `cpuset`, so AllowedCPUs is out and CPUQuota is in.

Interim, by hand at 23:59Z: every running build was reniced to 19 with idle I/O.

## Acceptance

- `conductor build run` starts its command at nice 19 with I/O class idle.
- It sets `CARGO_BUILD_JOBS` (and `MAKEFLAGS=-j`) to cores − `thresholds.cpu_reserve` (default 2), divided by the CPU slots once story:build-run-cpu-slots lands, unless the command sets them itself.
- Under the systemd provider, each instance's slice gets `CPUQuota=(cores − cpu_reserve) × 100%` and `CPUWeight=20`.
- Tests:
  - the command's own nice is 19, read from /proc;
  - `CARGO_BUILD_JOBS` is 18 with a fake core count of 20;
  - the slice's properties are set (fake systemctl).

## Out of Scope

Limits on sessions' own (non-build) processes beyond their slice.

## Ambiguities

None.
