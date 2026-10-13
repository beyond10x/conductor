---
format: aep.planning-md/3
id: story:direction-context
kind: story
status: draft
title: '`conductor direction context` renders the Direction block, and dispatch and spawn carry it'
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
- depends_on: story:direction-sources
revision: 1
---
## Why

`docs/design/goal-documents.md` § D6, approved 2026-10-13.

## Acceptance

- Excerpt to the author's `<!-- conductor: end of excerpt -->` marker, else the second section or 4 KB; 8 KB marker cut; 16 KB block cap; kind order; `send-dispatch` records the digests; `spawn` passes the block with the charter; tests over fixture documents.

## Scope

`crates/conductor/src/direction.rs`, `dispatch.rs`, `spawn.rs`, tests.
