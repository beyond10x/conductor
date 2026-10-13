---
format: aep.planning-md/3
id: story:goal-docs-validation
kind: story
status: draft
title: Doctor, the board and the watch report missing, stale and unconfigured goal documents
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D5, approved 2026-10-13.

## Acceptance

- Each row of § D5's table shows where the design says, with its severity; tests per row.

## Scope

`crates/conductor/src/doctor.rs`, `status.rs`, `watch.rs`, tests.
