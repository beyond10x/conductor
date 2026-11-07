---
format: aep.planning-md/3
id: story:forge-route-per-instance-with-gitlab
kind: story
status: draft
title: The gates forge route works for a GitLab instance
summary: controllers on a GitLab instance ended 4 of 4 waves at MR not opened
revision: 1
---
## Why

From the host teacher's brief, 2026-11-07, wave B: on one instance whose repositories are on a
self-hosted GitLab, the controllers ended 4 of 4 waves at "MR not opened": the bot route
(`forge.route: gates`) writes only to GitHub. `story:forge-route` adds a `direct` route (gh or glab
on the session's own login); this story makes the `gates` route itself work for GitLab.

## Acceptance

- As written in the brief: `forge.route: gates` works for a GitLab instance (bot route to
  `gitlab.<host>`, policy from `gates_policy`).
- The guard admits the bot route's merge-request writes (create, update, note, merge) of the
  session's own GitLab project under `gates`, and denies the same writes for another project; a
  test in `crates/conductor/tests/guard_rules.rs` holds both.
- What the bot route itself must support for GitLab (the Gates tool's own release) is named in the
  story's notes when it is found missing; it is not built here.
- Switching the route on for a given instance is the operator's call (a credential); this story
  builds the feature only.
