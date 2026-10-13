---
format: aep.planning-md/3
id: story:dashboard-worktree-repo
kind: story
status: implemented
title: The dashboard passes worktree a checkout, not the records directory
relations:
- informed_by: story:gc-archive-profile
scope:
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:07Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:08Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:08Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`Sources::with` in `crates/conductor/src/dashboard.rs` builds `worktree sweep` and `worktree gc
--dry-run` with the records directory as `--repo`, the same defect `story:gc-archive-profile`
fixed for `prune-archives` (found by its implementor, wave 17): the records directory is in no
worktree profile, so those measurements can be refused.

## Acceptance

- The dashboard's `worktree sweep` and `worktree gc --dry-run` calls pass `--repo` the checkout
  `story:gc-archive-profile` picks (the first git checkout under `checkouts.root`), not the records
  directory; a test in `crates/conductor/tests/dashboard.rs` with a fake `worktree` on `PATH`
  records the `--repo` of both calls and finds the checkout in each.

## Scope

`crates/conductor/src/dashboard.rs`, `crates/conductor/tests/dashboard.rs`.
