---
format: aep.planning-md/3
id: story:resumed-controller-done-held
kind: story
status: draft
title: A resumed controller is told its dispatch first, a done right after a resume is held, and a done dispatch can be reopened
summary: Resume names the open dispatch as its first input; a done within ten minutes of a resume is held for the operator; dispatch reopen-dispatch undoes a wrong done
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#202
relations:
- decomposes: epic:architecture-next
- informed_by: story:started-report-after-restart
- informed_by: story:dispatch-done-from-blocked
scope:
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests/dispatch.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 3
---
## Why

Issue #202 (friction F54). After a box restart a controller was resumed from its transcript; within
the same second it reported `started` and then `done` on its open dispatch for goal B with the
evidence of an earlier goal A. The next tick recorded `unblocked` and `done`; the dispatch closed. The
controller's own withdrawal (`unblocked`) was refused, and the work was sent again as a new dispatch.

Read in the code:

- A resume continues the transcript and sends no first input: `spawn::resume`
  (`crates/conductor/src/spawn.rs:573-590`) runs `claude --bg --resume <id>`
  (`crates/conductor/src/confine/mod.rs:425`) or `claude -p --resume <id>` "with no prompt"
  (`crates/conductor/src/confine/child.rs:37`). A fresh spawn's charter names the dispatch; a resumed
  session goes back to what its context said last.
- The mailbox takes a `started` on a `Blocked` dispatch as `unblocked`
  (`crates/conductor/src/run/mailbox.rs:294-305`) and a `done` as `dispatch report-done`
  (`run/mailbox.rs:342-352`), which moves `Sent`, `Started` or `Blocked` to `Done` on any evidence
  (`crates/conductor/src/dispatch.rs:889-916`, `spec/domains/dispatch.yaml:562-563`). Nothing compares
  the report with the dispatch or with when the session was resumed.
- `Done` is final: no command reopens it, and `report-unblocked` on it is refused
  `DispatchStateConflict` (`dispatch.rs:835`).

## Outcome

- A controller session resumed by the loop or by the runtime's restart gets, as its first input, its
  open dispatch: id, goal and brief, in the message format of design § 6, as a fresh spawn's charter
  names it. A session with no open dispatch is told it has none.
- A `done` from a session within ten minutes of its resume is held, not applied: the dispatch stays
  as it was, the dispatch log gets a `done-held` line with the evidence and the reason, and the
  controller is answered with its dispatch and told to report again. A `done` the operator then gives
  with `conductor dispatch report-done <id>` closes it as today.
- `conductor dispatch reopen-dispatch <id> --reason <text>` moves a `Done` dispatch back to `Started`,
  for a human actor only, and writes a `reopened` line.
- Spec first (AGENTS.md rule 1): the command `ReopenDispatch` with its transition `Done` to `Started`
  and its refusals, and the `DispatchLogEvent` variants `DoneHeld` and `Reopened`, in
  `spec/domains/dispatch.yaml`; the CLI binding in `crates/conductor/cli.yaml`; then `task regen`.

Decision (class C, controller, 2026-11-18): the hold is tied to a recent resume, not to every `done`
on a `Blocked` dispatch, so `story:dispatch-done-from-blocked` keeps working for a controller that was
not resumed; the window is ten minutes, a constant. Undone by widening the rule.

Decision (class C, controller, 2026-11-18): a controller cannot withdraw its own `done`; the
operator's `reopen-dispatch` undoes a wrong one, and the refusal a controller's later report meets
names that command. Undone by admitting a controller's withdrawal within its cycle later.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_resumed_controller_is_told_its_open_dispatch_first`: the resume's first input
  names the dispatch's id, goal and brief.
- `tests/run.rs` `a_done_right_after_a_resume_is_held_and_leaves_the_dispatch_open`: `started` then
  `done` in the tick after a resume; the dispatch stays `Started`, the log holds `done-held`.
- `tests/dispatch.rs` `reopen_dispatch_moves_a_done_dispatch_back_to_started_for_a_person_only`: an
  agent actor is refused.

## Boundaries

- Comparing a report's evidence with the dispatch's goal by its words is not attempted.
