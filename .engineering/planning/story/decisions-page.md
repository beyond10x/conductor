---
format: aep.planning-md/3
id: story:decisions-page
kind: story
status: implemented
title: A top-level Decisions page shows every open O and H item with its options, and the history
summary: Read side of the Decisions section
relations:
- decomposes: epic:dashboard-decisions
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T01:54:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T01:54:07Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T01:54:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator asked for the decisions as a top-level section of the dashboard (epic:dashboard-decisions).

## Acceptance

- The dashboard's navigation has a top-level Decisions entry; its page lists every open class-O and class-H request with its question, lettered options with their cost, the recommendation, the default and the deadline, oldest first, and the decision history from the records' `decisions/` log below, newest first; a Vitest test and a dashboard API test hold both.
- Each request has a kind, modelled in `spec/` on `decision escalate-request`: `yes-no` (two
  buttons, Yes and No), `one-of` (lettered options, one is chosen) or `several-of` (lettered
  options, any number chosen, at least one); the page renders each kind with its own control and
  marks the recommendation and the default; a Vitest test per kind (the operator, 2026-10-14
  ~23:2xZ: "either binary (yes/no) or multi-options (multis-select) some times").
- What an escalation lacks for this (options, costs, recommendation, default, deadline) is modelled in `spec/` first on `decision escalate-request`, validated, regenerated, and required by the command for class O and optional for class H.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model`, `crates/conductor/src/dashboard.rs`, `crates/conductor/dashboard/`, tests.
