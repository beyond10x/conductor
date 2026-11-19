---
format: aep.planning-md/3
id: epic:routine-duties-in-loop
kind: epic
status: active
title: Conductor's loop carries the operator's routine duties
summary: Wake, free places, restart, merge units and route NEEDs in conductor's own loop, with one alert when it cannot act
refs:
- provider: github
  reference: beyond10x/conductor#207
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T19:25:01Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-19T19:25:01Z", actor: "human:unknown", revision: 4}
---
## Why

Issue #207: an operator session outside conductor carried the routine duties that kept three boxed
instances moving; when it went quiet for 19 hours everything stalled. The duties: wake an idle
controller on its request's pipeline (#193), free a done controller's place and stop re-sending
done dispatches (#203, #204), restart gone controllers and send them their next step (#205),
restart an instance whose start keeps failing on a box substrate no longer holds, merge a unit of
work's request on green when the instance allows it, and route a NEED to a repository with no
charter.

## Outcome

Conductor's own loop (watch pass, cycle, daemon reconcile) carries every one of those duties, each
action recorded, and the operator hears one alert line only when the loop cannot act.

## Stories

New in this epic:

- `story:loop-alerts-when-it-cannot-act`: the one alert route (`crate::alert`, an `alert` line in
  the records' dispatches log, the operator's notifier) and its first condition, a unit of work
  left to the operator after its failed starts.
- `story:unit-merge-policy`: `controllers.unit_merge` chooses who merges a unit of work's request.
- `story:failing-start-recreates-box`: a stored box substrate no longer holds is forgotten and a new
  one opened; a recreated start that fails alerts.
- `story:need-starts-unit-of-work`: a NEED to a chartless repository becomes a unit of work; a NEED
  that names no repository alerts.

Delivered elsewhere:

- Wake on the request's pipeline (#193): the `PR checks done` and `PR merged` relays
  (`story:merge-wakes-idle-controller`) and `story:default-branch-pipeline-wakes-controller`
  (wave 57).
- Free a done controller's place, one dispatch per goal (#203, #204):
  `story:idle-controller-yields-place`, `story:dispatch-v1-once-per-goal` (wave 59).
- Restart gone controllers with their next step (#205): the cycle's reconcile
  (`story:controllers-reconciled-with-live-sessions`) and `story:limit-resume-at-reset` (wave 56).

## Order

`story:loop-alerts-when-it-cannot-act` first: it adds the alert route the other three call, and it
and `story:unit-merge-policy` both regenerate `crates/conductor-model`. The other three then run in
one wave.
