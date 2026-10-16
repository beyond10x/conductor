---
format: aep.planning-md/3
id: story:tool-director-verbs
kind: story
status: draft
title: The director works through a tool verb group for its records, decisions, dispatch, metric reads and messages, with Bash denied
relations:
- decomposes: epic:agent-tools
- depends_on: story:agent-run-grants
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/message.rs
- confidence: cited
  path: crates/conductor/src/metric.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: inferred
  path: crates/tool
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 4
---
# Story: tool-director-verbs

## Outcome

The director calls a `tool` verb group for its records, its decisions (raise, answer within its grant, escalate, amend), dispatch, metric reads and messages. It runs with the harness's file tools and `tool mcp` only, Bash denied, so no session of the instance holds the admin binary.

## Context

decision-blocker:director-tool-verbs (cleared 2026-10-16, under the approval of docs/design/roles-and-surfaces.md): the director gets its own verb group; design § 3 lists these as its tools, and § 6.1 keeps the admin binary out of every session.

Measured baseline, one director session on 2026-10-16: 200k context in about 35 minutes over 120 Bash calls. The largest tool results were `conductor` views (50 KB over 15 calls), a loop over the records (34 KB over 40 calls) and `sed` reads of the records (34 KB over 8 calls). Bounded views and a records verb aim at exactly those three sources.

## Domain relations

- DecisionRequest → Decision, and the commands `RaiseDecisionRequest`, `AnswerRequest`, `EscalateRequest`, `RecordConductorDecision`: `spec/domains/decision.yaml` (ess/1). The verbs call these commands and add none.
- Dispatch → Goal, relation `goal`, and Message → Dispatch, relation `opened_dispatch`: `spec/domains/dispatch.yaml` (ess/1).
- Director grant → decision classes: the director answers only within its grant, and anything else escalates to the operator. Decided in docs/design/conductor.md § 5 (decision classes) and the cleared blocker; the grant enforces it.
- Session → Grant, revoked with its parent: story:agent-run-grants. The director's own grant comes from its role config, not from a parent.

## Acceptance

The director runs a full cycle (read status, answer a request, send a dispatch, record a decision, read a KPI) through `tool` with no Bash call. `conductor resource tool-calls --role director` over a window of the same length shows lower context per turn than the baseline above, recorded as evidence beside it.

1. **Spec first.** `spec/domains/tool.yaml` gains the director's verbs and maps each to the existing command it calls. The director role's default grant in `spec/domains/config.yaml` names them. Validate and regenerate.
2. **Records.** `tool records show <file> [--section s] [--max n]` reads the instance's records, bounded, and prints `unchanged` on a re-read. `tool records note <file> "<line>"` appends a timestamped line to a hand-over or status file the director owns. Every other records path is refused.
3. **Decisions.** `tool decision raise|answer|escalate|amend|record|list` call the decision domain's commands. An answer outside the director's grant (a class the grant does not hold) is refused naming the rule, and `escalate` sends it to the operator instead.
4. **Dispatch and messages.** `tool dispatch send|cancel|list` and `tool message list|handle|reject|route` call the dispatch domain's commands. A dispatch to a repository of another instance is refused `ForeignInstance`.
5. **Metrics.** `tool metric show|kpis|readings` read only, bounded, opening the store as a reader (story:views-open-as-reader).
6. **Bash denied.** The director's settings deny Bash and start `tool mcp`. The guard denies a director Bash call, records it, and names the verb that replaces it. Tests: `crates/conductor-broker/tests/director_verbs.rs` (each verb, the grant refusal, the foreign instance), `crates/conductor/tests/guard_rules.rs`, `crates/conductor/tests/profiles.rs`.

## Scope

- `spec/domains/tool.yaml`, `spec/domains/config.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (director module), `crates/conductor-broker/tests/director_verbs.rs`
- `crates/tool/src/`
- `crates/conductor/src/decide.rs`, `crates/conductor/src/dispatch.rs`, `crates/conductor/src/message.rs`, `crates/conductor/src/metric.rs`, `crates/conductor/src/records.rs` (called, not changed in behaviour)
- `crates/conductor/src/guard/rules.rs`, `.claude/conductor-settings.json`, `.agents/conductor.md` (the director's profile after epic:role-names)

## Out of Scope

- Removing the `conductor` commands these replace (story:conductor-drops-session-commands).
- The bridge and the maintainer, which stay on the `conductor` binary (§ 6.1).

## Ambiguities

None.
