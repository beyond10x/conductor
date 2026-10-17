---
format: aep.planning-md/3
id: story:gc-finished-trees
kind: story
status: draft
title: gc reclaims finished managed worktrees; stale ones give back their build output
summary: Merged, clean, idle, unleased trees are finished and removed through worktree finish + gc; dirty or unmerged trees untouched past thresholds.tree_stale lose target/ and are kept
relations:
- decomposes: epic:disk-protocol
revision: 1
---
# Story: gc-finished-trees

## Outcome

`conductor gc` and every reclaim run (`conductor host reclaim`, the watch, and `conductor build run` when a request is short of disk) reclaim managed worktrees whose work is finished. A tree whose work is unfinished but abandoned gives back its build output and is kept.

## Context

Live fault on 2026-10-17: free disk on the host fell from about 200G to about 20G. `conductor gc` and `conductor host reclaim` reported 0 reclaimable bytes, while about 120G sat in managed worktrees under `~/.local/state/worktree/trees/<owner>/<repo>/<id>`, many of them for units already merged. Two causes:
- Reclaim only touches trees `worktree` already lists as finished and eligible (`worktree gc --dry-run`), and the build cache `worktree sweep` names. Nobody runs `worktree finish` on a tree once its branch is merged, so a merged tree stays active for good.
- On this host `worktree gc --dry-run` runs longer than the disk panel's measure timeout, so even the eligible trees are not measured.

## Acceptance

1. Spec first: `thresholds.tree_stale`, a Duration, default 48h, in `spec/domains/config.yaml`. New reclaim kinds `FinishedTree` and `StaleTreeBuildOutput` in `conductor.host.ReclaimKind` (`spec/domains/host.yaml`). The gc row (`conductor.observation.GarbageRow`) carries the tree's id and its verdict (`conductor.observation.TreeVerdict`: `Finished`, `Stale`). Validated with ess 0.60, and `crates/conductor-model` regenerated, never hand-edited. `docs/config.md` and the generated reference describe the key.
2. The managed trees are the instance's registered ones, read from `worktree inspect --json --workspace`: lifecycle `active` or `finished`, path under `checkouts.trees`, path present.
3. An `active` tree is **finished** when all of the following hold:
   - (a) Its HEAD is contained in the repository's default branch on origin (`origin/HEAD`, else `origin/main` or `origin/master`). If its branch still exists on origin, that branch's tip is contained in the default branch too. This covers both a merged branch and a branch deleted on origin after its merge.
   - (b) Its tree is clean except for `target/` and `.check/`.
   - (c) No live process has its working directory, program or an open file in it. Before removing, `worktree`'s own in-use check runs (`finish --discard-cache` refuses `worktree-in-use`).
   - (d) No live session lease holds it (`live_leases`), and no live harness session works in it.
   - (e) It was not touched in the last 10 minutes, so a tree created a moment ago is not mistaken for a finished one.

   A `finished`-lifecycle tree is finished when (c) and (d) hold. A tree the disk panel already lists as eligible for removal is left to that row.
4. To reclaim a finished tree, conductor runs `worktree finish <id> --discard-cache --archive`, then `worktree gc --id <id> --apply`. It never deletes a tree itself. If `finish` refuses with any code other than `worktree-in-use`, for example `archive-unsupported-entry` from a nested `.git` under `target/`, conductor removes the tree's `target/` and retries `finish` once. The row reports the retry, and a second refusal leaves the tree in place, reported.
5. A tree that is dirty or unmerged, but has not been touched for longer than `thresholds.tree_stale` and is not in use, is **stale**. It is not deleted. Its `target/` is removed if it is cargo's (holds `CACHEDIR.TAG`, is no link) and was not written in the last 10 minutes, the equivalent of `cargo clean`. It is listed as "stale, kept". "Touched" means the newest of the registry's `last_seen_at`, the tree directory, its git `HEAD` and `index`, and every changed or untracked path.
6. `conductor gc --dry-run` lists, for each tree, what gc would do and the tree's size: finished trees with their whole size, stale trees with the size of their `target/`, and every other tree as a holder kept, with why. It ends with one summary line per repository: trees finished, stale and kept, each with GiB. `conductor gc` (apply) does it and reports the bytes freed per tree.
7. `conductor host reclaim` and `build run`'s reclaim run the same pass as their first step: finished trees first, then the `target/` of stale trees, one tree at a time, reading free disk after each and stopping at the floor. Each step is recorded in the ReclaimRun with its new kind.
8. Tests use temp git repositories (a bare origin, a clone and linked worktrees), a fake `worktree` that logs its calls, and a Drop cleanup. They cover: merged, branch-deleted-and-contained, unmerged, dirty, dirty-only-in-target/.check, leased, in use by a process, fresh, stale, the archive-refusal retry, the dry-run removing nothing, and reclaim taking the pass. `--test isolated_home` passes. The suite, clippy and conformance are green.
9. Evidence: the new `conductor gc --dry-run` run read-only against this host, with its per-repository summary recorded on the story.

## Out of Scope

- Changing the dashboard's disk panel.
- Squash-merged branches, whose HEAD is not contained in the default branch. These trees are classed stale, not finished.
- Trees under another owner's trees root than the instance's `checkouts.trees`.
- Fetching origin before the merge check. The check reads the refs the clone has; `worktree gc` revalidates recovery before it removes anything.

## Ambiguities

- "Touched" is not defined by the operator. Acceptance 5 defines it, and acceptance 3(e) adds a 10-minute grace for finished trees.
