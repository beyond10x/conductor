---
format: aep.planning-md/3
id: story:decisions-page
kind: story
status: draft
title: A top-level Decisions page shows every open O and H item with its options, and the history
summary: Read side of the Decisions section
relations:
- decomposes: epic:dashboard-decisions
revision: 1
---
## Why

The operator asked for the decisions as a top-level section of the dashboard (epic:dashboard-decisions).

## Acceptance

- The dashboard's navigation has a top-level Decisions entry; its page lists every open class-O and class-H request with its question, lettered options with their cost, the recommendation, the default and the deadline, oldest first, and the decision history from the records' `decisions/` log below, newest first; a Vitest test and a dashboard API test hold both.
- What an escalation lacks for this (options, costs, recommendation, default, deadline) is modelled in `spec/` first on `decision escalate-request`, validated, regenerated, and required by the command for class O and optional for class H.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model`, `crates/conductor/src/dashboard.rs`, `crates/conductor/dashboard/`, tests.
