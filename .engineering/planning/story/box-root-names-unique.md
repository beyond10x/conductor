---
format: aep.planning-md/3
id: story:box-root-names-unique
kind: story
status: draft
title: Two instance names never share one box root name
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
revision: 2
---
## Outcome

Two instances whose names differ only in characters the root name replaces (for example `a.b`
and `a_b`) get distinct writable-root names in a box request, or the configuration is refused
with a named error before any box is started.

## Why

`crates/conductor/src/placement.rs` derives a substrate writable-root name from the instance
name by replacing characters substrate does not admit. Two instance names can map to one root
name, so one instance's state, records or cache root is sent for the other. Found by the
adversary of `story:box-runtime-starts` (wave 49, pass 1, finding 6); it was there before that
unit. The case `adv_box_mounts` holds for it is ignored until this story lands.

## Acceptance

Given a configuration with the instances `a.b` and `a_b`, both placed in a box, when conductord
builds the box requests, then the root names of the two instances differ or `conductor config
check` refuses the configuration naming both instances; the ignored case in
`crates/conductor/tests/adv_box_mounts.rs` runs and passes.
