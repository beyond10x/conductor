---
format: aep.planning-md/3
id: story:build-run-cancel-group
kind: story
status: draft
title: Stopping conductor build run stops its whole process group and releases its grant
revision: 1
---
# Story: build-run-cancel-group

## Outcome

A `conductor build run` that receives SIGTERM, SIGINT or SIGHUP leaves no build running and no disk reserved.

## Context

This is P1 of the operator-requested external review of 0.14.0 (0e7ce756), at crates/conductor/src/build.rs:435. A SIGTERM to `build run` leaves the supervised process group alive.

On 2026-10-15, conductor saw 17 reservations queued behind 4 long runs. conductor-dev found that killing a paused agent's `build run` processes left the build chains running.

## Acceptance

- `build run` forwards SIGTERM, SIGINT and SIGHUP to the supervised process group.
- If the group has not exited after a grace period, it is sent SIGKILL.
- The run is reaped, its peak is recorded, and its grant is finalized (released), and `build run` exits with 128+signal.
- A request still waiting in the queue when the signal arrives is withdrawn.
- Tests cover each signal for a running command (no process of the group survives, and the grant is Released) and for a queued request (Withdrawn).

## Out of Scope

Withdrawing a request whose session has stopped (story:build-run-session-bound).

## Ambiguities

None.
