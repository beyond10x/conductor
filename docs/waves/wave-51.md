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
| U1 | need-repository-from-tag | | `w51-u1` | correction 1 after adversary pass 1 |
| U2 | handover-restart-by-controller-id | | `w51-u2` | correction 1 after adversary pass 1 |
| U3 | doctor-checks-guard-forge-writes | | `w51-u3` | implementing |
| U4 | box-reaches-forge-and-daemon | | `w51-u4` | correction 1; the egress half is blocked |
| U5 | daemon-probes-and-metrics | | `w51-u5` | adversary pass 1 |
| U6 | release-container-image | | `w51-u6` | correction 1 before its first commit |
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
- U6: the commit gate refuses a home directory under `/home/` in the `Dockerfile`; the image's
  user gets `/var/lib/conductor`. Decided: the job that builds and pushes the image holds
  `packages: write` only, and a third job with `contents: write` only writes the Release notes.

## What each story's close needs

`aep plan artifact lifecycle story`: `active -> implemented`, on a `test_result` record of the
gate on `wave/51`. `release-container-image` has two acceptance lines only a release run of the
workflow can show (the push and the Release notes); it moves on the gate's record and the first
release after the wave reads those two lines.

## Commits this wave makes

One commit per unit round on its `impl/<story>` branch, the adversary's cases, the merges into
`wave/51`, this page and the store writes, the closing commit, and the merge of `wave/51` into
`main` through one pull request. Each through the bot route. No tag and no release.
