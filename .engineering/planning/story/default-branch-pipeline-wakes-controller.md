---
format: aep.planning-md/3
id: story:default-branch-pipeline-wakes-controller
kind: story
status: implemented
title: A controller is told when the default-branch pipeline of its merge decides
summary: The watch keeps the merge commit it relayed and relays the first decided default-branch pipeline of it, green or red, once
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#193
relations:
- decomposes: epic:architecture-next
- informed_by: story:merge-wakes-idle-controller
- informed_by: story:watch-gitlab-ci
scope:
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: docs/design/conductor-run.md
- confidence: inferred
  path: website/docs/concepts/the-loop.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T11:48:53Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-18T11:48:53Z", actor: "human:unknown", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-19T17:02:34Z", actor: "human:unknown", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #193 (friction F51). A controller that merged its release request and waits for the default
branch's pipeline of the merge commit before it tags is never told that pipeline finished; the
operator tagged by hand and woke the session with `conductor session deliver`.

Release 1.14.4 (`story:merge-wakes-idle-controller`) does not cover it. The watch relays
`PR merged: <repo>#<n> (merge <sha>) <url>` once, at merge time (`crates/conductor/src/watch.rs:2126-2190`,
kept in `merged.reported`), before the default-branch pipeline has run. The `main` CI probe
(`watch.rs:30`, `main_ci` at `watch.rs:1978-2045`) prints a line only when a workflow turns red
(`CI red on main`) or green after red (`CI green again on main`); a pipeline that goes from pending to
green after a green one is no change and prints nothing. It also reads only repositories the newest
complete snapshot holds Active. The loop relays only those prefixes to the controller
(`crates/conductor/src/run/handlers.rs:126-137`).

## Outcome

- When the watch relays `PR merged` for an open dispatch, it keeps the merge commit with the
  repository and the dispatch (a watch state file, `merged.pending`).
- In each `main` CI read it reads the default branch's pipelines of each kept merge, Active or not:
  on GitLab the pipeline of that sha (the newest pipeline of the default branch created at or after
  the merge when none carries that sha); on GitHub every workflow run of the default branch for that
  sha. When it is decided (all runs completed), it prints once
  `main CI after merge: <repo>#<n> (merge <sha>) green|red (pipeline <id>)`, naming the red workflows,
  and forgets the merge. A merge is forgotten without a line once no open dispatch holds it.
- The loop relays that line to the repository's running controller as it relays `PR merged`.

Decision (class C, controller, 2026-11-18): of the issue's two options, the watch remembers the merge
from its own `PR merged` relay; no command for a controller to register a wait is added. Undone by
removing the kept file; the registered wait stays possible later and needs `spec/` first.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `the_default_branch_pipeline_of_a_merge_is_relayed_once_when_it_decides`: a GitLab
  fake whose merge request merges while its default-branch pipeline is pending, then green after an
  earlier green; the controller gets one `main CI after merge: ... green` delivery and none on the
  next pass.
- `tests/run.rs` `a_github_merge_is_relayed_red_naming_the_failed_workflow`.
- `tests/run.rs` `the_pipeline_of_a_merge_in_an_inactive_repository_is_read`.

## Boundaries

- The `CI red on main` and `CI green again on main` lines stay as they are.
- Waking a controller with no open dispatch is not in this story.
