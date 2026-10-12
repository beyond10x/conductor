---
format: aep.planning-md/3
id: review-result:ops-hardening-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, epic operating-loop-hardening
relations:
- reviews: epic:operating-loop-hardening
- reviews: story:session-tasks-follow-prefix
revision: 1
---
needs-revision

- story:session-tasks-follow-prefix — its acceptance "The three tasks take the session names from `conductor config show`" and story:instance-scoped-tasks (epic:multi-instance-host) "Every task resolves sessions by the active instance's full names" claim the same outcome, with overlapping Taskfile.yml scope. Both can be marked done and one will be a lie. The body must say which story owns session-name resolution in the Taskfile tasks, or narrow its claim to what instance-scoped-tasks leaves out. — `.engineering/planning/story/session-tasks-follow-prefix.md:28` (other claim at `.engineering/planning/story/instance-scoped-tasks.md:32`)

I extracted 9 promises from the parent and traced all 9 to a story:
- the full gate not tied to the host's disk (ci-full-gate)
- nothing internal reaching the public repository (public-text-check)
- a restart on a stale profile (restart-current-profile)
- two conductors at once (single-conductor-lease)
- a store write passing as a read (guard-store-migrate)
- an operator decision that does not say which session recorded it (operator-decision-guarded)
- session tasks missing prefixed sessions (session-tasks-follow-prefix)
- fresh records without an origin (init-records-without-origin)
- `task agents:link` serving no session (agents-link-user-scope)

I found no gaps. The epic's exclusions (`conductor gc`, `conductor hands`) are not touched by any story.

**What I read:** 10 artifacts: the epic and the 9 stories, each with `aep plan artifact show <id>`. I also read `aep plan artifact graph`, `story:instance-scoped-tasks` and `story:instance-session-names`, plus the critic rubric and the scope-critic procedure.

**What I could not establish:** none that affects the verdict. Two things are outside my lane or only an unease:
- **Dashboard text (unease):** session-tasks-follow-prefix also fixes the dashboard's hard-coded "over 300k" line and the refusal wording. The epic names only "session tasks miss prefixed sessions", so these extras trace only loosely to "gaps found on 2026-10-12". I did not count that as reach beyond the parent.
- **Scope list (out of my lane):** session-tasks-follow-prefix lists `crates/conductor/src/dashboard/` and `doctor.rs` in its Scope but not in its front-matter scope. That is for the design and parallel-safety critics.

```findings
- file: .engineering/planning/story/session-tasks-follow-prefix.md
  line: 28
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the acceptance that the tasks take session names from config show claims the same outcome as story:instance-scoped-tasks 'Every task resolves sessions by the active instance's full names' (instance-scoped-tasks.md:32), so both will be marked done and one will be a lie; the body must say which story owns it or narrow its claim"
```
