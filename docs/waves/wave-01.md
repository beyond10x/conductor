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
| U1 | watch-done-explains-exit | `w01/u1-watch` | `trees/b10x/conductor/cw01-u1` | `~/.cache/conductor-dev-analysis/w01/scratch/u1` | merged |
| U2 | instance-records-start | `w01/u2-records` | `trees/b10x/conductor/cw01-u2` | `~/.cache/conductor-dev-analysis/w01/scratch/u2` | merged |
| U3 | docs-crate | `w01/u3-docs-crate` | `trees/b10x/conductor/cw01-u3` | `~/.cache/conductor-dev-analysis/w01/scratch/u3` | merged |
| U4 | docs-site | `w01/u4-docs-site` | `trees/b10x/conductor/cw01-u4` | `~/.cache/conductor-dev-analysis/w01/scratch/u4` | merged |

Each tree builds into its own `target/`. Agents: `aep:implementor` per unit, `aep:adversary` on U2
(it changes how sessions start). Briefs: `~/.cache/conductor-dev-analysis/w01/briefs/u<n>.md`.

## Pre-flight

48G free on `/`, load average 7, no earlier wave tree for this repository, one conductor build
measured at 2.4G of `target/` for a package test run (round 16 of the earlier repository).

U5 joined during the close: `append-cost-test-under-load` (the wall-clock ratio test failed the
closing gate, its fourth failure of the day under load), branch `w01/u5-append-cost`, tree
`cw01-u5`, merged.

## Outcome

Closing gate on `wave/01`: `task check` exit 0, 523 passed, 0 failed, 3 ignored (measurements);
conformance 281 of 281; `docs-check` 5 generated files current; Gates scan 341 files; site build
`npm --prefix website ci && npm --prefix website run build` exit 0. The first closing run failed
only on the append-cost ratio (exit 201), which U5 moved out of the gate.

Adversary on U2: pass 1, 7 findings (6 fixed, 1 no-op); pass 2, 6 new, 0 carried (5 fixed, 1
no-op); the correction to pass 2 was checked by the coordinator.

| agent | tokens | tool uses | wall |
|---|---|---|---|
| U1 implementor | 45,218 | 20 | 31 min |
| U2 implementor, 3 rounds | 378,422 | 109 | 45 min |
| U2 adversary pass 1 | 94,809 | 31 | 5 min |
| U2 adversary pass 2 | 98,688 | 40 | 6 min |
| U3 implementor | 222,734 | 96 | 16 min |
| U4 implementor | 217,342 | 128 | 13 min |
