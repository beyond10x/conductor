---
format: aep.planning-md/3
id: review-result:routine-duties-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, epic routine-duties-in-loop
summary: approve
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
- reviews: epic:routine-duties-in-loop
revision: 1
---
approve

Every round-1 finding is resolved; every duty of #207 is claimed by exactly one story of the epic
or by a story the epic names as delivered elsewhere.

- Wake on the request's pipeline (#193): delivered elsewhere, now named correctly: the `PR checks
  done` and `PR merged` relays (`story:merge-wakes-idle-controller`,
  `crates/conductor/src/run/handlers.rs:8`) and `story:default-branch-pipeline-wakes-controller`.
- Free a place, one dispatch per goal (#203, #204): `story:idle-controller-yields-place`,
  `story:dispatch-v1-once-per-goal`.
- Restart gone controllers with their next step (#205): now named,
  `story:controllers-reconciled-with-live-sessions` (`run/dispatch.rs:5-16`, `:236-255`) and
  `story:limit-resume-at-reset`.
- Instance start failing on a gone box: `story:failing-start-recreates-box`, which owns its alert
  (`box-not-recreated`); the alert story no longer claims it.
- Merge policy: `story:unit-merge-policy`.
- NEED to a chartless repository: `story:need-starts-unit-of-work`, which owns its alert
  (`need-unroutable`).
- One alert only when the loop cannot act: `story:loop-alerts-when-it-cannot-act` owns the route
  and the condition `unit-left-to-operator`; its dashboard and status display, "dispatch waiting
  past a bound" and "red main with no controller" are gone.

Nothing reaches beyond #207. All four stories `decomposes` the epic.

```findings
[]
```
