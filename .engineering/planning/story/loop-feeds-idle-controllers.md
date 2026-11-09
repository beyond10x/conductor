---
format: aep.planning-md/3
id: story:loop-feeds-idle-controllers
kind: story
status: active
title: The loop feeds every idle controller at each watch tick
summary: An idle controller gets its next dispatch or a nudge at the watch tick, not two hours later at a cycle
scope:
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: inferred
  path: crates/conductor/src/dashboard/overview.rs
- confidence: inferred
  path: crates/conductor/src/decide.rs
- confidence: inferred
  path: crates/conductor/src/init.rs
- confidence: inferred
  path: crates/conductor/src/metric.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: docs/design/conductor-run.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T18:39:58Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T18:39:58Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "i dont see a good rate of shipping whats wrong, why are you not constantly improving things", and earlier the same day: "all modifications need to be persistent when generic, anything else should not even exist". At one reading that evening 7 of 12 controllers of one host were idle, 4 with no open dispatch. The loop sends a dispatch only at a cycle (`cadence.cycle`, 2 h) and never nudges an idle controller, so an idle controller stays idle until the next cycle. The host supervisor bridged it with a shell script outside the product.

## Acceptance

- At each watch tick, a controller session that is idle, has no open dispatch for its repository, and whose instance has a confirmed goal gets a dispatch: `send-dispatch` with the next id of the day, the charter's brief, priority 1, and the brief delivered to the session with its report header `[REPORT <repo> <dispatch-id>]`. An instance with no confirmed goal is named in the journal and skipped.
- An idle controller with an open dispatch gets one nudge per watch tick (dispatch id and brief), named in the run journal.
- The charter's default brief says: deliver the active stories in waves, release within the K5 target after the merge, and with no active story left draft the next ones from the repository's epics and findings (decompose, or reverse-engineer without a store), put them through the critic panel, activate those that pass, and continue.
- The dashboard and a KPI show idle controller-minutes per day, per instance and repository.
- Tests over fixtures: an idle controller with no dispatch gets one; one with an open dispatch gets a nudge; a busy one gets nothing; no confirmed goal is skipped and named.
- The host supervisor's feed script does the same by hand today; it is deleted once this ships (story:host-scripts-into-conductor).
