---
format: aep.planning-md/3
id: story:box-memory-default
kind: story
status: implemented
title: A box's default memory fits the controllers it runs, and doctor says when it does not
summary: The 8 GiB default memory_max of a box is derived from controllers.max_working, or doctor reports a box whose cap is below what its controllers need
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#195
relations:
- decomposes: epic:architecture-next
scope:
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: inferred
  path: spec/domains/config.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T11:48:53Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-18T11:48:54Z", actor: "human:unknown", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-19T17:02:27Z", actor: "human:unknown", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #195 (agent box friction F42, memory part). A box with `instances[].limits.memory_max` unset,
`controllers.max_working: 8`, ran 8 Claude Code sessions and one full `cargo test`; the cgroup's
`memory.max` was 8589934592, and the OOM killer took the runtime and all 8 controllers. The operator
set `limits: {memory_max: 24G}`. (The records left `Running` are
`story:controllers-reconciled-with-live-sessions`, fixed in 1.13.1.)

Read at 1.14.4 and on the wave 56 branch:

- `DEFAULT_MEMORY` is 8 GiB (`crates/conductor/src/placement.rs:2472`), applied when `memory_max` is
  unset and bounded by substrate's `MAX_EXEC_MEMORY_BYTES` (`placement.rs:2590-2599`). It is the same
  for one controller and for eight.
- The specification says the opposite: an absent key "sets no limit"
  (`spec/domains/config.yaml:311-314`).
- `conductor doctor` says nothing of a box's memory against its `max_working`.

## Outcome

- With `memory_max` unset, a box's cap is derived from `controllers.max_working`: 2 GiB per
  controller plus 8 GiB for the runtime and one build (24 GiB for 8 controllers, 12 GiB for 2),
  bounded by substrate's `MAX_EXEC_MEMORY_BYTES` and the host's memory. The formula is written in the
  `Limits` documentation of `spec/domains/config.yaml`, which also stops saying "no limit".
- An explicit `memory_max` below that value gets a doctor row, not failing:
  `box <instance> memory: <n>G for <k> controllers; set limits.memory_max to at least <m>G`.

Decision (class C, controller, 2026-11-18): the issue offers a derived default or a doctor row; both
are taken, the derived default only where the key is unset, so an operator's explicit cap is kept and
merely reported. Undone by setting `limits.memory_max`.

## Acceptance (tests that fail on the current code)

- `tests/placement.rs` `an_unset_memory_max_is_derived_from_max_working`: `max_working: 8` and no
  `memory_max` give a box request of 24 GiB, not 8.
- `tests/doctor.rs` `doctor_names_a_box_memory_cap_below_its_controllers_share`.

## Boundaries

- CPU and process limits keep their defaults.
