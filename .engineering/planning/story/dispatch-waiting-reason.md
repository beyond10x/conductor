---
format: aep.planning-md/3
id: story:dispatch-waiting-reason
kind: story
status: active
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
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: inferred
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/dispatch/behaviour.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/store/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/adv_cli.rs
- confidence: cited
  path: crates/conductor/tests/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: spec/components.yaml
- confidence: inferred
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/scenarios/record-dispatch-waiting-cleared.yaml
- confidence: cited
  path: spec/scenarios/record-dispatch-waiting.yaml
- confidence: cited
  path: website/docs/concepts/the-loop.md
- confidence: cited
  path: website/docs/guides/start-conductor.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:21:30Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-19T18:21:30Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #194 (friction F19). One pass sent 8 dispatches; all stayed `Sent`, no controller started, and
nothing said why: no `start-refused` line, no dispatch log event, no doctor row. Every snapshot failed
in the box, so no repository was Active.

Read in the code (release 1.16.0):

- Dispatch v1 starts a controller for a `Sent` dispatch with a `reference` (a unit of work), and
  writes `start-refused` when that start is refused (`crates/conductor/src/run/dispatch.rs:344-376`).
- Every other open dispatch waits on its repository being Active with a charter: the loop runs over
  `repository::activity`'s Active rows only (`run/dispatch.rs:299-303`, `:378`), so a repository with
  no complete snapshot, or one Inactive, is passed over with no word; one without a charter is
  skipped by `continue` (`run/dispatch.rs:379-381`).
- `max_working reached` (`crates/conductor/src/run.rs:795-797`) and a refused spawn
  (`run.rs:831`) reach only the cycle line's `<repo> waits: ...`, and only for Active
  repositories. Nothing is written on the dispatch, which has no field for it
  (`spec/domains/dispatch.yaml`, `conductor.dispatch.Dispatch`).

## Outcome

- After dispatch v1, every open dispatch (`Sent`, `Started`, `Blocked`) whose repository runs no
  `Running` or `Paused` controller carries why, one of: `no complete snapshot`, `not observed in the
  newest snapshot`, `repository Inactive`, `no charter`, `the charter is conductor's default: ...`,
  `max_working <n> reached`, `<k>/<n> pull requests` (the review cap), `usage limit until <reset>`,
  `skips goal <goal>: done in <dispatch>`, the refusal of a unit of work's start
  (`<WorkStartRefusal>: <detail>`), `<n> starts of it failed; the loop tries it no more`, or the
  spawn's error.
- The reason is the dispatch's `waiting` field, shown by `dispatch dispatches`, set through the new
  command `conductor.dispatch.RecordDispatchWaiting` (`dispatch record-waiting`), which writes one
  dispatch log line `waiting` with `id` and `reason` (and the tick's `tick_id` when the loop writes
  it) only when the reason changes; the same reason again changes nothing and writes nothing. Once
  a controller of the repository runs, the cycle clears the field (no line). The `cycle` line names
  each such repository `<repo> waits: <reason>`, Active or not.
- Spec first (AGENTS.md rule 1): the field, the command, its event `DispatchWaitingRecorded`, the
  `DispatchLogEvent` variant `Waiting` and the view's `waiting` are modelled in
  `spec/domains/dispatch.yaml`, with a scenario, then `task regen`.

Decision (class C, controller, 2026-11-18): the reason is a field, a log line and a view column, not
a dispatch state; no transition is added. Undone by dropping the field and the line.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `an_open_dispatch_with_no_complete_snapshot_says_why_no_controller_started`: a `Sent`
  dispatch, no complete snapshot; the dispatch log holds one `waiting` line `no complete snapshot`
  with the tick's id, `dispatch dispatches --format json` shows it in `waiting`, and the `cycle`
  line names the repository.
- `tests/run.rs` `an_open_dispatch_at_max_working_says_so_on_the_dispatch`.
- `tests/run.rs` `an_open_dispatch_without_a_charter_says_so_on_the_dispatch`.
- `tests/run.rs` `the_waiting_reason_is_written_once_until_it_changes`: two ticks with the same
  reason write one line; a third with another reason (`repository Inactive`) writes a second.
- `spec/scenarios/record-dispatch-waiting.yaml`, `record-dispatch-waiting-cleared.yaml` (the
  conformance run): the command's outcomes `waiting`, `unchanged`, `invalid-reason`, `cleared`.
- `tests/dispatch.rs` and `tests/adv_cli.rs` hold the view's new column and the new leaf
  `dispatch record-waiting`.

## Boundaries

- Whether a `Sent` dispatch of a repository that is not Active starts a controller anyway is not
  changed here.
- Why the snapshots failed in a box is the collectors' own errors and doctor's.
