---
format: aep.planning-md/3
id: story:daily-tick-runs
kind: story
status: implemented
title: The daily tick runs at cadence.daily, also after a restart
summary: run-ticks shows no daily tick and no REPORT.md is written
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T09:28:04Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T09:28:04Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-07T11:11:23Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Observed 2026-11-07: `conductor session run-ticks` of a running instance holds only `watch` and
`cycle` ticks and no `daily` one, though `cadence.daily` is 08:07; the records hold no
`REPORT.md`, and the readings of 2026-11-06 were computed by hand with
`conductor metric compute --day 2026-11-06`. The loop was restarted several times that night
before 08:07; whether a restart resets the daily schedule is a suspect, not verified.

## Acceptance

- A loop running across `cadence.daily` records one `daily` tick and writes that day's readings and
  `REPORT.md`.
- A loop restarted before `cadence.daily` still runs the daily tick at `cadence.daily`; one started
  after it, with no daily tick recorded for the day, runs it at start.
