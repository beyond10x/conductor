---
format: aep.planning-md/3
id: story:disk-escalation
kind: story
status: active
title: A disk shortfall becomes one operator item; conductor stops granting slots by message
relations:
- decomposes: epic:disk-protocol
- depends_on: story:build-run
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 3}
---
## Why

`epic:disk-protocol`, step 4: when the binary cannot free enough, a person must act, and today
nobody is told until a session notices. Conductor also grants build slots by message, which the
protocol replaces.

## What

- A refused reservation and a reclaim that falls short each record one operator item through the
  decision store (`decision hands-todo` lists it): the shortfall, what is left by path and size,
  and the one command only a person can run; one item per fall, not per refusal.
- The watch prints `disk floor not reached: <n>G short` once per fall.
- `.agents/conductor.md`: build slots are no longer granted by message; conductor reads `conductor
  host grants` and answers a `[RESOURCE … disk]` with "use conductor build run". `.agents/repo-
  controller.md` and `.agents/conductor-dev.md`: every build through `conductor build run`.
- `rules.md` item 113 points at the protocol.

## Acceptance

- `crates/conductor/tests/build_run.rs`: a shortfall records one hands-todo item with the numbers;
  a second refusal in the same fall records none.
- A watch test: `disk floor not reached` once per fall.
- A profile test holds the three profile sentences.

## Scope

`crates/conductor/src/build.rs`, `crates/conductor/src/watch.rs`, `crates/conductor/src/decide.rs`
(the operator item), `crates/conductor/tests/build_run.rs`, `crates/conductor/tests/watch.rs`,
`.agents/*.md`, `crates/conductor/tests/profiles.rs`.
