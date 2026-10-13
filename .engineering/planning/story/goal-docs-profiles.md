---
format: aep.planning-md/3
id: story:goal-docs-profiles
kind: story
status: draft
title: Conductor appends the Direction block; controllers name the goal an epic serves
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
- depends_on: story:direction-context
revision: 1
---
## Why

`docs/design/goal-documents.md` § D6, approved 2026-10-13.

## Acceptance

- `.agents/conductor.md`: every brief carries `direction context`; `.agents/repo-controller.md`: an epic's body cites the goal it serves, and the scope critic reads the block; profile tests.

## Scope

`.agents/*.md`, `crates/conductor/tests/profiles.rs`.
