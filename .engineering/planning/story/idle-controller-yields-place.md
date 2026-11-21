---
format: aep.planning-md/3
id: story:idle-controller-yields-place
kind: story
status: implemented
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:21:29Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-19T18:21:30Z", actor: "human:unknown", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:35Z", actor: "human:unknown", revision: 8, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Issue #203 (friction F55). Four one-shot dispatches reported `done`; their controllers stayed
`Running`, idle, with no open dispatch, and for 30 minutes every cycle answered `max_working 4
reached` while three repositories waited. The operator stopped the four by hand.

Read in the code (release 1.16.0):

- `at_limit` counts every controller in state `Running`, whatever its dispatches
  (`crates/conductor/src/run.rs:983-992`); `Tick::spawn` (`run.rs:795-797`) and `start_work`
  (`run.rs:1044-1052`) refuse on it.
- The cycle's reconcile (`crates/conductor/src/run/dispatch.rs:154`, `lives_or_waits` at
  `run/dispatch.rs:259-264`) stops only records whose session is gone and not blocked; a live,
  idle session keeps its place.
- Nothing ends a controller when its repository's last dispatch reaches `Done`, `Failed` or
  `Cancelled`.

## Outcome

- When a start of the cycle needs a place and `max_working` controllers run (`Tick::make_room`,
  called by `Tick::spawn` and before each unit of work's start in dispatch v1), the `Running`
  controller of another repository that has no open dispatch and whose session the session list
  shows live and `idle` (every live entry of its name, by `status`, else `state`) is stopped
  through `controller stop-controller`, the one started first, one per start; its record's reason
  is `conductor run: no open dispatch; place given to <repo>`, and the `cycle` line names it:
  `stopped <repo> (<session>): no open dispatch; place given to <repo>`.
- A controller with an open dispatch, a busy session, a session not listed live, or a session list
  that does not read keeps its place. With no such idle controller the refusal is `max_working <n>
  reached`, as today.
- A repository on conductor's default charter without an open dispatch is refused for that before
  the place is looked at, so no controller is stopped for a start that cannot happen.

Decision (class C, controller, 2026-11-18): an idle controller is stopped only when a start needs its
place, not as soon as its last dispatch closes, so a follow-up dispatch can still reach it while
places are free; `at_limit` keeps counting every `Running` controller, since each holds memory. Undone
by stopping at once, or by counting only controllers with an open dispatch.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_start_at_max_working_stops_an_idle_controller_with_no_open_dispatch_first`.
- `tests/run.rs` `a_controller_with_an_open_dispatch_keeps_its_place_at_max_working` (passes today; it guards the rule).
- `tests/run.rs` `a_working_session_with_no_open_dispatch_keeps_its_place` (passes today; it guards the rule).
- `tests/run.rs` `a_cycle_leaves_running_controllers_whose_sessions_live_or_wait` gives its live, idle
  `child` session an open dispatch, which it now needs to keep its place.

## Boundaries

- Which repository takes the place, and sending one-shot work again, are
  `story:dispatch-v1-once-per-goal`.
- A limited session whose process is gone is `story:limit-resume-at-reset`.
