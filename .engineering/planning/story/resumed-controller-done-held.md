---
format: aep.planning-md/3
id: story:resumed-controller-done-held
kind: story
status: implemented
title: A resumed controller is told its dispatch first, a done right after a resume is held, and a done dispatch can be reopened
summary: A resumed controller gets its open dispatch as first input; a done within ten minutes of a resume or on a blocked dispatch is held; dispatch reopen-dispatch and a controller's withdrawal before the next cycle undo a wrong done
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
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/roles/controller.md
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/message.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests/dispatch.rs
- confidence: cited
  path: crates/conductor/tests/fake_provider/mod.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/scenarios/reopen-dispatch.yaml
- confidence: cited
  path: website/docs/reference
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T21:21:06Z", actor: "human:unknown", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-19T21:21:07Z", actor: "human:unknown", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-19T23:23:19Z", actor: "human:unknown", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #202 (friction F54). After a box restart a controller was resumed from its transcript; within
the same second it reported `started` and then `done` on its open dispatch for goal B with the
evidence of an earlier goal A. The next tick recorded `unblocked` and `done`; the dispatch closed. The
controller's own withdrawal (`unblocked`) was refused, and the work was sent again as a new dispatch.

Read in the code (release 1.17.0):

- A resume continues the transcript and sends no first input: `spawn::resume_leased`
  (`crates/conductor/src/spawn.rs:611-680`) calls `Chosen::resume`
  (`crates/conductor/src/confine/mod.rs:611-626`), whose providers run `claude --bg --resume <id>`
  (`confine/mod.rs:420-431`) or `claude -p --resume <id>` "with no prompt"
  (`crates/conductor/src/confine/child.rs:37`, `:769-779`). A fresh spawn's charter names the
  dispatch (`spawn.rs:881-889`); a resumed session goes back to what its context said last. Every
  resume goes through `resume_leased`: the loop's (`crates/conductor/src/run/handlers.rs:839`, after
  a runtime or box restart `exited`, a usage limit, a withdrawn token) and `controller
  resume-controller` (`crates/conductor/src/controller.rs:219`).
- The mailbox takes a `started` on a `Blocked` dispatch as `unblocked` and a `done` as `dispatch
  report-done` (`crates/conductor/src/run/mailbox.rs:263-345`), which moves `Sent`, `Started` or
  `Blocked` to `Done` on any evidence (`crates/conductor/src/dispatch.rs:981-1006`,
  `spec/domains/dispatch.yaml` `finish`). Nothing compares the report with when the session was
  resumed, or with a block it never reported over.
- `Done` is final: no command reopens it, and `report-unblocked` on it is refused
  `DispatchStateConflict` (`dispatch.rs:880-884`).

## Outcome

- A controller session resumed from its transcript (`spawn::resume_leased`, so by the loop and by
  `controller resume-controller` alike) gets, as its first input, its repository's open dispatch in
  the § 6 form `[DISPATCH <repo> <id>] <goal|-> resumed: ...`, with its goal or reference and its
  brief verbatim in a fenced block, as a fresh spawn's charter names it; several open dispatches
  are each named; none is said as `[REPORT <repo> -] resumed: <repo> has no open dispatch ...`. The
  `none` provider passes it as the last argument of `claude --bg --resume <id> ...`, the `child`
  provider as the first message on the session's FIFO (`Provider::resume` takes it). The resume's
  instant is kept in `<state>/sessions/<session name>.resumed`.
- A controller's `done` through the mailbox is held as a claim, not applied, when the repository's
  controller session was resumed less than ten minutes before (`RESUME_HOLD`), or when the dispatch
  is `Blocked` (no `unblocked` since its block). The dispatch does not move (a `Sent` one is not
  started either), the dispatch log gets a `done-held` line (`id`, `evidence`, `reason`: `resumed
  <at>` or `blocked`), and the controller is answered (`relayed`) with its dispatch (id, goal,
  brief) and what takes a `done` then: after the window, or after `unblocked`. The claim is
  confirmed by `conductor dispatch report-done <id> --evidence ...`, unchanged: the operator's, or
  the loop's decider acting as conductor. `dispatch report-done` itself holds nothing, so
  `story:dispatch-done-from-blocked` stands for the command.
- `conductor dispatch reopen-dispatch --dispatch-id <id> --reason <text>` moves a `Done` dispatch
  back to `Started` and writes a `reopened` line; an agent actor is refused `ReopenByAgent`, any
  other state `DispatchStateConflict`.
- A controller withdraws its own `done` with `[REPORT <repo> <id>] withdrawn <why>`: when the
  mailbox took that `done` after the last cycle started (`<state>/run/done.taken`, beside
  `cycle.last`), the loop reopens the dispatch as itself (`reopened`, reason `withdrawn by the
  controller: <why>`); later, the withdrawal is refused and the answer names `dispatch
  reopen-dispatch`. A `withdrawn` on an open dispatch (a held `done`) moves nothing and is not
  refused.
- Spec first (AGENTS.md rule 1): `ReopenDispatch` with the transition `reopen` (`Done` to
  `Started`), its outcomes and the error `ReopenByAgent`, the event `DispatchReopened`, and the
  `DispatchLogEvent` variants `DoneHeld` and `Reopened`, in `spec/domains/dispatch.yaml`, with the
  scenario `spec/scenarios/reopen-dispatch.yaml`; then `task regen`.

Decision (class C, implementor, 2026-11-19, replaces the two of revision 3 on the controller's
brief for wave 60): the hold has two triggers, a resume within ten minutes and a `Blocked`
dispatch, both only for a controller's report through the mailbox; the window is a constant.
Undone by narrowing the rule to the resume. A controller's withdrawal is accepted until the next
cycle starts, the step that acts on a `Done` dispatch; the operator's `reopen-dispatch` covers any
later one.

## Acceptance (tests that fail on the current code)

- `tests/dispatch.rs` `reopen_dispatch_moves_a_done_dispatch_back_to_started_and_refuses_an_agent`:
  the operator reopens a done dispatch (`reopened` line); an agent actor is refused
  `ReopenByAgent`, an open dispatch `DispatchStateConflict`.
- `tests/run.rs` `a_done_right_after_a_resume_is_held_and_leaves_the_dispatch_open`: the session
  resumed a minute before; `started` then `done`; the dispatch stays `Started`, the log holds
  `done-held` with `resumed`, the controller is answered with the dispatch's id and brief.
- `tests/run.rs` `a_done_on_a_blocked_dispatch_is_held_until_unblocked`.
- `tests/run.rs` `a_withdrawn_done_before_the_next_cycle_reopens_the_dispatch`, and after a cycle
  the withdrawal is refused naming `reopen-dispatch`.
- `tests/spawn.rs` `a_resumed_controller_is_told_its_open_dispatch_first`: the provider's resume
  is given the dispatch's id, goal and brief; `the_none_provider_resumes_with_the_start_form`
  holds the first input as the last argument.

## Boundaries

- Comparing a report's evidence with the dispatch's goal by its words is not attempted.
- A model-driven confirmation of a held claim by the decider is not added; the decider confirms
  through `dispatch report-done` like the operator.
