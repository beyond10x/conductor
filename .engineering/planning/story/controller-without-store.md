---
format: aep.planning-md/3
id: story:controller-without-store
kind: story
status: draft
title: A controller delivers in a repository with no planning store
relations:
- decomposes: epic:onboarding-from-url
revision: 3
---
## Why

An evaluation run on v0.8.0 (2026-10-13, finding V8-2) used a repository with no AEP planning store.
`.agents/repo-controller.md` assumes one, so the controller stopped to ask instead of working. Many
repositories conductor will be used in have no store.

## What

`.agents/repo-controller.md` gains a path for a repository with no `.engineering/project.yaml`:
- the controller delivers its dispatch without a store: one branch per unit, one integration
  branch, the same full gate and one pull request, with the dispatch's brief as the plan, written
  into the pull request's description;
- it does not create a store on its own; it offers it once in its `started` report
  (`[REPORT <repo> <dispatch>] started; no planning store: aep plan init would add one`) and goes
  on;
- everything else (gates, reports, boundaries) is unchanged.

## Acceptance

- A profile test in `crates/conductor/tests/profiles.rs` holds the no-store paragraph: "no planning
  store", "does not create", and the report line.
- `website/docs/concepts/roles.md` holds "A repository needs no planning store to be dispatched
  to"; the profile test reads the page for it.

## Scope

`.agents/repo-controller.md` (one section), `crates/conductor/tests/profiles.rs`,
`website/docs/concepts/roles.md`.
