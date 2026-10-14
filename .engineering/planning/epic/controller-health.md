---
format: aep.planning-md/3
id: epic:controller-health
kind: epic
status: draft
title: The instance measures its own efficiency and acts on the largest waste
relations:
- decomposes: initiative:self-improvement
revision: 1
---
## Why

The controller-health probe exists only as a script in conductor-dev's own skill; the instance
cannot see its efficiency, conductor cannot act on it, and nothing proves a fix moved a number.

## Outcome

`conductor health` measures every session's tokens, cache use, cold wakes, failed calls, context
size and limit stops, each repository's delivery and red CI, and the host build queue; reports are
kept and compared; the dashboard shows the efficiency KPIs; conductor-dev's daily loop acts on the
largest finding and records before and after.
