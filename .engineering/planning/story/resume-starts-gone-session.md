---
format: aep.planning-md/3
id: story:resume-starts-gone-session
kind: story
status: active
title: resume-controller reports a session only once it lives
summary: A resume refuses outside the box and answers only once the resumed session is live
refs:
- provider: github
  reference: beyond10x/conductor#226
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: crates/conductor/tests/run_handover.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T03:08:41Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-20T03:08:42Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #226. A paused controller whose session had died (its box restarted) was resumed with
`controller resume-controller`; the command printed `resumed <session> (<id>)` and recorded the
controller `Running`, but no process ran, and `session deliver` refused `SessionNotReachable`.

## Problem

`controller.rs:208` (`resume_controller`) records `Running` once `spawn::resume` answers, and
`spawn::resume_leased` (`spawn.rs:621`) answers once the provider's `resume` returned; no layer
checks that the resumed session lives:

- From the host, a boxed instance's resume is not refused `OutsideBox` the way `spawn` is
  (`spawn.rs:167`): the host's session list holds none of the box's sessions (`spawn.rs:639`), so
  the provider resumes outside the box, where the transcript is not; under the `none` provider
  `claude --bg --resume` returns at once (`confine/mod.rs:444`) and the command reports `resumed`.
- The `child` provider (`confine/child.rs` `run`) opens the session's output in append mode
  (`child.rs:525`) and counts the session started once that file holds an `init` line of its id
  (`child.rs:611`): a resume finds the `init` line the earlier run wrote there, so a resume that
  ends at once still counts as started. A session that died unstopped (box restart) and was then
  paused leaves that file behind.
- The `none` provider takes `claude --bg --resume`'s exit 0 as the session started, without
  reading the session list.

## Outcome

- `spawn::resume_leased` refuses a boxed instance's resume from outside its box, `OutsideBox`, with
  `spawn`'s detail, before anything is called; the controller stays `Paused`.
- A provider's `resume` answers only once the resumed session is live: the `child` provider counts
  only output written after the start and the session must still run after its grace; the `none`
  provider waits for the session list to show the session with a pid.
- `resumed <session>` is printed and `Running` recorded only after that; else exit 1 naming why,
  the controller stays `Paused`.

## Acceptance (fail on main)

- `tests/box_session.rs`: a resume whose harness ends at once, of a session that died unstopped and
  was then paused, exits 1 naming the end and leaves the controller `Paused`.
- `tests/box_session.rs`: from the host, a boxed instance's resume exits 1 `OutsideBox`, calls no
  harness, and leaves the controller `Paused`.
- `tests/spawn.rs`: the `none` provider's resume whose session the list never shows exits 1 and
  leaves the controller `Paused` (needs CI where Unix sockets are refused).
- Existing resume tests keep passing.

## Boundaries

- The spec's `ResumeController` is unchanged: the session start is outside the record, as for
  pause and stop, whose session errors are the handler's.
- A controller whose session no measurement records (a record-only start) is still resumed as a
  record.
- `pause-controller` from the host of a boxed session is not routed to the runtime here.
