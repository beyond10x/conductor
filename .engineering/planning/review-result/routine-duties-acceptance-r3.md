---
format: aep.planning-md/3
id: review-result:routine-duties-acceptance-r3
kind: review-result
status: active
title: Acceptance critic, round 3, epic routine-duties-in-loop
summary: approve; corrects round 2 on the fake substrate's file
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
approve

Round 2 approved `story:failing-start-recreates-box` on a wrong premise: it said the daemon's
substrate fakes live in `crates/conductor/src/daemon/reconcile.rs`. They do not; the fake substrate
the reconcile tests drive is `Fake` in `crates/conductor/tests/placement.rs:136` (its `close` at
`:192`, `keep_boxes` driven at `:2225`). The story now names that file, and its restart case now
expects the new box at the next pass, since `keep_boxes` runs apart from the reconcile pass
(`daemon/reconcile.rs:723`). Each of its three tests reads `boxes.json`, the reconciliation records
and the alert, and fails on main.

The other three stories are as round 2 read them.

```findings
[]
```
