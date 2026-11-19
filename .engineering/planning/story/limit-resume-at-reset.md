---
format: aep.planning-md/3
id: story:limit-resume-at-reset
kind: story
status: implemented
title: A limited session resumes at the reset its limit line names, with no ais
summary: Parse the 5-hour and weekly reset, resume at it, back off on a second limit, free the max_working place of a dead limited session
refs:
- provider: github
  reference: beyond10x/conductor#205
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/dashboard/sessions.rs
- confidence: cited
  path: crates/conductor/src/limit.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-19T23:49:59Z", actor: "human:unknown", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #205 (friction F58, F59). A limited session is resumed only once `ais status --json` reports
headroom (`run.rs` `Ais`, `run/handlers.rs` `limited`); without `ais` every tick errs and nothing
resumes. The limit line's reset time (`resets 9:40pm (UTC)`, `resets Nov 20, 11am (UTC)`) is parsed
only by the dashboard (`dashboard/sessions.rs` `reset`), which misses the weekly form, and
`token::limit` always sets now + 5h. Reconcile (`run/dispatch.rs` `lives_or_waits`) keeps a limited
session whose process is gone `Running`, holding a `max_working` place.

## Outcome

- One reset parser (`crates/conductor/src/limit.rs`) reads both forms; the dashboard and the loop use it.
- `limited.json` keeps each session's `resets_at` and `attempts`; at each tick a session whose reset
  has passed is resumed with no probe. A second limit after such a resume backs off (5 min, doubling,
  at most 1 h, never before a newly parsed reset); with no parsable reset the token's limit window applies.
- `ais` (`usage.probe`) is optional: missing, it is "unknown", not an error, and resume at reset goes on.
- A limited session whose process is gone gives its `max_working` place back (its record stops with
  reason `usage limit`) and is resumed from its transcript at the reset.
- `token::limit` takes `limited_until` from the parsed reset.
- The watch reports a limit text again for a session resumed since the last report.

## Acceptance (tests that fail on main)

- `limit.rs` `a_weekly_reset_with_a_date_parses`, `a_daily_reset_parses`.
- `tests/run.rs` `a_limited_session_is_resumed_at_the_reset_its_line_names_without_a_headroom_probe`.
- `tests/run.rs` `a_second_limit_after_a_resume_at_reset_backs_off`.
- `tests/run.rs` `a_cycle_stops_a_limited_session_whose_process_is_gone_and_resumes_it_at_reset`.
