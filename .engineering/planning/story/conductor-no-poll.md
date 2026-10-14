---
format: aep.planning-md/3
id: story:conductor-no-poll
kind: story
status: implemented
title: Conductor never waits inside its own session
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Conductor waited inside its own session: a background wait for a pull request ran 1 h 31 min with
no output, and a `sleep 45` was blocked by the harness (conductor's transcript, 2026-10-13).
`story:controller-no-poll` covers controllers only.

## Acceptance

- `.agents/conductor.md` says conductor never sleeps or polls on a pull request, a suite or a CI run
  in its session: its watch or the controller's report wakes it; `crates/conductor/tests/profiles.rs`
  holds the sentence.

## Scope

`.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
