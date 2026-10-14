---
format: aep.planning-md/3
id: epic:red-ci-ownership
kind: epic
status: draft
title: Every red pipeline and pull request has an owner and a clock
relations:
- decomposes: initiative:self-improvement
revision: 1
---
## Why

The operator, 2026-10-14: "we need more protocol in handling red CI pipelines or "red" PRs ... it
seems like nobody feels responsible for this". A pull request stayed red on several checks across
cycles; reports named one of them; no session owned the fix.

## Outcome

Every red `main` and red pull request of an Active repository has an owner (its controller,
spawned if none runs), a clock (acknowledged within one watch cycle, fixed or reverted within
`thresholds.red_fix`, decided after `thresholds.red_stale`), and is reported with every red check;
a red `main` holds merges in its repository. The watch and the board enforce it; rules item 125
says it by hand until then.
