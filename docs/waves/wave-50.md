# Wave 50 (wave 1 of the plan, and the two units wave 49 left open)

The operator chose on 2026-11-10 what this wave holds: the five stories of wave 1 of
`aep plan artifact waves --kind story --status active` as it stood at `4d39c3c5c`, and the two
stories whose units left [wave 49](wave-49.md) after their first adversary pass. One pull request
from `wave/50` into `main`. Skill version 0.26.0.

Integration branch `wave/50`, in managed worktree `w49-int` (kept from wave 49). Builds go through
`conductor build run`, one cargo command per run, which shares one target directory per repository
under a lock. Free disk at the start: 48 GB; `build run --reserve 8G` reclaims before it builds.

## Units

| unit | story | waits for | tree | stage |
|---|---|---|---|---|
| U1 | deliver-reaches-bg-session | | `w49-u11` | correction of adversary pass 1 dispatched |
| U2 | box-runtime-starts | | `w49-u12` | correction of adversary pass 1 dispatched |
| U3 | box-crash-loop-reads-failed | U2 | `w50-u3` | not started |
| U4 | checkout-name-differs-from-project | U1 | `w50-u4` | not started |
| U5 | gc-idle-targets | | `w50-u5` | dispatched |
| U6 | getting-started-substrate-steps-portable | U2 | `w50-u6` | not started |

`story:dashboard-memory-live` is in the plan's wave 1 and is not in this wave: its third acceptance
line is a reading of the live dashboard 24 hours after start, which no unit can produce in a wave.
It left wave 49 for the same reason. Nothing takes its place.

Each unit works on branch `impl/<story>`; its scratch is the session scratch directory's
`w50/u<n>/`. No unit has a build directory of its own: every build is in the repository's shared
target. U1 and U2 keep the branches and trees of wave 49's U11 and U12; each branch holds the
unit's commit, the adversary's cases of pass 1 and a merge of `main` at `4d39c3c5c`.

## Collisions the store printed for these stories

- `box-crash-loop-reads-failed` and `box-runtime-starts`: `crates/conductor/src/daemon/reconcile.rs`
  (cited), `crates/conductor/src/placement.rs` and `crates/conductor/tests/placement.rs` (inferred).
- `box-runtime-starts` and `getting-started-substrate-steps-portable`:
  `website/docs/getting-started.md` (inferred).
- `checkout-name-differs-from-project` and `deliver-reaches-bg-session`:
  `crates/conductor/src/spawn.rs` (cited), `crates/conductor/src/run/handlers.rs` (inferred).

The check was the verb's. A unit that shares a file with another forks from `wave/50` after that
one has merged; the table's "waits for" column holds those three waits. No story of the wave is
unassessed, and no blocker in the store names one.

## Decisions the coordinator took for the units

- U1: a delivery to a background session uses a socket and a tmux session name of its own, so two
  deliveries at once do not share one; a refusal that leaves the controller Running does not mark
  the delivery as refused by that controller.
- U2: the box mounts the instance's own state, records and cache directories writable and the
  config file read-only, never the whole store root; a read-only root that contains a writable
  root is not sent beside it.
- U4: the checkout's directory name is the repository's name for a GitLab source too, as it is
  for the other sources; the forge project stays an attribute of the checkout.

## What each story's close needs

A story moves from `active` to `implemented` on a `test_result` record. U2's acceptance also asks
for one live run on substrate-daemon 0.20.0 in the onboarding record; the coordinator makes that
run, and without it the story stays `active` with its code merged.

## Commits this wave makes

One or more commits per unit on its branch, the merges into `wave/50`, the closing store commit
(evidence and moves), and one pull request from `wave/50` into `main`, fast-forwarded once its
checks are green. No tag and no release.

## Agents

`aep:implementor` per unit, then `aep:adversary` per unit that returns green. At most four at once.
