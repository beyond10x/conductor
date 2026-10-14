---
format: aep.planning-md/3
id: story:reclaim-foreign-entries
kind: story
status: draft
title: Scratch conductor may not delete still reaches the operator
relations:
- decomposes: epic:disk-protocol
revision: 1
---
## Why

`story:reclaim-scratch-class-c` (wave 36) counts a `/var/tmp` entry owned by another user as
scratch, so a shortfall left only by such entries raises no operator item, although conductor never
deletes them and only a person can.

## Acceptance

- A shortfall whose left paths include an entry conductor may not delete (owned by another user,
  or kept for any reason `kept_scratch` gives) raises the class-H item, naming those entries by
  count and size; one left only by entries conductor may delete raises none. A test in
  `crates/conductor/tests/build_run.rs` holds both.
- The staleness age is `thresholds.scratch_stale` (default one day), spec first, replacing the
  constant `gc::reclaim::STALE`; a config test holds the default.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `crates/conductor/src/host.rs`,
`crates/conductor/src/gc/reclaim.rs`, `crates/conductor/tests/build_run.rs`, `crates/conductor/tests/config.rs`.
