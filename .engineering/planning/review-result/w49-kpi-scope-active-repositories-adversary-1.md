---
format: aep.planning-md/3
id: review-result:w49-kpi-scope-active-repositories-adversary-1
kind: review-result
status: active
title: Wave 49 U6 adversary, pass 1
relations:
- reviews: story:kpi-scope-active-repositories
revision: 1
---
unit: story:kpi-scope-active-repositories, branch impl/kpi-scope-active-repositories at 9fc13763b plus two uncommitted cases in w49-u6
verdict: NEEDS-CHANGE
cases: executed 1610→1612, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 logs in the unit's scratch adv/ directory
needs-coordinator: yes. The story says activity "on the reading's day" and the code uses activity at the time compute runs.

Cases added in crates/conductor/tests/metric.rs, both red:
- adversary_a_reading_counts_the_repositories_active_on_its_day (line 1435): a complete snapshot
  of 10-14 shows alpha Active, a later one (10-30) shows it quiet; `metric compute --day
  2026-10-14` must read K5.unreleased 1 for alpha and the instance, and K4.median 3 h.
- adversary_activity_comes_from_the_snapshot_the_forge_is_read_from (line 1468): two complete
  snapshots with the same start, first Active, second Inactive; alpha must have a K5 reading.

Red run of the two alone: 2 tests run: 0 passed, 2 failed (panicked at tests/metric.rs:517:32,
no reading of K5.unreleased for alpha). Suite with them: 1612 run, 1609 passed, 3 failed (the two
plus the build_run timing case); without them 1610 run, 1609 passed, 1 failed (build_run).

F1 (metric.rs:811): compute takes the Active set as it stands when compute runs. A past day
computed later (the supervisor's documented recovery, roles/supervisor.md:195, and the daily tick
in run/dispatch.rs:202) drops a repository that was Active then. Readings are kept on a second
run, so nothing repairs it. Base has no filter, so the defect is this unit's. Fix: take activity
from the newest complete snapshot that started before day.end; marks stay timeless.

F2 (metric.rs:80): on equal snapshot starts Forge::read (metric.rs:1444) keeps the first stored and
repository::activity (repository.rs:111) the last stored, against the doc. Nothing found reaches
it; a shared newest-complete helper or a doc line closes it.

Could not break: mutants dropping each of the four filters are caught; the instance sums Active
scopes for K4, K5, K6, K12; no_releases still leaves an Active repository out of K5; an
unobserved unmarked repository gets no reading; naming is consistent across observations, pulls
and releases.

```findings
- file: crates/conductor/src/metric.rs
  line: 811
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: compute filters by repository activity at the time it runs, not on the reading's day as the acceptance requires, so computing a past day drops a repository that was Active then and later went quiet
- file: crates/conductor/src/metric.rs
  line: 80
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the doc says activity comes from the snapshot the forge is read from, but on equal snapshot starts Forge::read keeps the first stored and repository::activity the last stored
```
