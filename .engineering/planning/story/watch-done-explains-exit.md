---
format: aep.planning-md/3
id: story:watch-done-explains-exit
kind: story
status: active
title: A dispatch's done line explains the exit of the session its sent line names
scope:
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 3}
---
## Why

The watch reports `session gone: <name> <id>` unless a dispatch-log line written in the last
`EXPLAINED_WITHIN` (15 min) names the session id or its pid (`crates/conductor/src/watch.rs`,
`explained`). Conductor logs a dispatch's `done` line after it stops the controller, and that line
names only the dispatch id; the session id is on the dispatch's `sent` line, which is often older
than 15 minutes by then. Observed in a live run: two `session gone` lines right after `done` lines,
each waking conductor for nothing.

## Acceptance

- A recent line of a dispatch explains the exit of a session that any line of the same dispatch,
  up to a month older, names by its session id.
- A pid links a dispatch only in a recent line; an older line naming only a pid does not.
- Tests: a `sent` line 60 min old naming session S and a `done` line 1 min old of the same
  dispatch print no `session gone`; a `done` line of another dispatch prints it; an old line naming
  only the pid prints it.
- Design § 7: a controller's SendMessage to a sub-agent is checked by the shape of the id, not by
  who started the agent.

## Scope

`crates/conductor/src/watch.rs` (cited), `crates/conductor/tests/watch.rs` (cited),
`docs/design/conductor.md` § 7 table (cited).
