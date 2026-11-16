---
format: aep.planning-md/3
id: story:build-target-per-tree
kind: story
status: draft
title: Reconsider the shared per-repository target directory where trees switch often
summary: build run's one target per repository rebuilds the workspace on every tree switch and leaves a second full build tree when a gate bypasses it; decide per-tree targets with the repository's sccache, or a bound on the shared one
tags:
- from-finding
relations:
- decomposes: epic:disk-and-gate
- informed_by: story:build-shared-target-dir
- informed_by: story:build-run-repo-cache
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: inferred
  path: crates/conductor/src/gc.rs
revision: 2
---
## Why

Agentbox friction logs F39 and F43 (2026-11-15/16, ai-agent-platform, a Rust workspace).

- F39: `conductor build run` gives every worktree of a repository one `CARGO_TARGET_DIR`
  (`crates/conductor/src/build.rs:94-105`, set at `:1292`). Each switch between trees marked
  about 6038 files changed and rebuilt the workspace crates. Fingerprints follow the source
  path, which differs per tree.
- F43: a full gate asked 40G and was refused for disk. The shared target held 49G and had not
  been used since 15:32, while the gate built in the worktree's own `target/` (27G). The result
  was two full build trees for one repository. The operator removed the stale shared target by
  hand, which took free space from 53G to 101G.

The store holds two implemented stories that disagree. `story:build-run-repo-cache` rejected
a shared target ("cross-tree correctness incidents", "cargo's build-dir lock would park a
granted slot") and gave each repository an sccache. `story:build-shared-target-dir` then
introduced the shared target under a per-repository lock.

## Outcome

A decision, recorded in this story, for repositories whose trees switch often: per-tree
targets with the repository's sccache, or the shared target kept and bounded. Bounded means
gc reclaims a shared target idle past `tree_stale`, and a gate that runs outside
`build run` is named by doctor or the watch.

## Acceptance

- On a fixture repository with two trees, alternating `build run -- cargo build` between them
  rebuilds no more workspace crates than the change requires, and no repository holds two
  full target trees after a gate. A `tests/build_run.rs` case holds whichever variant is
  decided.

## Decision

Per-tree targets with the repository's sccache (decided 2026-11-16). `build run` sets no
`CARGO_TARGET_DIR` and takes no repository lock; each tree builds in its own `target/`.

- The shared target cannot be fast across tree switches: cargo keys a workspace crate's artifacts
  by its path inside the workspace, the same in every tree, so correctness required
  `SharedTarget::claim` to mark the tree's files modified on every switch (F39). Bounding it
  would not remove the rebuilds.
- A gate run outside `build run` builds in the tree's own `target/`, so with per-tree targets it
  reuses what `build run` built there instead of building a second full tree (F43).
- The dependencies stay shared through `story:build-run-repo-cache`, which this story's sibling
  rejected a shared target in favour of.
- Idle per-tree targets are reclaimed by the host reclaim and `conductor gc`
  (`story:gc-idle-targets`). A shared target an earlier version left is an idle cargo target to
  the host reclaim (it no longer excludes them) and `conductor gc` removes it.
- Held by `tests/build_run.rs` `alternating_trees_each_keep_their_own_fresh_target` and
  `tests/reclaim.rs` `reclaim_cleans_a_shared_target_an_earlier_build_run_left`.
