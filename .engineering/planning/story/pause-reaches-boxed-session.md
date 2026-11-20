---
format: aep.planning-md/3
id: story:pause-reaches-boxed-session
kind: story
status: draft
title: pause-controller on the host reaches a session in the box through its runtime
summary: Run on the host for a session in the box, pause-controller finds no live session, stops nothing and records Paused while the session runs on; stop-controller already goes through the runtime
revision: 2
---
## Problem

`controller pause-controller` (crates/conductor/src/controller.rs `pause_controller`) stops the
session through `crate::spawn::end` on the side it runs on. Run on the host for a controller whose
session runs in the instance's box, it finds no live session there, stops nothing and records the
controller `Paused` while the session runs on. `stop-controller` sends the whole command to the box's
runtime through conductord for such a session (`story:host-commands-reach-boxed-session`);
`pause-controller` does not. This is likely how the controller of issue #226 came to be paused
with a live session that a later box restart ended.

## Outcome

On the host, `pause-controller` for a session in the box goes to the runtime as `stop-controller`
does; a runtime that does not answer leaves the record `Running` and exits 1 naming it.

## Acceptance (fails on main)

- Host, boxed instance, live session in the box: `pause-controller` stops it and records `Paused`.
- Runtime not reached: exit 1, the record stays `Running`, the session runs on.

## Boundaries

The pause path only; resume refuses outside the box since `story:resume-starts-gone-session`.
