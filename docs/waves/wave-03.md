# Wave 03

Coordinator: conductor-dev. Skill: `aep:implementing` 0.20.1, wave mode. Approved by the operator
on 2026-10-09 ("refine stories to your liking then dispatch waves"; standing word for waves 03–11).

Integration branch `wave/03` from `main` at `4a37012`, tree `cw03-int`. Commits this approval
covers: one per unit, the merges into `wave/03`, the coordinator's planning and wave-page commits,
the closing store commit, and the merge into `main`. No push, tag or release.

Planned on `wave/03` before dispatch: epic `session-confinement` (ADR
`external-confinement`), the multi-instance stories revised, two critic rounds per epic recorded,
`docs/analysis/2026-10-09-substrate-benefits.md` and `…-substrate-sessions.md`.

## Selection

`aep plan artifact waves --kind story --status draft` (before the three moves): wave 1 =
`instance-session-names`, `session-box-spike`, `substrate-tool-spike`; zero colliding pairs
without an ordering path (55 checked). Unassessed (no typed scope; not in this plan):
codex-controller-profile, codex-goals, config-fields-read, conformance, controller-start-reading,
decision-goal, dependency-map, focus-line, guard-lock-timeout, guard-sibling-tree, mailbox,
portable-tree-sizes, restart-message, specifications-owner-nonmembers, status-board,
watch-disk-reading, watch-silent-controller, watch-tree-sizes.

## Units

| unit | story | agent | tree | branch | scratch | stage |
|---|---|---|---|---|---|---|
| U1 | instance-session-names | `aep:implementor`, then `aep:adversary` | `cw03-u1` | `w03/u1-session-names` | `~/.cache/conductor-dev-analysis/w03/scratch/u1` | dispatched |
| U2 | session-box-spike | `aep:implementor` (page only) | `cw03-u2` | `w03/u2-box-spike` | `~/.cache/conductor-spikes/u2` | dispatched |
| U3 | substrate-tool-spike | `aep:implementor` (page only) | `cw03-u3` | `w03/u3-tool-spike` | `~/.cache/conductor-spikes/u3` | dispatched |

Each tree builds into its own `target/`. Briefs: `~/.cache/conductor-dev-analysis/w03/briefs/`.

Pre-flight 2026-10-09: main checkout clean on `main`; no tree of an earlier wave active; `/` 23G
free (floor 10G; units stop under 12G); model budget not stated, N = 3.

## Rollout notes

- U1 changes `.agents/conductor.md` (the conductor profile): its diff goes to the operator before
  `wave/03` merges into `main`.
- U1 keeps today's names for an instance without `session_prefix`, so `task install` changes
  nothing for the running instance; setting the prefix is conductor's change at a restart.
