---
format: aep.planning-md/3
id: story:dispatch-waiting-reason
kind: story
status: draft
title: An open dispatch that gets no controller says why, on the dispatch and in the cycle line
summary: 'No snapshot, Inactive, no charter, cap reached or start refused: written once per change as a waiting line and shown by dispatch dispatches'
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#194
relations:
- decomposes: epic:architecture-next
scope:
- confidence: inferred
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: spec/domains/dispatch.yaml
revision: 3
---
## Why

Issue #194 (friction F19). One pass sent 8 dispatches; all stayed `Sent`, no controller started, and
nothing said why: no `start-refused` line, no dispatch log event, no doctor row. Every snapshot failed
in the box, so no repository was Active.

Read in the code:

- Dispatch v1 starts a controller for a `Sent` dispatch with a `reference` (a unit of work), and
  writes `start-refused` when that start is refused (`crates/conductor/src/run/dispatch.rs:290-321`).
- Every other open dispatch waits on its repository being Active with a charter: the loop runs over
  `repository::activity`'s Active rows only (`run/dispatch.rs:256-260`, `:323`), so a repository with
  no complete snapshot, or one Inactive, is passed over with no word; one without a charter is
  skipped by `continue` (`run/dispatch.rs:324-326`).
- `max_working reached` (`crates/conductor/src/run.rs:867-869`) and a refused spawn
  (`run.rs:900-904`) reach only the cycle line's `<repo> waits: ...`, and only for Active
  repositories (`run/dispatch.rs:337-353`). Nothing is written on the dispatch.

## Outcome

- After dispatch v1, every open dispatch (`Sent`, `Started`, `Blocked`) whose repository runs no
  controller carries why, one of: `no complete snapshot`, `repository Inactive`, `no charter`,
  `charter is the default: units of work only`, `max_working <n> reached`, `review cap <k>/<n>`,
  `start refused: <error>`, `left to the operator after <n> failed starts`.
- The reason is written as a dispatch log line `waiting` with the reason and the tick's id, once per
  change of reason for that dispatch; `dispatch dispatches` shows the newest one; the `cycle` line
  names each such repository `<repo> waits: <reason>`, Active or not.
- Spec first (AGENTS.md rule 1): the `DispatchLogEvent` variant `Waiting` and the dispatches view's
  `waiting` field are modelled in `spec/domains/dispatch.yaml`, then `task regen`.

Decision (class C, controller, 2026-11-18): the reason is a log line and a view field, not a dispatch
state; no transition is added. Undone by dropping the line.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `an_open_dispatch_with_no_complete_snapshot_says_why_no_controller_started`: a `Sent`
  dispatch, every snapshot failing; the dispatch log holds one `waiting` line `no complete snapshot`,
  `dispatch dispatches --format json` shows it, and the `cycle` line names the repository.
- `tests/run.rs` `an_open_dispatch_at_max_working_says_so_on_the_dispatch`.
- `tests/run.rs` `an_open_dispatch_without_a_charter_says_so_on_the_dispatch`.
- `tests/run.rs` `the_waiting_reason_is_written_once_until_it_changes`: two ticks with the same
  reason write one line; a third with another reason writes a second.

## Boundaries

- Whether a `Sent` dispatch of a repository that is not Active starts a controller anyway is not
  changed here.
- Why the snapshots failed in a box is the collectors' own errors and doctor's.
