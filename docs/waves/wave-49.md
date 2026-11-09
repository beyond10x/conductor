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
| U1 | auto-resume | A | dispatched |
| U2 | board-hide-done | A | dispatched |
| U3 | build-shared-target-dir | A | dispatched |
| U4 | dashboard-memory-live | A | dispatched |
| U5 | guard-admits-own-pipelines | A | dispatched |
| U6 | kpi-scope-active-repositories | A | dispatched |
| U7 | status-current-snapshot | A | dispatched |
| U8 | conductor-context-budget | B | waits for A |
| U9 | context-watch-by-envelope | B | waits for A |
| U10 | dashboard-ui-loop | B | waits for A |

Each unit works in managed worktree `w49-u<n>` on branch `impl/<story>`; its scratch is the
session scratch directory's `w49/u<n>/`.

## Commits this wave makes

One or more commits per unit on its branch, the merges into `wave/49`, the closing store commit
(evidence and moves), and one pull request from `wave/49` into `main`, fast-forwarded once its
checks are green. No tag and no release.

## Backlog filed during the wave

None yet.
