---
format: aep.planning-md/3
id: review-result:routine-duties-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, epic routine-duties-in-loop
summary: 'changes: three stories hold untestable or wrong-surface acceptance lines'
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
changes

Each acceptance line was read for a test that would hold it and would fail on main.

**story:unit-merge-policy:** testable. "A test fails on main: a unit charter without `merge: human`"
fails on main (the key is unknown and the charter is the human form). Missing: a line for the
rewrite of an already written default charter (design finding), which is the line #207's running
instances need; and "the merge steps of the controller profile" is not checkable as written.

**story:failing-start-recreates-box:** "after N failed starts in a row ... whose runtime log or
substrate answer names a missing resource" is not testable as written (N unnamed, "log or
answer" two sources) and tests the wrong path. "The instance reads `failed` ... in `conductor
doctor`" names a state that does not exist. Needs: a fake substrate whose close (and restart)
answers the not-found name; the pass forgets the stored box and opens a new one, one `Start`
recorded `Succeeded`; a fake whose open then also fails gives one alert and the existing row of
failed starts. Both fail on main (main records `Refused` every pass and opens nothing).

**story:need-starts-unit-of-work:** "once per request id" is not testable as stated (request ids
collide across senders). The reference form is refused by the spec. Needs: the reference form
named exactly, the message `Handled` with the dispatch id (`RouteNeed`), a second identical NEED
opening nothing.

**story:loop-alerts-when-it-cannot-act:** "one alert kind per condition" without the conditions
listed is not testable; "shown on the dashboard's overview and by `conductor status`" names no
surface the test reads; "the instance's configured notification route" does not exist, so no
test can configure it. The failing test (a refused start over three cycles) is good once the
condition is named exactly (`FAILED_STARTS_MOST`, `crates/conductor/src/dispatch.rs:325`).

```findings
[
{"file":".engineering/planning/story/unit-merge-policy.md","line":30,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"'the merge steps of the controller profile' is not checkable; no acceptance for rewriting an existing default charter"},
{"file":".engineering/planning/story/failing-start-recreates-box.md","line":28,"category":"judgement","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"N unnamed, two evidence sources, wrong path, and a non-existent failed state: not testable as written"},
{"file":".engineering/planning/story/need-starts-unit-of-work.md","line":27,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"once per request id is ambiguous across senders and the reference form is invalid"},
{"file":".engineering/planning/story/loop-alerts-when-it-cannot-act.md","line":24,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"conditions not enumerated; dashboard/status display and configured route name no testable surface"}
]
```
