---
format: aep.planning-md/3
id: epic:routine-duties-in-loop
kind: epic
status: draft
title: Conductor's loop carries the operator's routine duties
summary: Wake, free places, restart, merge units and route NEEDs in conductor's own loop, with one alert when it cannot act
refs:
- provider: github
  reference: beyond10x/conductor#207
revision: 1
---
## Why

Issue #207: an operator session outside conductor carried the routine duties that kept three boxed
instances moving; when it went quiet for 19 hours everything stalled. The duties: wake an idle
controller on its merge request's pipeline (#193, delivered in wave 57), free a done controller's
place and stop re-sending done dispatches (#203, #204, wave 59), restart gone controllers (#205,
wave 56), restart an instance whose start keeps failing, merge a unit of work's request on green
when the instance allows it, and route a NEED to a repository with no charter.

## Outcome

Conductor's own loop (watch pass, cycle, daemon reconcile) carries every one of those duties, each
action recorded, and the operator hears one alert line only when the loop cannot act.

## Stories

`story:unit-merge-policy`, `story:failing-start-recreates-box`, `story:need-starts-unit-of-work`,
`story:loop-alerts-when-it-cannot-act`; delivered elsewhere: `story:merge-wakes-idle-controller`
(#193), `story:idle-controller-yields-place` (#203), `story:dispatch-v1-once-per-goal` (#204).
