---
format: aep.planning-md/3
id: story:failed-start-stop-covers-every-start
kind: story
status: draft
title: The stop after three failed starts covers every controller start the loop makes
tags:
- from-finding
relations:
- informed_by: review-result:work-start-security-round-1
revision: 1
---
## Why

Release 1.7.0 stops the loop from starting a unit of work again after three starts that failed
at the launch (commit `45ede59cd2`, the fix of finding 2 of
`review-result:work-start-security-round-1`). The stop covers the start of a unit only, and its
case was written after the code; the commit message says both. Read at `v1.7.0` (`951330f4f4`).

- What counts: `failed_starts` counts the `start-refused` lines of one dispatch whose reason is
  `StartFailed` or `LaunchFailed` (`crates/conductor/src/dispatch.rs:327-352`), against
  `FAILED_STARTS_MOST`, 3 (`dispatch.rs:325`). `ControllerLimit` and `AlreadyRunning` do not
  count (`dispatch.rs:328-329`). The lines are written by the work call's own start
  (`crates/conductor/src/dashboard/work.rs:234`) and by the cycle's start of a unit
  (`crates/conductor/src/run/dispatch.rs:190-192`).
- Where it holds: the cycle's loop over units of work skips a unit at the bound and writes one
  line that it is left to the operator (`run/dispatch.rs:173-208`).
- What does not count: every other start the loop makes goes through `Tick::spawn`
  (`crates/conductor/src/run.rs:812-854`), which answers a failed `spawn_for` with a string and
  writes no `start-refused` line (`run.rs:852`). Its callers: the start for each Active
  repository with a charter (`run/dispatch.rs:209-223`), the restart of a controller
  (`restart_with`, `run.rs:949`, `:980`) and the start on a red `main`
  (`crates/conductor/src/run/handlers.rs:753`). Nothing counts their failed launches, so each
  of them tries a repository whose launch fails again whenever it comes to it.
- What each failed `Tick::spawn` leaves behind (a controller record or not): not read.
- The case: `a_unit_whose_start_fails_is_tried_three_times_and_then_left_to_the_operator`
  (`crates/conductor/tests/work_start.rs:941`), added in `45ede59cd2` with the code; its message
  says it was not seen to fail.

## Acceptance

- A repository with a charter whose controller fails at the launch is started at most three
  times by the cycle, and one line says it is left to the operator; a case runs five cycles and
  counts the launches.
- A restart and a red `main` count against the same bound; a case each.
- The case for a unit's start is seen red with the stop taken out, and green with it; the
  change says how this was shown.
