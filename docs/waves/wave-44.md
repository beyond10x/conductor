# Wave 44 (trees the guard allows, KPIs the instance can read)

Three stories reported by instances on 2026-11-07: controllers that could not work in the trees
`worktree create` made, and KPI readings that no target judged or nothing done could lower. One
tree, one release.

## Units

| unit | story | note |
|---|---|---|
| U1 | guard-and-worktree-agree-on-tree-path | the guard reads a tree's repository from its `.git` file, not from its folder name; a checkout under the checkouts root is the topmost directory holding `.git`, at any depth; a linked worktree of the session's own checkout outside `checkouts.trees` is its own tree |
| U2 | metric-target-rejudges-readings | a target set after a day's compute judges the metric's newest reading |
| U3 | k5-tags-and-no-release-repositories | a semver tag counts as a release for K5; a repository marked as shipping without releases is left out of K5 |

`worktree create` places a tree at `<profile worktree_root>/<checkout folder name>/<id>`
(the `worktree` repository, `crates/worktree/src/lib.rs`, `create` plan). Conductor does not
choose that path, so U1 makes the guard accept the tree where it is; where the tree lands is the
`worktree` CLI's profile.

## Metric

Rows of `conductor metric kpi-readings` on an instance whose KPIs all have a target, and K5
readings a session can lower.

| | before (2026-11-07) | after |
|---|---|---|
| kpi-readings rows, this instance (9 KPIs `Targeted`) | 0 | |
| guard denials of Edit, `cd` or `git` in a tree `worktree create` made, grouped checkouts | reported by 4 controllers on two instances | |
