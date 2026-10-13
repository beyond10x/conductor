---
format: aep.planning-md/3
id: story:operator-items-age
kind: story
status: draft
title: An operator item shows its age and is re-sent with a default and a deadline
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
revision: 2
---
## Why

Two operator items aged for days: a class-O escalation open since 2026-10-08 and a hands item for a
tool upgrade open since the same day. Under rules item 99 conductor decides what can be undone; what
still goes to the operator needs a default and a deadline so it cannot stall.

## Acceptance

- `conductor decision hands-todo` and the daily brief show each open operator item's age, and
  conductor's profile tells it to re-send an item older than a day with the default it will take
  and its deadline, and to take the default at the deadline when the step can be undone; a test
  over the view's output holds the age column.

## Scope

`crates/conductor/src/decide.rs`, `crates/conductor/tests/decision.rs`, `.agents/conductor.md`.
