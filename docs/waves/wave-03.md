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
| U1 | instance-session-names, controller-report-sendmessage | `aep:implementor`, then `aep:adversary` ×2 | `cw03-u1` | `w03/u1-session-names` | `~/.cache/conductor-dev-analysis/w03/scratch/u1` | merged `318a73fd2` |
| U2 | session-box-spike | `aep:implementor` (page only) | `cw03-u2` | `w03/u2-box-spike` | `~/.cache/conductor-spikes/u2` | merged `915a0c4f7` |
| U3 | substrate-tool-spike | `aep:implementor` (page only) | `cw03-u3` | `w03/u3-tool-spike` | `~/.cache/conductor-spikes/u3` | merged `6b3c12b16` |
| U4 | tests-isolate-home (added mid-wave: U1 made the live config invalid for 17 test files) | `aep:implementor` | `cw03-u4` | `w03/u4-isolate-home` | `~/.cache/conductor-dev-analysis/w03/scratch/u4` | merged `efd871e0d` |

Each tree builds into its own `target/`. Briefs: `~/.cache/conductor-dev-analysis/w03/briefs/`.

Pre-flight 2026-10-09: main checkout clean on `main`; no tree of an earlier wave active; `/` 23G
free (floor 10G; units stop under 12G); model budget not stated, N = 3.

## Rollout notes

- U1 changes `.agents/conductor.md` (the conductor profile): its diff goes to the operator before
  `wave/03` merges into `main`.
- U1 keeps today's names for an instance without `session_prefix`, so `task install` changes
  nothing for the running instance; setting the prefix is conductor's change at a restart.

## Outcome

Closing gate on `wave/03` at `efd871e0d`: `task check` exit 0 (Gates scan 517 files, spec valid,
conformance 281 of 281, 619 passed, 0 failed, 3 ignored), `task docs-check` 5 generated files
current. Log: `~/.cache/conductor-dev-analysis/w03/close-check.log`.

| unit | result |
|---|---|
| U1 | adversary pass 1: 7 findings, pass 2: 4 new, 0 carried; all fixed; claim verified against the installed binary (`verify-u1/baseline.out` vs `treatment.out`) |
| U2 | way (a), `claude` in the foreground under `script` in a systemd unit: go; way (b): no-go |
| U3 | tool route: no-go on this host (no `io` controller in the user scope); six substrate and harness fixes asked of conductor; the operator chose B for staging (`approval-record:harness-staging-b`) |
| U4 | 598 of 598 with HOME empty, invalid, valid or real (87 failed against an invalid config before) |

Filed during the wave: `watch-unsent-report`, `watch-tests-shared-isolation`,
`decision-blocker:host-io-delegation`, upstream blockers `substrate-usage-without-io`,
`substrate-gates-in-trees`, `harness-gate-staging` (cleared by B).

Trees `cw03-u1` … `cw03-u4` finished with `--discard-cache --archive` and gc-ed (8.1 GiB freed);
the spike scratch copy removed.

Before `main`: the operator's word on the profile diff
(`~/.cache/conductor-dev-analysis/w03/profile-diff.patch`). At install, the config gains
`session_prefix: work` for the second instance in the same command as `task install`.
