---
format: aep.planning-md/3
id: story:test-lists-from-spec
kind: story
status: draft
title: The tests derive the leaf and observation lists from the specification
summary: No hand list a new unit can forget
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

Wave 36's local gate ran three times: twice because a unit added a command and an observation kind and did not add them to hand-written lists (`crates/conductor/tests/adv_cli.rs` IMPLEMENTED, `crates/conductor/tests/observation_retention.rs` every_kind), which another unit had joined. Rules item 112 found the same in wave 18 (six CI runs).

## Acceptance

- The lists of implemented leaves and of observation kinds the tests check are derived from the compiled specification (`ess specify compile --format json`) and the handlers, not written by hand; adding a leaf or an observation kind in `spec/` without its handler or its recorder call fails one named test that says which; the two hand lists are removed.

## Scope

`crates/conductor/tests/adv_cli.rs`, `crates/conductor/tests/observation_retention.rs`, `crates/conductor/tests/common/`.
