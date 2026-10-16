---
format: aep.planning-md/3
id: story:tool-plan-memo-wait
kind: story
status: draft
title: Plan, escalate, team memos with a time to live and waits on CI or an event go through tool
relations:
- decomposes: epic:agent-tools
- depends_on: story:tool-build-commit-report
- informed_by: story:watch-waits
scope:
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/tool
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 4
---
# Story: tool-plan-memo-wait

## Outcome

A session reads and drafts planning artifacts, escalates a question with options, keeps team-scoped notes that vanish after their time to live, and waits on a CI run or a named event without a turn per poll, all through `tool`.

## Context

docs/design/roles-and-surfaces.md § 6.3 (plan, team `escalate`, memory, wait rows), § 7 step 4. The 48 h analysis (docs/design/agent-tools.md § 5) estimates about 400 round trips a day spent in `until … sleep` loops and `.output` reads. Rules forbid such loops today, but nothing replaces them. `tool wait` resolves in the broker and answers once. It registers its wait the way story:watch-waits registers one, so a restart keeps it.

## Domain relations

- Memo → Team, many-to-one; the team owns its memos, and a memo ends at its time to live. Decided in docs/design/roles-and-surfaces.md § 6.3 ("team-scoped scratch that vanishes after its time to live"). No `ess/1` document declares Memo yet; acceptance 1 declares it.
- Escalation → Lead → Director: a worker's escalation goes to its lead, who raises it to the director as a decision request. The director side is `conductor.decision.DecisionRequest` and its `EscalateRequest` command in `spec/domains/decision.yaml` (ess/1); the worker → lead hop is decided in § 6.3.
- Plan move → lead only: decided in § 6.3 ("moves only for the lead").

## Acceptance

A worker drafts a story with `tool plan draft`, writes a memo with `--ttl 1h` that its teammate reads and that is gone after the hour, and returns from `tool wait --ci <pr>` with the run's outcome in one call.

1. **Spec first.** `spec/domains/tool.yaml` gains `Memo` (team, key, text, expires_at) and the inputs and errors of `plan show|draft|move`, `escalate`, `memo put|get|list` and `wait`. Validate and regenerate.
2. **Plan.** `tool plan show <id>` and `tool plan draft <kind> <name> --title t --from-text …` run the repository's `aep plan artifact` in the caller's checkout. `tool plan move` is refused `NotGranted` for any role but the lead.
3. **Escalate.** `tool escalate "<question>" --options a,b[,c]` records a message to the caller's lead. From a lead, it raises a decision request to the director. A question with fewer than two options is refused.
4. **Memo.** `tool memo put <key> <text> [--ttl 4h]`, `tool memo get <key>` and `tool memo list` are scoped to the caller's team. Another team's memo is not visible. An expired memo reads as absent and is removed by the broker.
5. **Wait.** `tool wait --ci <pr> | --event <name> [--deadline d]` blocks in the broker and prints one line with the outcome, or `deadline` when the deadline passes. The wait is registered and outlives a broker restart.
6. Tests: `crates/conductor-broker/tests/{plan_verbs,memo,wait}.rs`. The memo test uses a clock fake to expire a memo and checks team isolation. The wait test resolves on a fake forge, times out on a deadline, and survives a broker restart.

## Scope

- `spec/domains/tool.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (plan, memo, wait modules), `crates/conductor-broker/tests/`
- `crates/tool/src/`
- `crates/conductor/src/decide.rs`, `crates/conductor/src/watch.rs` (reused)

## Out of Scope

- `wait --agent <id>`, which needs workers that are sessions (story:agent-run-grants).
- Memos shared across teams or instances.

## Ambiguities

None.
