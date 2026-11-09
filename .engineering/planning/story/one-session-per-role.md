---
format: aep.planning-md/3
id: story:one-session-per-role
kind: story
status: active
title: 'A role runs one session: start refuses a second, the watch reports one'
summary: two maintainer sessions ran at once, the second named -2
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/confine.rs
- confidence: inferred
  path: crates/conductor/tests/profiles.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:50:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:50:17Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07: "why there are now two beyond 10x conductor dev sessions ... make sure this doesn't happen again". On 2026-11-07 a maintainer session was kept alive for its three sub-agents and a successor was started beside it with a bare `claude --bg -n <name>`; the broker does not refuse a held name and named the successor `<name>-2`. The same day a `claude --bg --resume <id>` kept its old name beside a running session of that name (two teacher sessions). `conductor session start` refuses only an exact name match (`crates/conductor/src/session.rs:1074`, `live_role_session`).

## Acceptance

- `conductor session start --role <role>` refuses with `SessionRunning` when a live session holds the role's name or that name followed by `-<digits>`; a test over a fake session list covers both.
- The watch reports, once per tick while it holds, a role with two or more live sessions whose names are `<name>` or `<name>-<digits>`, naming each id, as a fault the loop sends to the escalation session.
- The shipped supervisor profile (`story:host-session-profiles`) starts conductor-dev only through `task maintainer:start`, and stops before it starts.
