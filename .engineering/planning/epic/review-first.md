---
format: aep.planning-md/3
id: epic:review-first
kind: epic
status: draft
title: Conductor paces work to the team's review capacity
revision: 2
---
## Why

Users running many agent sessions in parallel reported on 2026-10-13 that review, not output, is now
the bottleneck: more agents do not ship more work. Some work cannot be handed fully to agents and
needs stricter gates. Conductor today dispatches regardless of how many pull requests already wait
for a person, and a controller merges its own work once its gate is green.

## Outcome

Conductor paces work to the team's review capacity: it shows what waits for a person's review,
oldest first, holds new dispatches to a repository whose review queue is full, and lets a
repository's charter say that only a person merges.

## Stories

- `story:review-limit`: the waiting-for-review list on the board, the dashboard and the watch, and
  the per-repository limit that holds dispatch.
- `story:merge-human`: a charter's `merge: human` makes the controller open the pull request and
  never merge.

## Not covered

Assigning reviewers, or measuring review time per person.
