---
format: aep.planning-md/3
id: story:reclaim-scratch-class-c
kind: story
status: active
title: host reclaim frees stale scratch and caches itself
relations:
- decomposes: epic:disk-protocol
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T17:07:18Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T17:07:18Z", actor: "human:timo", revision: 3}
---
## Why

Rules item 122 (the operator, 2026-10-14: "not a decision I want to take, it's obvious"): freeing
disk is class C, stale scratch and caches included. `conductor host reclaim` and `conductor build
run` still raise a class-H item ("only a person may remove what is left") when what is left is
scratch or a cache.

## Acceptance

- `host reclaim` deletes a stale scratch directory or cache (no live process holds a file in it,
  no uncommitted git work inside, untouched for `thresholds` age) and prints one line per deletion
  with its bytes; it never deletes a `target/` directory, a tree a session holds or a directory
  with uncommitted git work; tests in `crates/conductor/tests/` over fixtures hold each case.
- A shortfall left only by such paths raises no class-H item; one left by anything else still does.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/src/gc/reclaim.rs`, `crates/conductor/src/build.rs`,
`crates/conductor/tests/build_run.rs`, `crates/conductor/tests/gc.rs`.
