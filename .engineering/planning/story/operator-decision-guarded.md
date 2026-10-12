---
format: aep.planning-md/3
id: story:operator-decision-guarded
kind: story
status: draft
title: An operator decision says which session recorded it and on whose words
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: website/docs/security/overview.md
revision: 4
---
## Why

`record-conductor-decision` refuses class O and class H, but the store tells an operator's decision
from conductor's by the command used, not by who runs it: conductor's own session may run
`conductor decision record-operator-decision` and record a class-O or class-H decision as the
operator's. The class boundary of design § 5 is then held only by the profile. Found by the docs
overhaul's security review, 2026-10-12 (`website/docs/security/overview.md`, `audit-trail.md`).

## Acceptance

A plain denial is not the fix: the operator often answers inside conductor's own session, and
conductor records that answer with `record-operator-decision`.

- An operator decision recorded from a guarded session (conductor or a controller) carries the
  recording session and the operator's quoted words as required evidence; the command refuses it
  without them, and a controller session is denied the command outright. Tests in
  `tests/decision.rs` and `tests/guard_rules.rs`.
- `decision show` and the dashboard show who recorded an operator decision.
- The website's security pages state the boundary as built.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`,
`website/docs/security/overview.md`.
