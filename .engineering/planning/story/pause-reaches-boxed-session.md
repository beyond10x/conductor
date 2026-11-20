---
format: aep.planning-md/3
id: story:pause-reaches-boxed-session
kind: story
status: active
title: pause-controller on the host reaches a session in the box through its runtime
summary: Run on the host for a session in the box, pause-controller finds no live session, stops nothing and records Paused while the session runs on; stop-controller already goes through the runtime
scope:
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/dashboard/operator.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T04:41:03Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-20T04:41:04Z", actor: "human:unknown", revision: 5}
---
## Problem

`controller pause-controller` (crates/conductor/src/controller.rs:162-194) stops the session through
`crate::spawn::end` on the side it runs on (:172) and never asks `runs_in_box`. Run on the host for
a controller whose session runs in the instance's box, it finds no live session there, stops
nothing and records the controller `Paused` while the session runs on. `stop-controller`
(:267-317) asks `runs_in_box` (:285) and sends the command to the box's runtime through
`through_daemon`, `controllers/{id}/stop` (:286-304); its unreached arm refuses before any record
is written.

## Outcome

On the host, `pause-controller` for a session in the box goes to the runtime as `stop-controller`
does: a new runtime route `POST controllers/{id}/pause` (no body), served by a
`pause_here(state, id, actor, providers)` split out of `pause_controller` as `stop_here` is
(controller.rs:333), and proxied as a write: the route table in
crates/conductor/src/dashboard/operator.rs:57 (and its module doc :1-10), `writes()` in
crates/conductor/src/daemon/api.rs:2638 (and its doc :31-33). `grant_for` (api.rs:739-754) puts it
under ChangeSettings, as it does stop; that is intended. A runtime that does not answer leaves the
record `Running` and exits 1 naming it.

## Acceptance (fails on main)

- tests/box_session.rs: from `Side::Host`, `pause-controller` ends the boxed session's pid (`gone`),
  the record is `Paused`, and the output is `stopped <session>`.
- With conductord down (as stop's unreached case): exit 1, the record stays `Running`, the session
  runs on.

## Boundaries

The pause path only; resume refuses outside the box since `story:resume-starts-gone-session`. The
route is hand-written, as stop's is: neither `crates/conductor/http.yaml` nor
`tests/fixtures/api_parity.md` holds `controllers/*/stop`, so no spec or binding change.

## Scope

crates/conductor/src/controller.rs, crates/conductor/src/dashboard/operator.rs,
crates/conductor/src/daemon/api.rs, crates/conductor/tests/box_session.rs.
