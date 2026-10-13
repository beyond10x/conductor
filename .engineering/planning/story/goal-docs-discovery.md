---
format: aep.planning-md/3
id: story:goal-docs-discovery
kind: story
status: draft
title: Setup discovers goal documents in repositories and direction sources
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D4, approved 2026-10-13.

## Acceptance

- `conductor setup` and `conductor config discover-documents [--apply]` find the names of § D4 (case-insensitive, `-`/`_` alike, `.md`), add them to the right rule or to `direction.documents`, never remove a person's entry; tests over fixture repositories.

## Scope

`crates/conductor/src/setup.rs`, `crates/conductor/src/direction.rs`, tests.
