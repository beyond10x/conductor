---
format: aep.planning-md/3
id: story:health-loop
kind: story
status: draft
title: conductor-dev runs the health loop daily and proves each fix by its number
relations:
- decomposes: epic:controller-health
- depends_on: story:health-command
revision: 1
---
## Why

Self-improvement happens when someone asks. The operator, 2026-10-14: make it "a goal to act on".

## Acceptance

- `.agents/conductor-dev.md` gains a step: once a day and at session start, run `conductor
  health --compare`, act on the largest finding with a citable cause (raise a story already in the
  store, change `rules.md`, file a story, or report a single session to conductor), and record the
  finding, its numbers and the action in the hand-over; a fix's story records its before and after
  from two reports, and a fix whose number did not move is not implemented. A test in
  `crates/conductor/tests/profiles.rs` holds the step.
- `.agents/conductor.md` says conductor answers a health finding about a session (spinning,
  context over the threshold, red CI unowned) in its next cycle; a profile test holds it.

## Scope

`.agents/conductor-dev.md`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
