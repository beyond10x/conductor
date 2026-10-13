---
format: aep.planning-md/3
id: story:operator-items-age
kind: story
status: draft
title: An operator item shows its age and is re-sent with a default and a deadline
relations:
- decomposes: epic:conductor-autonomy
- depends_on: story:claims-cite-source
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 4
---
## Why

Two operator items aged for days: a class-O escalation open since 2026-10-08 and a hands item for a
tool upgrade open since the same day. Under rules item 99 conductor decides what can be undone; what
still goes to the operator needs a default and a deadline so it cannot stall.

## Acceptance

- `conductor decision hands-todo` shows each open operator item's age in whole days; a test in
  `crates/conductor/tests/decision.rs` over a fixture item opened two days before the fixture's now
  reads `2d`.
- `.agents/conductor.md` tells conductor to re-send an operator item older than a day with the
  default it will take and its deadline, and to take that default at the deadline when the step can
  be undone; a test in `crates/conductor/tests/profiles.rs` holds both sentences.

## Notes

- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with the other profile
  stories of `epic:conductor-autonomy`; it depends on `story:claims-cite-source` so it lands after it
  on those files.

## Scope

`crates/conductor/src/decide.rs`, `crates/conductor/tests/decision.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
