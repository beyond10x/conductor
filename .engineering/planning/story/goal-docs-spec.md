---
format: aep.planning-md/3
id: story:goal-docs-spec
kind: story
status: draft
title: The direction config, goal-document observation and direction context are specified first
relations:
- decomposes: epic:goal-documents
revision: 1
---
## Why

`docs/design/goal-documents.md` § D2, D5, D6, approved 2026-10-13.

## Acceptance

- `spec/domains/config.yaml` (Direction, DirectionSource, GoalDocument, DocumentKind with principles; `repositories[].documents`; `activity` optional), `spec/domains/observation.yaml` (GoalDocumentObservation), the `direction context` view; `ess specify validate --path spec` exits 0.

## Scope

`spec/`, `crates/conductor/cli.yaml`.
