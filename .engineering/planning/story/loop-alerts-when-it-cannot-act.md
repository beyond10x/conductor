---
format: aep.planning-md/3
id: story:loop-alerts-when-it-cannot-act
kind: story
status: implemented
title: The loop alerts the operator once when it cannot act
summary: One alert route (crate::alert, an alert line, the operator's notifier), deduplicated by condition and subject; first condition a unit left to the operator
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
scope:
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: inferred
  path: crates/conductor/src/alert.rs
- confidence: inferred
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: docs/design/conductor-run.md
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: inferred
  path: website/docs/reference
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T19:24:57Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-19T19:24:57Z", actor: "human:unknown", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-19T23:23:19Z", actor: "human:unknown", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Why

When the loop cannot act on one of #207's duties, nothing tells the operator; an operator session
found these by reading logs (#207). The loop already reaches the operator through the notifier
(`notify-send`, `crates/conductor/src/run/handlers.rs:13`, `notify` at `:1284`), which wave 56 kept
unconfigurable with a doctor row, but only for escalated decisions. A unit of work whose start
failed `FAILED_STARTS_MOST` times (`crates/conductor/src/dispatch.rs:325`) is "left to the operator"
in a `cycle` line only (`crates/conductor/src/run/dispatch.rs:342-372`).

## Outcome

One alert route, used by the loop and the daemon: a stuck condition writes one alert line, once
per condition and subject, recorded in the records and sent to the operator's notifier.

## Acceptance

- `conductor.dispatch.DispatchLogEvent` gains `Alert` (`spec/domains/dispatch.yaml:249-278`), its
  line carrying `condition`, `repo` (or the instance for an instance condition) and `reason`; spec
  first, then `task regen`.
- `crates/conductor/src/alert.rs` holds the route: `raise` records the alert unless the same
  condition and subject is already in its seen file (`<state>/run/alerts.seen` for the loop, the
  daemon's directory for the daemon), and sends `<instance>: alert` with the line through the
  notifier `run/handlers.rs` uses today, which moves there; a missing notifier is a journal line,
  never an error. A subject is forgotten from the seen file when its condition clears, so a
  second occurrence alerts again.
- First condition, `unit-left-to-operator`: the cycle that leaves a unit to the operator
  (`run/dispatch.rs:366-372`) raises one alert naming the dispatch and the repository.
- A test in `crates/conductor/tests/run.rs` that fails on main: a unit whose start fails over
  three cycles and then stays left over two more writes exactly one `alert` line in the
  dispatches log and calls the fake notifier once.

## Boundaries

No alert for a condition the loop acts on itself; no new config key or notification route; the
`escalated` and `unanswered` notifications keep their text.

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model` (regenerated), `crates/conductor/http.yaml`
(re-pinned `source_digest`), `crates/conductor/src/alert.rs` (new), `crates/conductor/src/lib.rs`,
`crates/conductor/src/run/handlers.rs`, `crates/conductor/src/run/dispatch.rs`,
`crates/conductor/tests/run.rs`, `docs/design/conductor-run.md`, `website/docs/reference/`
(regenerated).

## Notes

First of the epic: `story:unit-merge-policy`, `story:failing-start-recreates-box` and
`story:need-starts-unit-of-work` depend on it (the first for the regenerated crate, the other two
for `crate::alert`).
