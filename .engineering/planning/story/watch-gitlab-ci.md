---
format: aep.planning-md/3
id: story:watch-gitlab-ci
kind: story
status: draft
title: The watch reports a GitLab default branch turning red or green
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gitlab-source-host
- depends_on: story:gitlab-merge-requests
scope:
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
revision: 4
---
## Why

The watch reads `main` CI through `gh` for the first `github` owner only
(`crates/conductor/src/watch.rs:241`, `:259`), so a GitLab instance never hears that its default
branch turned red or green.

## Acceptance

- For the Active repositories of a `gitlab` source in the newest complete snapshot, `conductor
  watch run` prints the `main` red line when a project's default-branch pipeline turns from success
  to failed, and the green line when it turns from failed to success, read with `glab api
  --hostname <host>` and mapped by `collect/gitlab.rs`'s status function; a test in
  `crates/conductor/tests/watch.rs` with a fake `glab` holds both.

## Notes

- Depends on `story:gitlab-merge-requests` (the status mapping) and through it on
  `story:gitlab-repositories`, which records the rows the watch picks Active repositories from.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
