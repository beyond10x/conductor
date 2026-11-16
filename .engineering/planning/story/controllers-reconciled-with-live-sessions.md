---
format: aep.planning-md/3
id: story:controllers-reconciled-with-live-sessions
kind: story
status: draft
title: The cycle stops a Running controller record whose session no longer lives
summary: Before it counts controllers against max_working, the loop checks each Running record against the live session list, so a runtime restart or an OOM kill does not freeze the instance
tags:
- from-finding
relations:
- decomposes: epic:controller-health
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
revision: 1
---
## Why

Agentbox friction logs F31 and F42. Twice the controller records outlived their sessions and
the instance stopped working:

- 2026-11-15 13:39-14:10, after a runtime restart: five records were `Running`, one process
  lived, and four cycles said "max_working 5 reached".
- 2026-11-16 00:42: the OOM killer took the runtime and all 8 controllers, and the records
  stayed `Running`.

The operator stopped the stale records by hand both times.

Read at v1.13.0:

- `at_limit` counts the store's `Running` controllers and asks nothing about their sessions
  (`crates/conductor/src/run.rs:1003-1012`).
- A dead session is noticed only as `session gone`, from the watch's diff of one session list
  against the one before (`crates/conductor/src/watch.rs:1306`). A runtime that restarts has no
  "before" for sessions that died with it.

## Outcome

At each cycle, before `at_limit`, the loop checks every `Running` controller's session against
the session list. That includes child-provider sessions, by pid and id. A record whose session
is gone is stopped with a reason line, and its open dispatch is resumed as after
`session gone`.

## Acceptance

- After a runtime restart with two `Running` records whose sessions are gone, the next cycle
  stops both, records why, and starts a controller for an open dispatch within
  `max_working`. A run test with a fake provider holds it.
