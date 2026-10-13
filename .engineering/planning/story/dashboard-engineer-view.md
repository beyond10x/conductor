---
format: aep.planning-md/3
id: story:dashboard-engineer-view
kind: story
status: draft
title: An engineer's view leaves out the operator's items
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:dashboard-health
- depends_on: story:session-forge-credentials
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: spec/domains/observation.yaml
revision: 2
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13); mockup: the canvas "Conductor dashboard redesign".

## Acceptance

- `conductor dashboard serve --view engineer` (spec first, `DashboardServe` gains `view`) serves no operator-only data: no `awaiting_operator`, no decisions without a repository, no other repositories' briefs, transcripts or spend; a test reads `/data.json` and every `/api/*` endpoint under the engineer view and finds none of those fields.

## Notes

- Spec first wherever a command, endpoint or record changes.
- After `story:dashboard-health` (shares the app and `dashboard.rs`).

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/cli.yaml`, `crates/conductor/src/dashboard.rs`, `crates/conductor/tests/dashboard.rs`.
