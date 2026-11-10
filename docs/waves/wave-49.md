# Wave 49 (the next ten accepted stories)

The operator asked on 2026-11-09 for the next ten accepted stories, shipped in one pull request.
The plan is `aep plan artifact waves --kind story --status active` as it stood at `334fa7eb5`:
its wave 1 (seven stories) and the first three of its wave 2. Wave 2's stories collide with wave
1's on `crates/conductor/src/session.rs`, `src/run/handlers.rs`, `src/watch.rs` and
`tests/run.rs` (the verb's collision lines), so they run as batch B, forked from the integration
branch after batch A has merged. Wave 2's fourth story, `doctor-checks-guard-forge-writes`, is not
in this wave.

Wave 48's branches (`wave/48`, `wave/48-u1`, `-u2`, `-u5`, `-u6`) were never merged and are not
touched; `wave/48-u1` holds a work-in-progress commit for `kpi-scope-active-repositories` that
U6 may read.

Integration branch `wave/49`, in managed worktree `w49-int`. Builds go through
`conductor build run`, which shares one target directory per repository under a lock.

## Units

| unit | story | batch | stage |
|---|---|---|---|
| U1 | auto-resume | A | merged |
| U2 | board-hide-done | A | merged |
| U3 | build-shared-target-dir | A | no code change; closed on a test result |
| U4 | dashboard-memory-live | A | left the wave |
| U5 | guard-admits-own-pipelines | A | merged |
| U6 | kpi-scope-active-repositories | A | merged |
| U7 | status-current-snapshot | A | merged |
| U8 | conductor-context-budget | B | merged |
| U9 | context-watch-by-envelope | B | no code change; closed on a test result |
| U10 | dashboard-ui-loop | B | merged |
| U11 | deliver-reaches-bg-session | C | left for wave 50 after adversary pass 1 (blockers) |
| U12 | box-runtime-starts | C | left for wave 50 after adversary pass 1 (blockers) |
| U13 | dispatch-refuses-unknown-repository | C | merged |
| U14 | decision-options-spellings | C | merged |
| U15 | guard-admits-own-mr-auto-merge | C | merged |
| U16 | spawn-pins-binary-and-config | C | not started; left for wave 50 |
| U17 | worktree-create-own-instance-trees | C | not started; left for wave 50 |
| U18 | box-reaches-forge-and-daemon | C | not started; left for wave 50 |
| U19 | box-crash-loop-reads-failed | C | not started; left for wave 50 |
| U20 | started-report-after-restart | C | not started; left for wave 50 |
| U21 | need-repository-from-tag | C | not started; left for wave 50 |
| U22 | checkout-name-differs-from-project | C | not started; left for wave 50 |
| U23 | documented-commands-parse | C | merged |
| U24 | dashboard-live-controllers-and-report-text | C | merged |
| U25 | getting-started-substrate-steps-portable | C | not started; left for wave 50 |

Each unit works in managed worktree `w49-u<n>` on branch `impl/<story>`; its scratch is the
session scratch directory's `w49/u<n>/`.

## Commits this wave makes

One or more commits per unit on its branch, the merges into `wave/49`, the closing store commit
(evidence and moves), and one pull request from `wave/49` into `main`, fast-forwarded once its
checks are green. No tag and no release.

## Metric: `target/` bytes on the host (U3, build-shared-target-dir)

Bytes of every `target/` directory under the checkouts and the managed trees
(`find -maxdepth 2|4 -name target` and `du -sb`), plus the shared targets of `build run` under the
instance's cache. The before reading is wave 46's, taken before shared targets existed.

| | before (2026-11-08T07:57Z) | after (2026-11-09T19:37Z) |
|---|---|---|
| per-tree `target/` directories | 43 | 7 |
| their bytes | 34.06 GB | 7.47 GB |
| shared targets | 0 | 2 |
| their bytes | 0 | 58.59 GB |
| total | 34.06 GB | 66.06 GB |

The total rose. This repository's shared target holds 50.30 GB of it: `debug/` 42.03 GB, `tmp/`
7.29 GB (the scratch failed test cases keep) and `release/` 0.97 GB. The other shared target
holds 8.08 GB. A build of this repository ran in its shared target during the reading.

## Backlog filed during the wave

From the adversary passes: `story:board-grid-columns-tested`, `story:shipped-newest-complete-shared`,
`story:handover-restart-by-envelope-id` (from the implementor of U9).

From a trial instance on a self-hosted GitLab running conductor 1.0.1: `story:spawn-pins-binary-and-config`,
`story:checkout-name-differs-from-project`, `story:dispatch-refuses-unknown-repository`,
`story:documented-commands-parse`, `story:deliver-reaches-bg-session`,
`story:started-report-after-restart`, `story:decision-options-spellings`,
`story:dashboard-live-controllers-and-report-text`, `story:need-repository-from-tag`,
`story:guard-admits-own-mr-auto-merge`, `story:box-runtime-starts`,
`story:box-reaches-forge-and-daemon`, `story:box-crash-loop-reads-failed`,
`story:getting-started-substrate-steps-portable`, `story:worktree-create-own-instance-trees`,
`story:host-topology-page`.

The operator accepted `story:deliver-reaches-bg-session` and `story:box-runtime-starts` on
2026-11-10 as the first stories of wave 50. Later that day the operator asked for every defect
the trial reported to join this wave instead, with five more implementors at once. Those fifteen
stories are batch C, units U11 to U25, forked from `wave/49` as it stands when each starts; a unit
that shares a file with an earlier one waits for that one's merge. `story:host-topology-page` is a
documentation request that waits for a live boxed run and stays in the backlog.

The ten batch C stories that had not started were scoped by `story-scoper` on 2026-11-10, and the
waits in the table follow those scopes: U16, U22 and U17 share `spawn.rs` and `config.rs`; U14, U20,
U21 and U22 share `run/mailbox.rs` or `tests/run.rs`; U12, U18 and U19 share `placement.rs` and
`daemon/reconcile.rs`; U17 and U19 share `doctor.rs`; U12, U18 and U25 share `getting-started.md`.

## Close

Twelve units are in `wave/49`: ten merged and two (U3, U9) that needed no code change. U4 left the
wave. U11 and U12 left it after their first adversary pass found blockers that need design work;
their branches `impl/deliver-reaches-bg-session` and `impl/box-runtime-starts` stay, and their
stories stay active. Eight batch C units did not start. All ten are left for wave 50.

Backlog drafts filed at the close, from the adversary passes and from what the wave's builds showed:
`story:guard-direct-write-expansions`, `story:guard-admits-numeric-project-id`,
`story:dispatch-unknown-repository-named-error`, `story:repository-page-live-controller`,
`story:build-run-waiting-ends-on-signal`, `story:build-run-no-slot-while-waiting-for-target`,
`story:build-run-shared-target-for-any-command`, `story:build-queue-class-and-view` and
`epic:deployable-in-kubernetes`.

Not measured: the first-turn tokens and the four-hour figure with eight controllers of
`story:conductor-context-budget`. Not built: the numeric project id of
`story:guard-admits-own-mr-auto-merge`; a second adversary pass ran only where the first one found
something the unit introduced and its correction was more than a few lines.
