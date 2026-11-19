---
format: aep.planning-md/3
id: story:loop-alerts-when-it-cannot-act
kind: story
status: draft
title: The loop alerts the operator once when it cannot act
summary: One deduplicated alert line per stuck condition, recorded and routed
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
revision: 1
---
## Why

When the loop cannot act (a start refused for good, a box that cannot be recreated, a dispatch
waiting past a bound, a red main with no controller), nothing tells the operator; an operator
session found these by reading logs (#207).

## Outcome

The loop writes one alert line per stuck condition to the operator's route, and repeats it only
when the condition changes or a bound passes.

## Acceptance

- One alert kind per condition, deduplicated by condition and subject, recorded in the records'
  decisions log and shown on the dashboard's overview and by `conductor status`.
- The alert goes to the operator through the instance's configured notification route; with none
  configured, it is only recorded.
- A test fails on main: a refused start repeated over three cycles writes no alert.

## Boundaries

No alert for a condition the loop acts on itself.
