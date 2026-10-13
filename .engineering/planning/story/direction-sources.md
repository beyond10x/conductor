---
format: aep.planning-md/3
id: story:direction-sources
kind: story
status: draft
title: Direction sources are fetched into the cache and read at their ref
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D3, approved 2026-10-13.

## Acceptance

- Each source is fetched prompt-free into `<cache>/direction/<name>` on every snapshot and when older than 10 minutes on a read; documents are read from the fetched ref, never a working tree; a pinned tag never moves; a failed fetch keeps the last text, marked stale with its age; tests with a local bare repository.

## Scope

`crates/conductor/src/direction.rs` (new), tests.
