# Wave 61 (2026-11-19: who merges a unit, a gone box recreated, a NEED becomes a unit)

The sixth wave of the goal that fixes the friction the agent box filed. Its units forked from the
head of wave 60; the integration branch from `main` after wave 60 landed. The wave before is
[wave 60](wave-60.md).

Width: two units at a time (`controllers.max_subagents` is 2); U3 started when U2 finished. All
three stories build on wave 60's alerts. Rust builds run one at a time (one lock, one build per
box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | unit-merge-policy | #207 | merged into the wave branch |
| U2 | failing-start-recreates-box | #207 | merged into the wave branch |
| U3 | need-starts-unit-of-work | #207 | merged into the wave branch |

## Outcome

U1 adds `controllers.unit_merge` (`controller`, `human`; default `human`) and a second compiled
default charter ending `merge: controller`; a charter equal to either default form follows the key
at the next unit start. The key enters the configuration every pod renders, so each Kubernetes pod
is replaced once at the upgrade. The dashboard's settings page offers no choice for the key yet.

U2 forgets a box substrate answers as gone and opens a new one; the first failed start of a
recreated box alerts once (`box-not-recreated`).

U3 turns a `[NEED]` to a known repository with no charter of its own (none, an empty one or the
default one), no running controller and no open dispatch into a unit of work with the reference
`need-<repo>-<request-id>`; a `[NEED]` that names no repository of the instance alerts once
(`need-unroutable`). Three `run` cases that relied on a chartless repository keeping the NEED now
give it a charter. U2 and U3 each add an alert condition; the merge keeps both.

In the box: the specification and the HTTP binding validate and a fresh generation equals the
committed model; the suites that launch a session or bind a socket are proven by CI.
