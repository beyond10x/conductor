---
format: aep.planning-md/3
id: story:spawn-session-id
kind: story
status: active
title: spawn records the session id, and a dispatch without a priority runs first
relations:
- decomposes: epic:loop-without-handwork
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T18:32:09Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T18:32:09Z", actor: "human:timo", revision: 3}
---
## Why

Conductor reported on v0.10.0 (2026-10-13): `conductor spawn` started a controller correctly but
recorded the whole `claude --bg` banner (five lines) as the `sent` line's `session_id`, and
`dispatch send-dispatch` refused a dispatch without `--priority` although its help does not mark the
flag required.

## What

- `conductor spawn` records the id of the banner's `backgrounded · <id> · <name>` line, or the
  output itself when it is one word.
- `dispatch send-dispatch` takes priority 0 (runs first) when `--priority` is not given.

## Acceptance

- `crates/conductor/tests/spawn_session_id.rs`: the banner gives its id; a bare id is read as is.
- `crates/conductor/tests/dispatch.rs`: a dispatch missing any other required flag is still refused
  naming it; `--priority` is no longer among them.

## Scope

`crates/conductor/src/spawn.rs`, `crates/conductor/src/dispatch.rs`,
`crates/conductor/tests/spawn_session_id.rs`, `crates/conductor/tests/dispatch.rs`.
