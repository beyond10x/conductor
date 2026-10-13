---
format: aep.planning-md/3
id: epic:loop-without-handwork
kind: epic
status: draft
title: Conductor runs a day without hand-kept records, needless restarts or blind waits
revision: 2
---
## Why

A review of 2026-10-13 (the operator: "derive some improvements for conductor by looking at the
todays sessions") read conductor's eight sessions, 26 controller sessions and the instance's
records. The day's largest costs were work a command should do: records typed by hand beside an
empty store, eight restarts that each re-read about 100k tokens, a watch that expires every 30
minutes and does not see pull-request CI or stalled runs, hand-overs re-typing the store, guard
denials of legitimate reads, one gate script per controller, and surveys by hand for stale
blockers and closable epics.

## Outcome

Conductor runs a day without hand-kept records, without restarts beyond one an hour, and without
waits nobody watches: the store writes the records, the watch sees every wait it is given, a
hand-over is drafted from the store, and the guard decides a write on where it lands.

## Stories

`story:records-from-commands`, `story:request-ids-allocated`, `story:decision-amend`,
`story:watch-pr-checks`, `story:watch-goal-health`, `story:watch-waits`,
`story:conductor-start-reading`, `story:handover-draft`, `story:guard-write-targets`,
`story:rules-show`, `story:gate-run`, `story:plan-hygiene`, `story:news-pending`,
`story:status-command`.

## Added 2026-10-13

`story:report-md-in-records`: conductor's run report at the records root was denied and written
elsewhere by hand; it belongs with the guard deciding a write on where it lands.
