---
format: aep.planning-md/3
id: story:build-run-queue
kind: story
status: implemented
title: The host build ledger grants requests in arrival order
relations:
- decomposes: epic:disk-protocol
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T04:34:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T04:34:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T04:34:26Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Conductor asked conductor-dev on 2026-10-13 23:09Z for a queue or priority in the host build ledger:
a 6G build waited from about 21:00Z to 00:45Z while 2G builds took every freed slot, and holds
asked for by message did not hold.

## Acceptance

- Spec first: requests are granted in arrival order; a 6G request made before two 2G requests is
  granted first when 6G frees, and the 2G requests wait behind it while the head does not fit; a
  test in `crates/conductor/tests/build_run.rs` holds it.
- A request that waits longer than one `thresholds` value names its position in the queue on
  standard error.

## Scope

`spec/domains/host.yaml`, `crates/conductor-model`, `crates/conductor/src/host.rs`,
`crates/conductor/src/build.rs`, `crates/conductor/tests/build_run.rs`.
