---
format: aep.planning-md/3
id: story:loop-tells-escalation-session
kind: story
status: active
title: The loop tells a session of the events it cannot handle
summary: refusals and over-threshold lines reach no session
scope:
- confidence: inferred
  path: conductor.example.yaml
- confidence: inferred
  path: crates/conductor-docs/src/status.rs
- confidence: inferred
  path: crates/conductor/src/agent/mod.rs
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/config/convert.rs
- confidence: inferred
  path: crates/conductor/src/config/effective.rs
- confidence: inferred
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 4}
---
## Why

Observed 2026-11-07 on an instance whose repositories are on a self-hosted GitLab: the run unit's
journal held 10 lines `no open dispatch of <repo>` between 09:26 and 11:11Z and a request refused
for its format, and no session was told; the host's supervisor now polls the journal every 20
minutes. The loop writes these refusals to standard error only (`run/mailbox.rs`), and its watch
lines go to its own handlers, not to a session.

## Acceptance

- At the watch tick that sees it, the loop sends one message to the session the instance's config
  names for escalations, for each of: a new mailbox request (decision request, need, blocked), a
  `context: … over threshold` line, `LaunchFailed`, `no open dispatch of <repo>`, a request that
  lists no lettered option, and a message refused for its format.
- The same event is sent once, not at every tick it stays true.
- With no such session configured the loop sends nothing and says so once at start.
