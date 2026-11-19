---
format: aep.planning-md/3
id: story:dispatch-v1-once-per-goal
kind: story
status: active
title: Dispatch v1 sends a goal to a repository once, and takes waiting repositories oldest first
summary: A done dispatch per repository and goal ends that repository's part; places go to the repository that has waited longest, not the first by name
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#204
relations:
- decomposes: epic:architecture-next
- informed_by: story:idle-controller-yields-place
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:21:30Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-19T18:21:30Z", actor: "human:unknown", revision: 4}
---
## Why

Issue #204 (friction F56). Four repositories finished a one-shot intake (`done`); 28 minutes later a
cycle sent them the same goal again and started their controllers, since they come first by name; the
three after them got nothing. The operator removed the serving after each `done` and cancelled the
duplicates.

Read in the code (release 1.16.0):

- `Tick::dispatch_for` (`crates/conductor/src/run.rs:844-904`): with no open dispatch
  (`Open::None`, `run.rs:851`) it sends a new one for the first goal the repository serves
  (`Serving`, goal `Confirmed`, else `Draft`, `run.rs:853-871`). A `Done` dispatch of the same
  repository and goal is no reason to stop.
- Dispatch v1 takes the Active repositories in the order `repository::activity` gives them, by name
  (`crates/conductor/src/run/dispatch.rs:299-303`, the loop at `:378-393`), while under
  `max_working`, so the same first repositories take every place that frees.

## Outcome

- Dispatch v1 sends no dispatch for a goal to a repository that already has a `Done` dispatch for
  that goal (`Tick::dispatch_for` passes such a goal over). A repository with no open dispatch whose
  every open goal it serves is done that way gets no controller from dispatch v1, and the `cycle`
  line says `<repo> skips goal <goal>: done in <dispatch>` for the first such pair. A `Failed` or
  `Cancelled` dispatch does not end the part. The operator sends one with `conductor dispatch
  send-dispatch` when more is wanted; a repository that serves no goal keeps today's start for its
  charter alone.
- Repositories waiting for a place are taken oldest waiting first: by the earlier of the oldest
  `sent_at` of their open dispatches and the first cycle that left them waiting (kept in
  `<state>/run/waiting.json`, dropped once a controller starts for them or they no longer wait); a
  repository that never waited comes after those that did; ties by name.

Decision (class C, controller, 2026-11-18): a `done` per repository and goal ends that repository's
part of the goal for dispatch v1; no "one-shot" kind is added to the serving, and a change of the goal
or its serving does not re-send by itself (neither records when it changed). Undone by re-sending by
hand or by a serving kind later (`spec/` first).

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_repository_with_a_done_dispatch_for_its_goal_is_sent_no_other`.
- `tests/run.rs` `repositories_waiting_for_a_place_are_taken_oldest_first_not_by_name`: with
  `max_working: 1`, the repository last by name that has waited longest gets the place.
- `tests/run.rs` `a_failed_dispatch_does_not_end_the_repository_s_part` (passes today; it guards the rule).

## Boundaries

- Freeing places held by idle controllers is `story:idle-controller-yields-place`.
