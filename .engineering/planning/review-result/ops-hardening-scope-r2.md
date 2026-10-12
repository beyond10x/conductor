---
format: aep.planning-md/3
id: review-result:ops-hardening-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, epic operating-loop-hardening
relations:
- reviews: epic:operating-loop-hardening
revision: 1
---
approve

All 12 stories trace to a promise in the epic. Every promise in the epic is claimed by exactly one story. Round 1's one finding (session-name resolution claimed twice) is fixed.

I extracted 11 promises from the epic's Why list (`.engineering/planning/epic/operating-loop-hardening.md:14-25`) and traced all 11:
- The full gate is not tied to the host's disk: `story:ci-full-gate`.
- Nothing internal reaches the public repository: `story:public-text-check`.
- A restart cannot start conductor on a stale profile: `story:restart-current-profile`.
- Two conductors run at once: `story:single-conductor-lease`, `story:lease-guard` and `story:watch-two-conductors`.
- A store write passes as a read: `story:guard-store-migrate`.
- An operator decision does not say which session recorded it: `story:operator-decision-guarded`.
- Session tasks miss prefixed sessions: `story:instance-scoped-tasks`. The epic names it (lines 49-52) and it sits outside the set. Its acceptance, "Every task resolves sessions by the active instance's full names", is the only claim on this outcome. `story:session-tasks-follow-prefix` lines 23-27 now disclaim it.
- Fresh records cannot be snapshotted without an origin: `story:init-records-without-origin`.
- `task agents:link` serves no session: `story:agents-link-user-scope`.
- The dashboard names a fixed context threshold: `story:session-tasks-follow-prefix`. Its id no longer matches its content, but the epic says so (line 47).
- A refusal message has a verb that does not agree with its subject: `story:doctor-refusal-wording`.

The epic's exclusions (`conductor gc`, `conductor hands`) are untouched. The omissions the stories name themselves are not gaps. These are the `AGENTS.md` follow-up in `story:ci-full-gate`, the dashboard and docs follow-ups in `story:operator-decision-guarded`, and the "no upstream: started" case in `story:restart-current-profile`.

What I read: 14 artifacts, namely the epic, the 12 stories, `story:instance-scoped-tasks` and `review-result:ops-hardening-scope-r1`. I read each file under `.engineering/planning/` directly. I did not run `aep plan artifact graph`.

What I could not establish: one unease, not a finding. `story:operator-decision-guarded` also makes the operator's quoted words a required field (`operator-decision-guarded.md:46-49`). The epic says only "does not say which session recorded it" (`epic:22`). The words field traces only loosely to "authority gaps". Acceptance checkability and Taskfile ordering belong to other critics.

```findings
[]
```
