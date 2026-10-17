---
format: aep.planning-md/3
id: review-result:scope-attach-tui-quickstart-r2
kind: review-result
status: active
title: Scope critic, round 2 (session-attach, tui-frontend, quick-start)
relations:
- reviews: epic:session-attach
- reviews: epic:tui-frontend
- reviews: epic:quick-start
- reviews: story:session-attach-command
- reviews: story:session-attach-watch
- reviews: story:session-attach-refuses-sessions
- reviews: story:session-list-attachable
- reviews: story:tui-overview
- reviews: story:tui-decisions-read
- reviews: story:tui-decisions-answer
- reviews: story:tui-dispatches-activity
- reviews: story:tui-host-disk-builds
- reviews: story:tui-open-session
- reviews: story:install-one-liner
- reviews: story:demo-mode
- reviews: story:init-defaults
- reviews: story:quick-start-page
- reviews: story:quick-start-docs-test
revision: 1
---
approve

No findings. The round 1 scope findings all landed, and I found no new gap, reach, duplicate or narrowed promise. For the five findings:

- **Retarget of `task conductor:attach` and the guide's `claude attach <id>` pointer:** `story:session-attach-command` now claims it (AC8 and AC9).
- **TUI attach or watch:** `epic:tui-frontend`'s Outcome now claims it, and `story:tui-open-session` cites it.
- **`story:init-defaults`:** it is now the step after the demo, and `epic:quick-start`'s Outcome names it as off the five-minute path. The unused `depends_on` is gone.
- **Demo label:** `story:demo-mode` AC3 now requires the `demo` label on the session row, the dispatch row and the activity-feed event, and on `/api/events`.
- **Pre-release CI run:** `story:quick-start-docs-test` AC7 now runs the check on tag push in `release.yml`, before publish. AC3 states that it does not run on `release: published`.

What I read: 3 epics, 16 stories, initiative:adoption, `review-result:scope-attach-tui-quickstart-r1`, `story:watch-stalled-sessions` and `story:dashboard-build-queue`. I ran `aep plan artifact show`, `graph`, `list`, `kinds` and `relations`, and wrote each epic's promise list before reading its stories.

Promises extracted and traced to a story:
- `epic:session-attach`: 10 extracted, 10 traced.
- `epic:tui-frontend`: 11 extracted, 11 traced.
- `epic:quick-start`: 13 extracted, 13 traced.
- Total: 34 extracted, 34 traced.

What I could not establish:
- **History and grants (judgement call, not a finding):** `story:tui-decisions-read` adds the decision history, and `story:tui-host-disk-builds` adds the host grants list. The `epic:tui-frontend` Outcome names neither. I read both as covered by "shows what the dashboard shows" and by the build queue, so I did not flag them. If the operator wants the TUI strictly lean, these are the two to trim.
- **Guard-rule and Next-list items:** the guard rules in `story:tui-decisions-answer` AC6 and `story:demo-mode` AC7 are not named in their epics. I read them as safety consequences of a write. The "TUI once delivered" and confinement links in `story:quick-start-page` AC2's Next list are likewise unclaimed pointers. None changes my verdict.
- **Out of my lane (acceptance or design):** `story:quick-start-docs-test` AC3 puts tag-push runs in a "separate workflow file" while AC7 puts the tag-push run in `release.yml`. AC7 also relies on an `AGENTS.md` release-step update "by the coordinator" that no story claims.

```findings
[]
```