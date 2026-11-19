---
format: aep.planning-md/3
id: story:dispatch-v1-once-per-goal
kind: story
status: draft
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
revision: 2
---
## Why

Issue #204 (friction F56). Four repositories finished a one-shot intake (`done`); 28 minutes later a
cycle sent them the same goal again and started their controllers, since they come first by name; the
three after them got nothing. The operator removed the serving after each `done` and cancelled the
duplicates.

Read in the code:

- `Tick::dispatch_for` (`crates/conductor/src/run.rs:916-975`): with no open dispatch
  (`Open::None`, `run.rs:923`) it sends a new one for the first goal the repository serves
  (`Serving`, goal `Confirmed`, else `Draft`, `run.rs:926-945`). A `Done` dispatch of the same
  repository and goal is no reason to stop.
- Dispatch v1 takes the Active repositories in the order `repository::activity` gives them, by name
  (`crates/conductor/src/run/dispatch.rs:256-260`, the loop at `:323-341`), while under
  `max_working`, so the same first repositories take every place that frees.

## Outcome

- Dispatch v1 sends no dispatch for a goal to a repository that already has a `Done` dispatch for
  that goal; the cycle line says `<repo> skips goal <goal>: done in <dispatch>` once per such pair.
  The operator sends one with `conductor dispatch send` when more is wanted.
- Repositories waiting for a place are taken oldest waiting first: by the oldest `sent_at` of their
  open dispatch, else the time their last controller stopped, a repository that never had one first;
  ties by name.

Decision (class C, controller, 2026-11-18): a `done` per repository and goal ends that repository's
part of the goal for dispatch v1; no "one-shot" kind is added to the serving, and a change of the goal
does not re-send by itself. Undone by re-sending by hand or by a serving kind later (`spec/` first).

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_repository_with_a_done_dispatch_for_its_goal_is_sent_no_other`.
- `tests/run.rs` `repositories_waiting_for_a_place_are_taken_oldest_first_not_by_name`: with
  `max_working: 1`, the repository last by name that has waited longest gets the place.
- `tests/run.rs` `a_failed_dispatch_does_not_end_the_repository_s_part` (passes today; it guards the rule).

## Boundaries

- Freeing places held by idle controllers is `story:idle-controller-yields-place`.
