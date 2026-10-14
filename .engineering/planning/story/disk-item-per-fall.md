---
format: aep.planning-md/3
id: story:disk-item-per-fall
kind: story
status: implemented
title: Refused builds in one fall of free disk share one operator item, withdrawn when disk recovers
relations:
- decomposes: epic:disk-protocol
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T02:14:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T02:14:58Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T02:15:04Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`story:disk-escalation` promises one operator item per fall of free disk, not one per refusal. On
2026-10-13 between 21:06Z and 23:05Z, conductor's decisions log gained seven class-H disk items,
two of them two minutes apart, each keyed to one refused build command and with a different
repository field. All seven stayed open in `conductor decision hands-todo` after about 17G had been
freed.

## Acceptance

- Two refusals in one fall, for different commands and repositories, record exactly one hands-todo
  item; a test in `crates/conductor/tests/build_run.rs` holds it.
- That item is withdrawn on its own once free disk is back above the floor (`disk_clear`); the same
  test file holds it.

## Scope

`crates/conductor/src/build.rs`, `crates/conductor/src/host.rs`, `crates/conductor/tests/build_run.rs`.
