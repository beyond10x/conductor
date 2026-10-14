---
format: aep.planning-md/3
id: story:wave-width
kind: story
status: implemented
title: Conductor sees a wave narrower than its ready units
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T04:34:29Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator asked on 2026-10-14 for wider, concurrent waves (rules item 66). Conductor cannot see
an under-filled wave: nothing it reads gives a wave's width against the ready units with disjoint
scope and `controllers.max_subagents`.

## Acceptance

- Spec first: a repository's snapshot row records the width of its newest wave (units started
  together) and the ready units the store's waves offered then; the board flags `narrow wave` when
  the width is under both the ready count and `controllers.max_subagents` and the wave page names no
  reason; a test in `crates/conductor/tests/collect_stores.rs` over a fixture store holds it.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/collect/repositories.rs`,
`crates/conductor/src/status.rs`, `crates/conductor/tests/collect_stores.rs`.
