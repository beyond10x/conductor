---
format: aep.planning-md/3
id: story:operator-decision-guarded
kind: story
status: implemented
title: An operator decision says which session recorded it and on whose words
relations:
- decomposes: epic:operating-loop-hardening
- depends_on: story:instance-scoped-tasks
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
- confidence: cited
  path: spec/domains/decision.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T18:50:36Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-12T18:50:36Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-12T19:44:37Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

`record-conductor-decision` refuses class O and class H, but the store tells an operator's decision
from conductor's by the command used, not by who runs it: conductor's own session may run
`conductor decision record-operator-decision` and record a class-O or class-H decision as the
operator's. The class boundary of design § 5 is then held only by the profile. Found by the docs
overhaul's security review, 2026-10-12 (`website/docs/security/overview.md`, `audit-trail.md`).

A plain denial is not the fix: the operator often answers inside conductor's own session, and
conductor records that answer with `record-operator-decision`. What is missing is a record of who
wrote the decision down and on what words.

A controller is already denied the command: `("decision", "record-operator-decision")` is not a
read leaf, and `crates/conductor/tests/guard_rules.rs:1727` holds the case. This story does not
change the guard.

## Design

The command takes no session from its environment today: `RecordOperatorDecisionArgs`
(`crates/conductor/src/cli.rs:1557`) has no session field and `decide::record_operator_decision`
(`crates/conductor/src/decide.rs:239`) reads none; only the guard sees a session id, from the hook
payload (`crates/conductor/src/guard.rs:229`). So the recording session arrives as an explicit
`--recorded-by <session id>` flag (and the same field under `--input-json`).

- Spec first (`ess:specifying`): `spec/domains/decision.yaml` gains the recording session and the
  operator's quoted words on `RecordOperatorDecision` and on the decision it records, both
  required; `crates/conductor-model` is regenerated.
- The command refuses an operator decision without either field.
- `conductor decision show` (`decide.rs`, `show`) prints both.

## Acceptance

- An operator decision recorded with `conductor decision record-operator-decision` stores the
  recording session (`--recorded-by`) and the operator's quoted words as required evidence, and
  `conductor decision show` prints both; a test in `crates/conductor/tests/decision.rs` holds it.

## Notes

- The dashboard could show who recorded an operator decision (`crates/conductor/src/dashboard.rs`);
  a later story.
- The website's security pages (`website/docs/security/overview.md`, `audit-trail.md`) should
  state the boundary as built once this lands; a later docs edit.
- Ordered before story:single-conductor-lease, which also changes `spec/domains/`,
  `crates/conductor-model` and `crates/conductor/src/cli.rs`.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model`, `crates/conductor/src/decide.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/tests/decision.rs`.
