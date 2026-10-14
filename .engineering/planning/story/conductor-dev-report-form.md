---
format: aep.planning-md/3
id: story:conductor-dev-report-form
kind: story
status: implemented
title: conductor-dev replies to a NEED in the report form of design section 6
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T02:14:57Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T02:14:58Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T02:15:03Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

conductor-dev's replies to conductor broke the message protocol twice on 2026-10-13: one first line
had no verb and was recorded as rejected, another used the prefix `[REPORT conductor-dev -]`.
`.agents/conductor-dev.md` gives only the `ready` form.

## Acceptance

- `.agents/conductor-dev.md` gives the reply to a `[NEED conductor <id>]` as `[REPORT conductor -]
  <started|progress|blocked|unblocked|done|failed> <id>: <line>`: the dispatch-id slot is `-`, the
  NEED id goes in the line (conductor's correction, 2026-10-14); `crates/conductor/tests/profiles.rs`
  holds it.

## Scope

`.agents/conductor-dev.md`, `crates/conductor/tests/profiles.rs`.
