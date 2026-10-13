---
format: aep.planning-md/3
id: story:dashboard-snapshot-api
kind: story
status: archived
title: The dashboard serves the newest complete snapshot read-only
relations:
- decomposes: epic:dashboard-for-engineers
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/github.rs
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: spec/domains/observation.yaml
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-10-13T11:36:28Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":5}}}
---
## Why

Retired after the plan critics' first round on `epic:dashboard-for-engineers`. This story declared
all seven snapshot endpoints in one layer, ahead of the views that read them, so no story showed a
view reading a served endpoint and the seam between them went untested.

## Acceptance

- None: nothing is to be built under this story.

## Notes

- Its endpoints moved into the stories that render them:
  - `/api/board` (the figures `status.rs` computes, shared), `/api/repositories`,
    `/api/pull-requests` (with the new `url` field), `/api/workflow-runs` and `/api/releases` to
    `story:dashboard-overview`;
  - `/api/blockers` and `/api/specifications` to `story:dashboard-repo-view`;
  - the newest snapshot in any state, with collector runs and member failures, to
    `story:dashboard-health` (`/api/health`).

## Scope

None.
