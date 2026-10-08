# Wave 01

Coordinator: conductor-dev. Skill: `aep:implementing` 0.20.1, wave mode. Approved by the operator
on 2026-10-08 ("good, do this, use /wave and /aep:implementing and use as many agents as needed").

Integration branch `wave/01` from `main` at `21734cd`, tree `~/.local/state/worktree/trees/b10x/conductor/cw01-int`.

Commits this approval covers: one per unit, the merges into `wave/01`, the closing store commit,
and the merge of `wave/01` into `main`. Nothing else.

## Selection

`aep plan artifact waves --kind story --status active`:

```
wave 1
  story:docs-crate
  story:docs-site
  story:watch-done-explains-exit
wave 2
  story:instance-records-start (inferred)
collision: story:docs-crate story:instance-records-start Taskfile.yml
2 wave(s), 1 collision(s), 0 unassessed
```

All four run at once. The `Taskfile.yml` collision is split by task: `instance-records-start` owns
`conductor:start`, `conductor:attach`, `conductor:restart`, `trust` and the new `agents:link`;
`docs-crate` owns the new `docs-generate`, `docs-check` and one line in `check`. A
`git merge-tree` dry run of the two branches runs before either merges.

Left out: `append-cost-test-under-load` (filed this wave, not scoped for it), every other draft.

## Units

| unit | story | branch | tree | scratch | stage |
|---|---|---|---|---|---|
| U1 | watch-done-explains-exit | `w01/u1-watch` | `trees/b10x/conductor/cw01-u1` | `~/.cache/conductor-dev-analysis/w01/scratch/u1` | planned |
| U2 | instance-records-start | `w01/u2-records` | `trees/b10x/conductor/cw01-u2` | `~/.cache/conductor-dev-analysis/w01/scratch/u2` | planned |
| U3 | docs-crate | `w01/u3-docs-crate` | `trees/b10x/conductor/cw01-u3` | `~/.cache/conductor-dev-analysis/w01/scratch/u3` | planned |
| U4 | docs-site | `w01/u4-docs-site` | `trees/b10x/conductor/cw01-u4` | `~/.cache/conductor-dev-analysis/w01/scratch/u4` | planned |

Each tree builds into its own `target/`. Agents: `aep:implementor` per unit, `aep:adversary` on U2
(it changes how sessions start). Briefs: `~/.cache/conductor-dev-analysis/w01/briefs/u<n>.md`.

## Pre-flight

48G free on `/`, load average 7, no earlier wave tree for this repository, one conductor build
measured at 2.4G of `target/` for a package test run (round 16 of the earlier repository).
