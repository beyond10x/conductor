---
format: aep.planning-md/3
id: review-result:autonomy-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, epic conductor-autonomy
relations:
- reviews: story:self-arming-restart
- reviews: story:status-board
- reviews: story:prebuilt-binaries
- reviews: story:resumable-gates
- reviews: story:work-beside-long-gate
- reviews: story:claims-cite-source
- reviews: story:gc-before-pause
- reviews: story:operator-items-age
revision: 1
---
needs-revision

- story:gc-before-pause — the first acceptance bullet states two outcomes, that `conductor gc` lists the archive strip and that `--apply` runs it, but its test only records that `--apply` called `worktree` and the dry run did not, so the listing is never checked — .engineering/planning/story/gc-before-pause.md:27

**Round-1 findings**

- **story:self-arming-restart** (taskfile test did not cover the hand-over rule): resolved. The hand-over rule now has its own bullet with a `crates/conductor/tests/profiles.rs` test (self-arming-restart.md:25-26). The restart bullet's test is in `taskfile.rs` (self-arming-restart.md:21-23).
- **story:status-board** (only the delta, `no checkout` and the overview were checked): resolved. The first bullet requires all seven headings in order plus the snapshot id in the header, and the delta, `no checkout`, refusal and Taskfile bullets are separate (status-board.md:44-58). Row-level content for most deviation kinds and overview columns is still unchecked. That is thin rather than unobservable, so I do not raise it.
- **story:resumable-gates** (`.agents/conductor.md` unchecked): resolved. The second bullet puts the rule in `.agents/conductor.md`'s dispatch rules and names `profiles.rs` as the test (resumable-gates.md:23-25).
- **story:claims-cite-source** (the news cron prompt does not exist): resolved. The acceptance now sits in `.agents/conductor.md`'s reports and daily brief with a `profiles.rs` test (claims-cite-source.md:23-26). `.agents/conductor.md` has a Report step (line 80) and a daily cycle (line 22), so the rule has somewhere to live.
- **story:gc-before-pause**, two outcomes joined in one sentence: split into three bullets, but the first still joins "lists" and "runs", as in the finding above.
- **story:gc-before-pause**, the "before it pauses work for disk" ordering: resolved. It is now a conductor.md sentence held by a profile test (gc-before-pause.md:33-34), and an ordering rule in the profile is checked by its wording.
- **story:operator-items-age** (three outcomes, one check): resolved. The age shown is a `decision.rs` test reading `2d`, and the re-send and default-at-deadline rule is a `profiles.rs` test over both sentences (operator-items-age.md:26-31).

`story:prebuilt-binaries` and `story:work-beside-long-gate` still raise no acceptance defect. The prebuilt-binaries bullet can only be checked at the next release, but it names the five assets, the checksum check and where the result is recorded.

Read 8 of 8 ids (story:self-arming-restart, story:status-board, story:prebuilt-binaries, story:resumable-gates, story:work-beside-long-gate, story:claims-cite-source, story:gc-before-pause, story:operator-items-age). I ran `aep plan artifact show` on each, read `autonomy-acceptance-r1` and the critic rubric, and ran `git grep` for `SnapshotNotComplete`, `hands-todo`, `cadence.cycle`, `watch run` and the conductor.md steps. I also ran `worktree prune-archives --help`, which confirmed `--strip-build-output`.

Not established: whether `ess`'s `release.yml` uses the naming the prebuilt-binaries story copies, since I did not open that repository. I did not run `aep plan artifact kinds` or `lifecycle` this round, relying on round 1's reading. Nothing out of my lane.

```findings
- file: .engineering/planning/story/gc-before-pause.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first acceptance bullet states two outcomes, that conductor gc lists the archive strip and that --apply runs it, but its test only records that --apply called worktree and the dry run did not, so the listing is never checked"
```
