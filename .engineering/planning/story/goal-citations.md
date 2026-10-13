---
format: aep.planning-md/3
id: story:goal-citations
kind: story
status: draft
title: Goals cite documents by anchor id or heading, and a broken citation is a deviation
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D1, approved 2026-10-13.

## Acceptance

- Anchors `{#ID}` and `<a id="ID"></a>` are read; `goal propose-goal --source <doc>#<anchor|heading>`; the board flags a citation whose anchor or heading is gone, a duplicate anchor, and NORTHSTAR.md citing no organisation document; tests.

## Scope

`crates/conductor/src/goal.rs`, `direction.rs`, `status.rs`, spec (the goal's source), tests.
