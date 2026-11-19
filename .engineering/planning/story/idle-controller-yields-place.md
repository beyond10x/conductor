---
format: aep.planning-md/3
id: story:idle-controller-yields-place
kind: story
status: draft
title: A controller with no open dispatch gives its max_working place to a repository that waits
summary: When a start needs a place, the cycle stops the longest idle controller with no open dispatch first and names it
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#203
relations:
- decomposes: epic:architecture-next
- informed_by: story:controllers-reconciled-with-live-sessions
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 2
---
## Why

Issue #203 (friction F55). Four one-shot dispatches reported `done`; their controllers stayed
`Running`, idle, with no open dispatch, and for 30 minutes every cycle answered `max_working 4
reached` while three repositories waited. The operator stopped the four by hand.

Read in the code:

- `at_limit` counts every controller in state `Running`, whatever its dispatches
  (`crates/conductor/src/run.rs:1055-1064`); `Tick::spawn` (`run.rs:867-869`) and `start_work`
  (`run.rs:1116-1123`) refuse on it.
- The cycle's reconcile (`crates/conductor/src/run/dispatch.rs:150`, `lives_or_waits` at
  `run/dispatch.rs:233-239`) stops only records whose session is gone or not blocked; a live, idle
  session keeps its place.
- Nothing ends a controller when its repository's last dispatch reaches `Done`, `Failed` or
  `Cancelled`.

## Outcome

- When a start needs a place and `max_working` controllers run, the cycle first stops a `Running`
  controller whose repository has no open dispatch and whose session the list does not show
  `working`, the one idle longest first, one per place needed; its record is stopped with the reason
  `no open dispatch; place given to <repo>`, and the `cycle` line names it.
- A controller with an open dispatch, or a working session, keeps its place. With no such idle
  controller the refusal is `max_working <n> reached`, as today.

Decision (class C, controller, 2026-11-18): an idle controller is stopped only when a start needs its
place, not as soon as its last dispatch closes, so a follow-up dispatch can still reach it while
places are free; `at_limit` keeps counting every `Running` controller, since each holds memory. Undone
by stopping at once, or by counting only controllers with an open dispatch.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_start_at_max_working_stops_an_idle_controller_with_no_open_dispatch_first`.
- `tests/run.rs` `a_controller_with_an_open_dispatch_keeps_its_place_at_max_working` (passes today; it guards the rule).
- `tests/run.rs` `a_working_session_with_no_open_dispatch_keeps_its_place` (passes today; it guards the rule).

## Boundaries

- Which repository takes the place, and sending one-shot work again, are
  `story:dispatch-v1-once-per-goal`.
- A limited session whose process is gone is `story:limit-resume-at-reset`.
