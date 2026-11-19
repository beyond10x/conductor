---
format: aep.planning-md/3
id: review-result:routine-duties-parallel-r3
kind: review-result
status: active
title: Parallel-safety critic, round 3, epic routine-duties-in-loop
summary: approve; tests/placement.rs added to one story's scope
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
approve

After round 2, `story:failing-start-recreates-box` added `crates/conductor/tests/placement.rs` to
its scope (cited). No other story of the set touches it, so the grouping of round 2 holds:

- Wave A: `story:loop-alerts-when-it-cannot-act` alone.
- Wave B, at once: `story:unit-merge-policy`, `story:failing-start-recreates-box`,
  `story:need-starts-unit-of-work`, each `depends_on` the alert story.

`tests/placement.rs` binds Unix sockets in some of its tests; the story's tests must not, so they
run in the sandbox the units build in. Outside the set, as in round 2: the alert story shares
`run/dispatch.rs` and `tests/run.rs` with this wave's `story:idle-controller-yields-place` and
`story:dispatch-v1-once-per-goal`.

```findings
[]
```
