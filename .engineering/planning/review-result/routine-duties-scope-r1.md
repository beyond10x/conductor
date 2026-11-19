---
format: aep.planning-md/3
id: review-result:routine-duties-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, epic routine-duties-in-loop
summary: 'changes: two #207 duties misattributed, alert story reaches beyond #207'
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
- reviews: epic:routine-duties-in-loop
revision: 1
---
changes

Two duties of #207 are claimed by the wrong story or by none, and one story reaches beyond #207.

**Duties of #207, one by one:**
- Wake an idle controller when its request's pipeline turns green or red (#193): the epic names
  `story:merge-wakes-idle-controller`, which relays `PR merged` only. The pipeline duty is
  `story:default-branch-pipeline-wakes-controller` (implemented, wave 57, `main CI after merge`)
  together with the existing `PR checks done` relay (`crates/conductor/src/run/handlers.rs:8`,
  `:132-137`). The epic must name both.
- Free a done controller's place, stop re-sending done dispatches (#203, #204):
  `story:idle-controller-yields-place`, `story:dispatch-v1-once-per-goal`, delivered elsewhere
  (wave 59). Correct.
- Restart gone controllers and send them their next step (#205): the epic names wave 56 but no
  story. The box restart / OOM kill case is `story:controllers-reconciled-with-live-sessions`
  (the cycle's reconcile, `crates/conductor/src/run/dispatch.rs:5-16`, `:236-255`, restarts a
  repository with an open dispatch, reason `gone`); the usage-limit case is
  `story:limit-resume-at-reset` (wave 56). The epic must name them.
- Restart an instance whose start keeps failing on a gone box: `story:failing-start-recreates-box`.
- Merge a unit's request on green when the instance allows it: `story:unit-merge-policy`.
- Route a NEED to a chartless repository: `story:need-starts-unit-of-work`.
- One alert line only when the loop cannot act: `story:loop-alerts-when-it-cannot-act`.

**Reach beyond #207:** `story:loop-alerts-when-it-cannot-act` alerts on "a dispatch waiting past a
bound" and "a red main with no controller", neither a duty #207 lists, and shows alerts on the
dashboard overview and in `conductor status`, which #207 does not ask. Its conditions must be the
cases of #207's duties the loop cannot act on: a unit of work left to the operator after its
failed starts, a box that cannot be recreated, a NEED that names no repository.

**Overlap:** the alert for a failed recreated start is promised by both
`story:failing-start-recreates-box` and `story:loop-alerts-when-it-cannot-act` (the latter lists
"a box that cannot be recreated"). One story must own each condition.

Out of my lane: the alert route "the instance's configured notification route" does not exist
(design critic).

```findings
[
{"file":".engineering/planning/epic/routine-duties-in-loop.md","line":17,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"#193 pipeline wake is delivered by story:default-branch-pipeline-wakes-controller and the PR checks done relay, not by story:merge-wakes-idle-controller alone"},
{"file":".engineering/planning/epic/routine-duties-in-loop.md","line":19,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"#205 restart of gone controllers is named by wave only; name story:controllers-reconciled-with-live-sessions and story:limit-resume-at-reset"},
{"file":".engineering/planning/story/loop-alerts-when-it-cannot-act.md","line":20,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"alert conditions 'dispatch waiting past a bound' and 'red main with no controller' and the dashboard/status display reach beyond #207"},
{"file":".engineering/planning/story/failing-start-recreates-box.md","line":33,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the alert for a box that cannot be recreated is claimed by two stories"}
]
```
