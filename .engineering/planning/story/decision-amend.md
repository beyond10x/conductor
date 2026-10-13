---
format: aep.planning-md/3
id: story:decision-amend
kind: story
status: draft
title: A conductor decision can be amended and can expire; no slot under build_size
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/src/resource.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
- confidence: cited
  path: crates/conductor/tests/goal_resource.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/decision.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/scenarios/amend-conductor-decision.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor amended two of its decisions after recording them: one 29 s later (it had
told a controller to write the decision id into its repository, against an operator rule), one
39 min later (a build-slot grant made on an estimate, though builds under `thresholds.build_size`
need no slot; a controller reported `blocked disk` until the amendment). The store still shows the
second decision in force with its old choice, because there is no amend command, and a reversal was
written as a hand-made key in the decision log. A time-boxed six-hour grant was recorded with no
expiry.

## Acceptance

- Spec first: `AmendConductorDecision` (decision id, new choice, reason) and an optional
  `valid_until` on a recorded decision are declared in `spec/domains/decision.yaml`; a scenario
  `spec/scenarios/amend-conductor-decision.yaml` amends a decision and reads the new choice and an
  `amended` history entry in the `Decisions` view, and amending an operator's decision is refused
  with `ReservedForOperator`; `ess verify conform run --target interpreted` passes it.
- `conductor decision amend-conductor-decision <id> --choice <c> --reason <r>` changes the choice
  `decision decisions` prints and keeps the earlier choice in `decision show <id>`; a test in
  `crates/conductor/tests/decision.rs` holds it.
- A decision whose `valid_until` has passed prints `expired` in `decision decisions`; a test in
  `crates/conductor/tests/decision.rs` holds it.
- `resource grant-resource` for a `BuildSlot` whose amount is under `thresholds.build_size` is
  refused with `NoSlotNeeded`, declared in `spec/domains/dispatch.yaml` first; a test in
  `crates/conductor/tests/goal_resource.rs` holds it.

## Notes

- After `story:records-from-commands`, which makes the amend command write its log line.
- `story:decision-goal` (draft) also changes the decision record; order the two.

## Scope

`spec/domains/decision.yaml`, `spec/domains/dispatch.yaml`,
`spec/scenarios/amend-conductor-decision.yaml` (new), `crates/conductor-model`,
`crates/conductor/src/decide.rs`, `crates/conductor/src/resource.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/tests/decision.rs`,
`crates/conductor/tests/goal_resource.rs`, `docs/design/conductor.md`,
`website/docs/reference/cli.md`.
