# Wave 72 (2026-11-20: a controller waiting on a person is not restarted; a pause holds)

The seventeenth wave of the goal that fixes the friction the agent box filed: issue #258. It forks
from `main` after wave 70. The critic panel's result: `review-result:w72-critic-panel`.

Width: one unit. Both halves of #258 edit the same start path (`Tick::try_start`,
`restart_with`) and the same test file, and the only other ready story builds in wave 71.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | person-wait-and-pause-hold-restarts | #258 | merged |

## Outcome

A repository whose every open dispatch is `Blocked` with a reason that starts `person`, and for
which nothing is kept, gets no controller from any start the loop makes (`session gone`, a second
`session silent`, a hand-over, the cycle's reconcile, dispatch v1, a resume); the loop says so once
and keeps it in `<state>/run/person.json`. A decision answer, a `[NEED]` or its answer, a resource
answer, a reply, or the dispatch moving starts it again in the same watch tick. A watch line for
it (a pull request's checks, a merge, CI, red `main`) is kept, one per kind and subject, and
offered to its next controller, but starts none: each CI event restarting a controller that waits
on a person was the churn #258 reports. A `Paused` controller's repository is never restarted,
resumed or stopped by the loop, and a pause keeps its hand-over request and usage-limit mark. The
controller profile names the report form `blocked person: waits on <who> for <what>`.

An adversary review found that a watch line ended the wait, that kept lines piled up while a start
was refused, that a second delivery pass used up a live session's offers, that resumes ignored the
wait, that a pause dropped the session's loop state, and that two profile rewordings lost meaning;
each is fixed. Still open, named in the story: a pause ends the session before it records `Paused`.

In the box: clippy, the unit tests and the profiles, roles and docs suites; the run suite's cases
need a host that may create a Unix socket, and CI proves them.
