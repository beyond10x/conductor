---
format: aep.planning-md/3
id: story:goal-docs-snapshot
kind: story
status: draft
title: The snapshot records each goal document's source, ref, size and digest
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D6, approved 2026-10-13.

## Acceptance

- A collector records `GoalDocumentObservation` per configured document; a missing or unreadable one is a member failure naming the path; tests.

## Scope

`crates/conductor/src/collect/`, tests.
