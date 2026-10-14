---
format: aep.planning-md/3
id: initiative:self-improvement
kind: initiative
status: draft
title: Conductor improves itself from what it measures, every day
revision: 1
---
## Why

The operator, 2026-10-14: "how efficient is everything going right now? create yourself a skill you
can periodically use for recursive self improvement", then "make self-improvement a goal to act
on". The first measurement (`docs/analysis/efficiency-baseline.md`) found 12 sessions reading 164M
input tokens in three hours for 3 merged pull requests and 9 commits on `main`, held up by a build
queue, oversized contexts and unowned red CI.

## Outcome

Conductor measures how much merged work each token buys, acts on the largest measured waste every
day, and proves each fix by the number it moved. The instance goal (conductor's goal list) holds
the targets; the epics below build the instruments and the fixes.
