---
format: aep.planning-md/3
id: story:failing-start-recreates-box
kind: story
status: draft
title: A start that keeps failing on a gone box recreates the box once, then alerts
summary: No silent retry of a stored box substrate no longer holds
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
revision: 1
---
## Why

An instance whose stored box names a workspace that no longer exists failed its start with
`resource.not-found` every minute for 19 hours with no alert (#207). The daemon's reconcile
(`crates/conductor/src/daemon/reconcile.rs`, `retry_after`, `story:box-crash-loop-reads-failed`)
backs off and retries the same stored box for ever.

## Outcome

A start that fails the same way repeatedly because the stored box is gone recreates the box once,
and, should that fail too, raises one alert instead of retrying silently.

## Acceptance

- After N failed starts in a row (a constant named in the code) whose runtime log or substrate
  answer names a missing resource of the stored box, the daemon drops the stored box reference and
  opens a new box for the instance, recorded as one line in the daemon log and the instance's
  start record.
- When the recreated start also fails, the instance reads `failed` with the cause on
  `/v1/instances` and in `conductor doctor`, and one alert line goes to the operator
  (`story:loop-alerts-when-it-cannot-act`).
- A test with a fake substrate answering `resource.not-found` fails on main.

## Boundaries

The daemon never removes a box's workspace data; it only forgets the reference to a box substrate
no longer holds.
