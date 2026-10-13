---
format: aep.planning-md/3
id: story:dashboard-worktree-repo
kind: story
status: draft
title: The dashboard passes worktree a checkout, not the records directory
relations:
- informed_by: story:gc-archive-profile
scope:
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
revision: 2
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
