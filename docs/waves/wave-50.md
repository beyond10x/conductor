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
| U1 | deliver-reaches-bg-session | | `w49-u11` | merged after five rounds and two adversary passes |
| U2 | box-runtime-starts | | `w49-u12` | merged after three rounds and two adversary passes |
| U3 | box-crash-loop-reads-failed | U2 | `w50-u3` | merged after three rounds and two adversary passes |
| U4 | checkout-name-differs-from-project | U1 | `w50-u4` | merged after two rounds and one adversary pass |
| U5 | gc-idle-targets | | `w50-u5` | merged after three rounds and two adversary passes |
| U6 | getting-started-substrate-steps-portable | | `w50-u6` | merged after two rounds and one adversary pass |

U6 did not wait for U2 after all: it forked from `wave/50` at the start and merged first; the
merge of U2 resolved the two places both changed (`SETUP.md` step 9 and the `install -d` line of
the getting-started page).

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

- U1, later rounds: a delivery counts as delivered only after one look, taken after the typing,
  that shows nothing unread on the pane's terminal; an unread text is the new outcome `unread`
  and is not retried. The pane outlives its client, and the delivery writes a mark the pane
  removes, so a step the pane takes late leaves no file. Five offers per controller.
- U2, later rounds: the box needs the store at an absolute path of its own; `setup --apply` writes
  that path only into a new config and never moves a store; three named refusals
  (`placement.writable-root-shared`, `placement.mounts-exceeded`, `placement.store-not-absolute`).
- U3: a runtime that ends within 10 s of its start is a failed start, recorded as refused with the
  last lines of `runtime.log`; restarts wait 10 s, doubling to 5 minutes, and the row ends only
  when a runtime ran 60 s. The log is read only as a regular file, never through a link. `doctor`
  dry-runs the box, asks one local substrate daemon once with a 5 s bound and skips a config file
  that writes no `daemon.store`.
- U4, round 2: one project that two `gitlab` sources list is one repository, recorded by the
  source written first; a clone under a directory that is no repository name is left out and
  named on standard error; an `exclude` entry matches the directory name or the project path.
- U5: a cargo target is recognised by cargo's tag line beside `.rustc_info.json` or a profile's
  `.cargo-lock`; gc takes cargo's lock, moves the target aside under it and removes the moved
  directory; scan roots are the managed trees and the cache's `target` and `scratch` directories.
- U6: no second adversary pass, because the correction changed the unit's test only.

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

## Close

Six units are merged in `wave/50`. Five stories moved to `implemented` on the gate's test record:
`deliver-reaches-bg-session`, `box-crash-loop-reads-failed`,
`checkout-name-differs-from-project`, `gc-idle-targets` and
`getting-started-substrate-steps-portable`. `box-runtime-starts` has its code merged and stays
`active`: the live run on substrate-daemon 0.20.0 ([the onboarding record](../analysis/onboarding-run.md),
"Wave 50 live run") shows the box and its mounts hold and the runtime ends at the bind of its
API socket, which substrate 0.20.0 denies in a box. `decision-blocker:box-api-unix-socket-denied`
holds the reading.

The gate on `wave/50` at `59c8f653f`: the Gates scan, `ess specify validate`, the conformance
run, the model diff, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the docs
check exit 0. `cargo nextest run --no-fail-fast`: 1911 run, 1909 passed, 2 failed, both in files
the wave does not touch: `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`
fails on the host, and `dashboard_fast::a_repeat_request_is_answered_from_memory_and_304_when_unchanged`
passed when its binary ran alone. The dashboard build and its UX gate were left to the pull
request's checks: the wave changes no file of the dashboard.

Adversary passes: two each for U1, U2, U3 and U5, one each for U4 and U6; ten review results, each
with its outcome recorded. A further pass ran where the correction of the one before was more
than a test change, except U4 round 2 and the last round of U1, U3 and U5, which merged on the
adversary's own cases and the gate.

Backlog drafts filed at the close: `story:box-runtime-api-without-unix-socket`,
`story:box-root-names-unique`, `story:conductord-substrate-wait-bounded`,
`story:spawn-exclude-by-project-path`, `story:secret-put-refusal-case-broken-pipe`,
`story:stopped-controller-stays-stopped` and `story:store-postgres-proven`.
`decision-blocker:cluster-placement-and-store` is cleared: a cluster deployment keeps the sqlite
store on a persistent volume, and a Postgres store is the other supported option, not the default.

Left open: conductord's own wait on a local substrate socket has no bound; a delivery killed with
SIGKILL leaves its pane's process until the next delivery ends left panes; `conductor gc` scans
the managed trees and the cache's `target` and `scratch` directories only.
