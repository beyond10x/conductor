---
format: aep.planning-md/3
id: review-result:autonomy-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, epic conductor-autonomy
relations:
- reviews: story:self-arming-restart
- reviews: story:status-board
- reviews: story:resumable-gates
- reviews: story:claims-cite-source
- reviews: story:gc-before-pause
- reviews: story:operator-items-age
revision: 1
---
needs-revision

- story:self-arming-restart — the acceptance's second outcome, "the hand-over keeps only open work", is not checkable by the named test, which covers only the Taskfile and hook — .engineering/planning/story/self-arming-restart.md:30
- story:status-board — the acceptance checks only the delta, `no checkout` and the overview table, so the deviation rows, Specifications, Open blockers, Sessions, Free disk and header from "## What" can be missing and the story still closes — .engineering/planning/story/status-board.md:91
- story:resumable-gates — the acceptance requires conductor's dispatch rules and charter template to state the rule, but its only check is "a test over `.agents/repo-controller.md`", so `.agents/conductor.md` is unchecked — .engineering/planning/story/resumable-gates.md:26
- story:claims-cite-source — the acceptance requires the "news cron prompt" to tell conductor to check each claim, but no news cron prompt exists in `.agents/` or `crates/conductor/src` and "a test over the profile" cannot hold it — .engineering/planning/story/claims-cite-source.md:24
- story:gc-before-pause — the acceptance joins two independent outcomes, stripping archive build output and reporting other holders, in one sentence — .engineering/planning/story/gc-before-pause.md:30
- story:gc-before-pause — "Before conductor pauses work for disk" names an ordering that a test over a fixture archive root and a fixture cache never exercises — .engineering/planning/story/gc-before-pause.md:30
- story:operator-items-age — the acceptance joins three outcomes (age shown, re-send with default and deadline, default taken at the deadline) but its only check is "the age column", so the other two cannot be verified — .engineering/planning/story/operator-items-age.md:26

Read 8 of 8 ids (story:self-arming-restart, story:status-board, story:prebuilt-binaries, story:resumable-gates, story:work-beside-long-gate, story:claims-cite-source, story:gc-before-pause, story:operator-items-age). I ran `aep plan artifact show` on each, `aep plan artifact kinds` and `aep plan artifact lifecycle story`, plus `git grep` for the symbols and prompts they name.

Not established: where the news cron prompt lives. story:prebuilt-binaries and story:work-beside-long-gate raised no acceptance defect. I did not compare prebuilt-binaries against `ess`'s `release.yml`. Out of my lane: the `status-board` frontmatter scope omits `STATUS.md`, which is for the scope or parallel-safety critics.

```findings
- file: .engineering/planning/story/self-arming-restart.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance's second outcome, the hand-over keeps only open work, is not checkable by the named test, which covers only the Taskfile and hook"
- file: .engineering/planning/story/status-board.md
  line: 91
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks only the delta, no checkout and the overview table, so the deviation rows, Specifications, Open blockers, Sessions, Free disk and header from the What section can be missing and the story still closes"
- file: .engineering/planning/story/resumable-gates.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires conductor's dispatch rules and charter template to state the rule, but its only check is a test over .agents/repo-controller.md, so .agents/conductor.md is unchecked"
- file: .engineering/planning/story/claims-cite-source.md
  line: 24
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires the news cron prompt to tell conductor to check each claim, but no news cron prompt exists in .agents/ or crates/conductor/src and a test over the profile cannot hold it"
- file: .engineering/planning/story/gc-before-pause.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes, stripping archive build output and reporting other holders, in one sentence"
- file: .engineering/planning/story/gc-before-pause.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'Before conductor pauses work for disk' names an ordering that a test over a fixture archive root and a fixture cache never exercises"
- file: .engineering/planning/story/operator-items-age.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins three outcomes (age shown, re-send with default and deadline, default taken at the deadline) but its only check is the age column, so the other two cannot be verified"
```
