---
format: aep.planning-md/3
id: story:specifications-owner-nonmembers
kind: story
status: draft
title: A github repository outside the workspace gets a specifications row
revision: 1
---
## Why

Since `specifications-every-source`, a `local:` source gives each repository a specifications row.
A repository of a `github:` owner that is not a member of the workspace still fails the snapshot
(`a_repository_not_in_the_workspace_fails_the_snapshot_naming_it`): the collector has no read path
for it. One unlisted repository then costs the whole snapshot.

## Acceptance

- A `github:` repository that is not a workspace member gets a row (read at `origin/main` through a
  fetch into the cache, or `Missing` with a reason), and the snapshot does not fail on it.
- The test above is rewritten to assert the row.

## Scope

`crates/conductor/src/collect/specifications.rs`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/tests/collect_stores.rs`.
