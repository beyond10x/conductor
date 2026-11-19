---
format: aep.planning-md/3
id: story:daily-tick-logged
kind: story
status: active
title: The loop logs each daily tick, a skipped one with its cause, and a refused records commit with the check's findings
summary: runtime.log says why a daily tick is missing or its commit refused
refs:
- provider: github
  reference: beyond10x/conductor#212
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/records.rs
- confidence: cited
  path: crates/conductor/tests/run_records.rs
- confidence: cited
  path: spec/domains/session.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:32:00Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-19T18:32:00Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #212. An instance had no `daily` tick for three days and its runtime log said nothing about
it; two instances' daily ticks ended `records-commit-refused` while the log held no line naming
`records check`'s findings. The loop wrote a line for a daily tick only when it returned an error
(`crates/conductor/src/run.rs:355`), and the refused commit only into the dispatch log
(`crates/conductor/src/run/dispatch.rs:491`).

## Outcome

- The loop logs each daily tick (standard streams, a runtime's `runtime.log` or the journal):
  `conductor run: daily tick <day>: ran, tick <id>`, or `... not run: <error>`.
- Before a daily tick whose last one ended before the day it covers ([`DAILY_LAST`]), one line
  names the last day and the cause: the loop did not reach `cadence.daily` on the days between
  (stopped, yielding its lock to another run, or a tick held past the day) (`run::daily_missed`).
- A refused records commit of the daily tick logs `conductor run: daily tick <day>: records commit
  refused at <step>: <what it printed>`, `records check`'s findings among them
  (`run::records::Committed::refusal`).
- Spec: `conductor.session.RunDaemon` in `spec/domains/session.yaml`.

## Acceptance (fail on main)

- `crates/conductor/tests/run_records.rs` `a_daily_tick_s_refused_commit_logs_the_check_s_findings`.
- `crates/conductor/tests/run_records.rs` `a_skipped_daily_tick_is_logged_with_its_cause`.

## Boundaries

Logging only: the daily tick's schedule and steps do not change, and a missed day is not run
afterwards.
