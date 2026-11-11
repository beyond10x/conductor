# Wave 51 (wave 1 of the plan, and three stories a consumer control plane asked for)

The operator asked on 2026-11-11 for a release and then the next wave, and that the wave serve
what a consumer control plane asked of conductor (`approval-record:wave-51-consumer-stories`).
The wave holds the four stories of wave 1 of `aep plan artifact waves --kind story --status
active` that can run, and the three stories of `epic:deployable-in-kubernetes` that system asked
for first, which that record moved from `draft` to `active`. One pull request from `wave/51`
into `main`. Skill version 0.26.0. The wave before is [wave 50](wave-50.md).

Integration branch `wave/51`, in managed worktree `w51-int`, forked from `release/1.2.0`; `main`
is merged into it once the release is on `main`, before the first unit forks. Builds go through
`conductor build run`, one cargo command per run, which shares one target directory per
repository under a lock. Free disk at the start: 104 GB.

## Units

| unit | story | waits for | tree | stage |
|---|---|---|---|---|
| U1 | need-repository-from-tag | | `w51-u1` | merged (`900de5852`), two adversary passes |
| U2 | handover-restart-by-controller-id | | `w51-u2` | merged (`10542e8b1`), two adversary passes |
| U3 | doctor-checks-guard-forge-writes | | `w51-u3` | adversary pass 2, after correction 2 |
| U4 | box-reaches-forge-and-daemon | | `w51-u4` | adversary pass 2, after correction 2; the egress half is blocked |
| U5 | daemon-probes-and-metrics | | `w51-u5` | correction 2 after adversary pass 2 |
| U6 | release-container-image | | `w51-u6` | merged (`61eb01e39`), two adversary passes |
| U7 | container-start-without-prompt | U3, U4, U5, U6 | `w51-u7` | not started |

Each unit works on branch `impl/<story>`; its scratch is the session scratch directory's
`w51/u<n>/`. No unit has a build directory of its own: every build is in the repository's shared
target. Implementors are `aep:implementor`, adversaries `aep:adversary`.

`story:dashboard-memory-live` is in the plan's wave 1 and is not in this wave: its third
acceptance line is a reading of the live dashboard 24 hours after start, which no unit can
produce in a wave. It left waves 49 and 50 for the same reason. Nothing takes its place.

`story:control-plane-work-handler` and `story:cluster-install` of the same epic stay drafts: the
work contract is not frozen on the consumer's side, and the install waits for these three.

## What the store printed for these stories

The check was the verb's, after the scope entries of the three new stories were written. It
places the four plan stories in its wave 1, `container-start-without-prompt` in its wave 3 and
`daemon-probes-and-metrics` and `release-container-image` in its wave 4; no story is unassessed.
Its collision lines between two stories of this wave:

- `box-reaches-forge-and-daemon` and `daemon-probes-and-metrics`:
  `crates/conductor/src/daemon/reconcile.rs`, `website/docs/concepts/the-host.md` (inferred).
- `box-reaches-forge-and-daemon` and `release-container-image`:
  `website/docs/getting-started.md` (inferred).
- `box-reaches-forge-and-daemon` and `container-start-without-prompt`:
  `crates/conductor/src/daemon/reconcile.rs`, `crates/conductor/src/secret.rs` (inferred).
- `container-start-without-prompt` and `daemon-probes-and-metrics`: `crates/conductor-model`,
  `crates/conductor/http.yaml`, `crates/conductor/src/daemon/mod.rs`,
  `crates/conductor/src/daemon/reconcile.rs`, `crates/conductor/tests/daemon.rs` (inferred).
- `container-start-without-prompt` and `doctor-checks-guard-forge-writes`:
  `crates/conductor/src/doctor.rs` (cited), `crates/conductor/src/config.rs`,
  `crates/conductor/tests/doctor.rs` (inferred).
- `container-start-without-prompt` and `release-container-image`: `Dockerfile`,
  `website/docs/guides/run-in-a-container.md`, `website/sidebars.ts` (inferred).

How the wave runs with them: U7 forks from `wave/51` after U3, U4, U5 and U6 have merged. U4
runs beside U5 and U6 on a split by function their briefs declare: in `daemon/reconcile.rs` U4
holds `box_request` and `service` and U5 holds `Daemon::open`; in the host page U4 holds the
token sentence and U5 the listeners; in the getting-started page U4 holds the substrate-daemon
command and U6 at most one link in the install section. A `git merge-tree` dry run of each pair
is read before the first of the two merges.

## Decisions the coordinator took for the units

- U1: the first-line form of a NEED stays; the repository a NEED is for is its `<other-repo>`
  field, read as the leading run of name characters; an unknown repository is refused in one
  line. Written into the story.
- U3: doctor asks the guard in process through the function the hook calls, on the CLI path
  only, and records nothing.
- U4: one egress aperture per forge host beside `conductor`, declared by the operator at the
  substrate daemon; the daemon's token reaches the runtime through a sealed slot, never the
  box's environment. Held against the fake substrate only: the live box does not serve its API
  on substrate-daemon 0.20.0 (`decision-blocker:box-api-unix-socket-denied`).
- U5: paths `/healthz`, `/readyz` and `/metrics`, without a bearer, bodies with states, counts,
  ages and a reason only; the daemon serves while the store does not answer; a metric the
  daemon could not read is absent, never `0`.
- U6: the image is `ghcr.io/beyond10x/conductor:<version>`, built from the release run's own
  binary, non-root; the workflow appends the image line to the Release body; the `docker run`
  line is on one new guide page.

## What the units found

- U1, adversary pass 1 (`review-result:w51-u1-adversary-1`): a NEED for conductor itself, a host
  tool or a sibling with no checkout was answered "refused, not kept" to its sender, while the
  role texts have the supervisor answer it from the mailbox. Decided: a NEED for the mailbox's own
  recipient is left for the supervisor with no answer; any other field that names no repository
  is not kept and its sender is told in one line that it is not relayed to a controller and stays
  in the mailbox. A role line that teaches a NEED with one tag field was there before the wave
  and goes to the backlog.
- U1, adversary pass 2 (`review-result:w51-u1-adversary-2`): nothing red against the correction;
  three cases added, each failing under a change the suite let through. One judgement left as
  it was before the wave: where the instance has a checkout that is really named `conductor`, a
  NEED for `conductor` is relayed to or kept for that repository, and the file stays in the
  mailbox for the supervisor too. Package run on the unit's head: 1837 run, 1836 passed, the
  one failure the ledger case that fails under host load.
- U2, adversary pass 1 (`review-result:w51-u2-adversary-1`): the usage-limit path restarted
  through a kept entry without the start check; a session of the same name in another directory
  dropped the request to the controller's own session; an entry written by 1.2.0 for the session
  still running was dropped and the controller never asked again. All three go back to the
  implementor, the first because acceptance line 2 needs it although the base has it.
- U4: the pinned substrate SDK (0.20.0) sends one aperture per session and has no field for the
  working directory, and the SDK on that repository's `main` does not either, so the box request
  cannot name a forge host (`decision-blocker:substrate-sdk-single-aperture`). The unit built
  the token half: the daemon's token reaches a local box through a sealed slot. Decided: a
  substrate daemon that does not declare the slot does not refuse a box that started on 1.2.0;
  the box starts without the slot and doctor names the flag. The story stays `active` at the
  close.
- U5: only `/readyz` is in the HTTP binding, and its 503 is not: the binding format gives a view
  route one success status. `/healthz` reads no entity and `/metrics` is text; both are unbound
  rows of the parity baseline.
- U3: after the first round a new instance left `conductor setup` on the `gates` route with no
  `gates_policy`, and doctor then said `not ready`. Decided: setup chooses no route; it prints
  the `forge route` line with its fix and writes `gates_policy` from the environment's policy
  path into an instance it creates; SETUP.md gains a step where the person chooses the route;
  every runnable example config names the direct route; a policy only the environment names
  stays `not ready`, and the fix line carries the path. Still open after that round: setup's
  closing verdict can say `ready` where doctor says `not ready`; it goes back with the
  adversary's findings.
- U2, correction 1: the usage-limit path and the hand-over path share one rule; the session
  list is read for every session of a name; an entry of 1.2.0 is tied to its session by the id
  in its line. Decided: the loop's own resume of a session keeps the request, and the entry is
  stamped at that resume; a session resumed by hand after the delivery loses its request. Found
  beside the findings: a resume on another token dropped the entry of a session that runs on.
- U2, adversary pass 2 (`review-result:w51-u2-adversary-2`): nothing red against the correction;
  three cases added, one of them failing under a change the suite let through. One note stays
  open and goes to the backlog: the stamp of the loop's resume is the instant the resume
  command returned, and the tree does not show whether the harness dates a resumed session
  before or after it. Package run on the unit's head: 1848 run, 1847 passed, the one failure
  the ledger case that fails under host load.
- U4, adversary pass 1 (`review-result:w51-u4-adversary-1`): the token path held. Doctor's dry
  run of a box did not know the daemon's new refusal for a slot file it cannot write, and the
  slot's descriptor and variable were inherited by every process the boxed runtime starts.
  Both go back. The file `runtimes.json`, which holds the same token at the umask mode, was
  there before the wave and goes to the backlog.
- U3, adversary pass 1 (`review-result:w51-u3-adversary-1`): doctor asked the guard about the
  first checkout of a role only, so it ended `ready` while the hook denied the lead of a second
  checkout; the fix line for a policy named relative gave a path the config check refuses; a
  `gates_policy` naming no file read `ready`; setup ended `ready` where doctor did not. All go
  back. Decided with them, on the operator's word: setup asks the person nothing about the
  route; `setup --apply` derives it for an instance it creates from the kind of the
  repository's origin (GitLab or no forge: direct; GitHub with a readable policy in the
  environment: the bot route; GitHub without one: direct).
- U5, adversary pass 1 (`review-result:w51-u5-adversary-1`): `/readyz` answered 200 for ever
  after one failed write on the reconciler's store handle, and 200 before the first
  publication; `/v1/instances` answered `200 []` while the store was down. All three go back.
- U5, adversary pass 2 (`review-result:w51-u5-adversary-2`): the probes, the 503 of the `/v1/`
  reads and the bound of the held records held. Two cases were red: a pass the store refused
  half-way ran again as a cadence pass, so a config change no longer ended a back-off; and a
  daemon stopped while it held the record of a start started that runtime again when it came
  back. The base stops a refused pass at the same place, but the unit made the store outage
  its subject and its page promised more, so both go back with the page.
- U6: the commit gate refuses a home directory under `/home/` in the `Dockerfile`; the image's
  user gets `/var/lib/conductor`. Decided: the job that builds and pushes the image holds
  `packages: write` only, and a third job with `contents: write` only writes the Release notes.
- U6, adversary pass 1 (`review-result:w51-u6-adversary-1`): the push step's test of the newest
  release sat inside `[ … ]`, so an API error ended the step green with `latest` not moved;
  four of the unit's cases held a substring of the workflow and no behaviour; the image's
  extracted tools kept the archive's owner id; the notes script ended its section only at a
  second-level heading. All go back; the adversary's cases run the step scripts against
  stand-in programs.

- U6, adversary pass 2 (`review-result:w51-u6-adversary-2`): the push step, the rule for
  `latest` and the image's owners held. Two cases were red on Release bodies nobody is shown
  to write: a fence never closed, after which a re-run left two sections, and a heading quoted
  in an HTML comment. Both corrected in a last round; the lane is 29 run, 29 passed.

## What each story's close needs

`aep plan artifact lifecycle story`: `active -> implemented`, on a `test_result` record of the
gate on `wave/51`. `release-container-image` has two acceptance lines only a release run of the
workflow can show (the push and the Release notes); it moves on the gate's record and the first
release after the wave reads those two lines.

## Commits this wave makes

One commit per unit round on its `impl/<story>` branch, the adversary's cases, the merges into
`wave/51`, this page and the store writes, the closing commit, and the merge of `wave/51` into
`main` through one pull request. Each through the bot route. No tag and no release.
