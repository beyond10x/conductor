---
format: aep.planning-md/3
id: story:dashboard-export
kind: story
status: draft
title: A static export of the dashboard can be shared with the team
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:dashboard-engineer-view
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
- confidence: cited
  path: website/docs/guides/read-the-dashboard.md
revision: 2
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13); mockup: the canvas "Conductor dashboard redesign".

## Acceptance

- `conductor dashboard export --out <dir> [--view engineer]` (spec first) writes the app and a snapshot of every endpoint's JSON into a directory a static host (for example GitLab Pages) serves read-only; a test exports a fixture instance and finds the files and no operator-only field under the engineer view.

## Notes

- Spec first wherever a command, endpoint or record changes.
- After `story:dashboard-engineer-view` (shares the app and `dashboard.rs`).

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/cli.yaml`, `crates/conductor/src/dashboard.rs`, `crates/conductor/tests/dashboard.rs`, `website/docs/guides/read-the-dashboard.md`.
