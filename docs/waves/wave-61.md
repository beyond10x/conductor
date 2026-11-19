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
| U3 | need-starts-unit-of-work | #207 | in progress |

## Outcome

U1 adds `controllers.unit_merge` (`controller`, `human`; default `human`) and a second compiled
default charter ending `merge: controller`; a charter equal to either default form follows the key
at the next unit start. The key enters the configuration every pod renders, so each Kubernetes pod
is replaced once at the upgrade. The dashboard's settings page offers no choice for the key yet.

U2 forgets a box substrate answers as gone and opens a new one; the first failed start of a
recreated box alerts once (`box-not-recreated`).
