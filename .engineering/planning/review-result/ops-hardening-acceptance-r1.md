---
format: aep.planning-md/3
id: review-result:ops-hardening-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, epic operating-loop-hardening
relations:
- reviews: story:ci-full-gate
- reviews: story:single-conductor-lease
- reviews: story:guard-store-migrate
- reviews: story:operator-decision-guarded
- reviews: story:session-tasks-follow-prefix
- reviews: story:init-records-without-origin
- reviews: story:agents-link-user-scope
- reviews: story:public-text-check
- reviews: story:restart-current-profile
revision: 1
---
needs-revision

- story:ci-full-gate — the acceptance holds two independent outcomes (the workflow runs and goes red on a failing test, and `AGENTS.md` carries a merge rule), so one can pass while the other fails — .engineering/planning/story/ci-full-gate.md:25
- story:single-conductor-lease — the acceptance lists four independent outcomes (claim refusal plus guard denial, takeover, the watch incident, the spec validating) instead of one statement — .engineering/planning/story/single-conductor-lease.md:48
- story:single-conductor-lease — no acceptance says the conductor role's settings template runs `conductor session claim` at session start, which is the only thing that makes the lease get taken in a real session: "A second claim while the holder's pid lives exits non-zero…" — .engineering/planning/story/single-conductor-lease.md:48
- story:guard-store-migrate — the sentence ends with "the guard's read-command list is derived so that a new write command cannot pass as a read" and names no check that shows this, so it is true whenever someone says it is — .engineering/planning/story/guard-store-migrate.md:26
- story:operator-decision-guarded — the acceptance joins three independent outcomes (refusal without evidence and controller denial, `decision show` and the dashboard showing the recorder, the website text) — .engineering/planning/story/operator-decision-guarded.md:31
- story:operator-decision-guarded — "The website's security pages state the boundary as built" names no page, sentence or check, so nobody could tell it from before the work — .engineering/planning/story/operator-decision-guarded.md:36
- story:session-tasks-follow-prefix — the acceptance joins three unrelated outcomes (the three tasks follow the prefix, the dashboard threshold, the refusal wording) — .engineering/planning/story/session-tasks-follow-prefix.md:28
- story:init-records-without-origin — the acceptance joins two independent outcomes (the snapshot completes without an origin, the guide drops a step) — .engineering/planning/story/init-records-without-origin.md:28
- story:agents-link-user-scope — "removed, or its description and `AGENTS.md` say what it still serves" names no text the test looks for, and `AGENTS.md:177` already says the link "serves none of the sessions above", so the `AGENTS.md` half cannot be told from before the work: only the Taskfile description at `Taskfile.yml:163` ("so a session started in another directory finds them") still carries the old claim — .engineering/planning/story/agents-link-user-scope.md:28

I found nothing to change in story:public-text-check and story:restart-current-profile. Each has one checkable outcome with a before and after, and public-text-check names a positive control.

What I read: all 9 ids given, 9 read. I ran `aep plan artifact show` on each, `aep plan artifact lifecycle story`, and `git grep` and `ls` on the tree for `agents:link`, `migrate` in the guard, the workflows and the test files.

What I could not establish: I did not check whether the scope lists are complete or correct, and I did not check how the stories depend on or collide with each other. For example, operator-decision-guarded's scope omits `tests/decision.rs` and the dashboard, and several stories touch `Taskfile.yml` and `taskfile.rs`. Both belong to the scope and parallel-safety critics, so they did not set my verdict.

```findings
- file: .engineering/planning/story/ci-full-gate.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance holds two independent outcomes (the check.yml workflow going red on a failing test, and an AGENTS.md merge rule), so one can pass while the other fails"
- file: .engineering/planning/story/single-conductor-lease.md
  line: 48
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance lists four independent outcomes (claim refusal plus guard denial, takeover, the watch incident, the spec validating) instead of one statement"
- file: .engineering/planning/story/single-conductor-lease.md
  line: 48
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no acceptance says the conductor role's settings template runs `conductor session claim` at session start, which is what makes the lease get taken in a real session"
- file: .engineering/planning/story/guard-store-migrate.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the clause 'the guard's read-command list is derived so that a new write command cannot pass as a read' names no check that shows it, so it is true whenever someone says it is"
- file: .engineering/planning/story/operator-decision-guarded.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins three independent outcomes (evidence required and controller denied, decision show and dashboard showing the recorder, the website text)"
- file: .engineering/planning/story/operator-decision-guarded.md
  line: 36
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'The website's security pages state the boundary as built' names no page, sentence or check, so it cannot be told from the state before the work"
- file: .engineering/planning/story/session-tasks-follow-prefix.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins three unrelated outcomes (the three tasks follow the prefix, the dashboard threshold, the refusal wording)"
- file: .engineering/planning/story/init-records-without-origin.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes (the snapshot completes without an origin, the guide drops a step)"
- file: .engineering/planning/story/agents-link-user-scope.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'say what it still serves' names no text the test looks for, and AGENTS.md:177 already says the link serves none of the sessions, so only the Taskfile.yml:163 description's old claim marks the transition"
```
