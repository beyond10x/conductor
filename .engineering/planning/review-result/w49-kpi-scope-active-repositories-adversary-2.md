---
format: aep.planning-md/3
id: review-result:w49-kpi-scope-active-repositories-adversary-2
kind: review-result
status: active
title: Wave 49 U6 adversary, pass 2
relations:
- reviews: story:kpi-scope-active-repositories
revision: 1
---
unit: story:kpi-scope-active-repositories, branch impl/kpi-scope-active-repositories at cda0f0688 plus two uncommitted cases in w49-u6
verdict: NEEDS-CHANGE
cases: executed 1613→1615, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 logs in the unit's scratch adv2/ directory
needs-coordinator: yes. How a past day's activity is decided once retention dropped its observations, and whether a repository the newest snapshot did not observe counts.

Cases added in crates/conductor/tests/metric.rs, both red:
- adversary_a_past_day_beyond_retention_keeps_the_repositories_active_then: the 10-14 23:00
  snapshot observes alpha Active; twelve later snapshots on 10-30 observe it Inactive, so the
  default retention.snapshots 12 drops the 10-14 observations; compute --day 2026-10-14 must give
  alpha K4.median 3 h.
- adversary_a_repository_the_newest_snapshot_of_the_day_does_not_observe_has_no_reading: 10-14
  10:00 observes alpha and beta, 20:00 observes beta only; `repository activity` has no alpha
  row (checked, passes); compute must give alpha no K4.median.

Run alone: 2 tests run: 0 passed, 2 failed (metric.rs:1559:5 and metric.rs:517:32). Suite: 1615
run, 1612 passed, 3 failed (the two plus the build_run case that also fails on the base).

F1 (repository.rs:92, against the doc at metric.rs:83): retention keeps the newest 12 complete
snapshots' observations, about a day at the default cadence; a day computed later than that has
no observation from before its end and the fallback reads the oldest retained snapshot. Reached
by a late `metric compute --day` (roles/supervisor.md:195). Fix: read the day's activity from
something retention keeps, or state the bound in the doc.

F2 (repository.rs:104-118): activity_on takes the newest snapshot before the end that observed
the repository, while activity uses only the newest complete snapshot; a repository missing from
the newest snapshot (a member failure, a collector over budget, a removed repository) has no row
in `repository activity` but compute counts it. The first commit 9fc13763b used activity() and
would pass. Fix: take the newest complete snapshot before the end and treat a repository it does
not observe as unobserved; fall back to a later snapshot only when none started before the end.

Could not break: ordering before/after the end, equal starts, the store check after activity_on.
Not probed: the exact day-end boundary (< against <=).

```findings
- file: crates/conductor/src/repository.rs
  line: 92
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the doc promises a past day computed later keeps the repositories Active then, but with the default retention of 12 snapshots the day's observations are gone after about a day and activity_on falls back to the oldest retained snapshot
- file: crates/conductor/src/repository.rs
  line: 104
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: activity_on counts a repository from an older snapshot when the newest complete snapshot of the day did not observe it, so compute gives readings to a repository that repository activity does not answer Active on that day
```
