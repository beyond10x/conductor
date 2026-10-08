---
format: aep.planning-md/3
id: story:conformance
kind: story
status: draft
title: The conformance suite runs against the CLI
revision: 1
---
# The conformance suite runs against the CLI

## What

The suite from `ess verify conform synthesize --path spec` runs against the `conductor`
implementation through a test target in `crates/conductor/tests/conformance.rs`. It needs every
command group implemented, so it depends on:
- `story:snapshot-driver`;
- `story:controller-commands`;
- `story:goal-and-resource-commands`;
- `story:decision-commands`;
- `story:repository-marks`.

Two `ess-scenario/1` scenarios are authored in `spec/scenarios/` and listed in
`spec/ess-inputs.yaml` `scenarios:`, for the two remaining `ESS-SYNTH-008` refusals:
- `AnswerRequest/wrong-state`: an answered request answered again;
- `AnswerEscalatedRequest/wrong-state`: an answered escalated request answered again.

The report is recorded on `executable-system-specification:conductor` through
`aep plan artifact evidence --from <report>`.

## Scope

- `crates/conductor/tests/conformance.rs`
- `spec/ess-inputs.yaml`
- `spec/scenarios/`

## Acceptance

- The suite runs with 0 refusals and every scenario passing.
- A planted defect makes it fail: removing the `reserved` outcome check in the class-O path of
  `crates/conductor/src/decide.rs`, applied in a scratch copy, not committed.
- The specification artifact moves to `conforming`, decided by the store.
