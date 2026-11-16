---
format: aep.planning-md/3
id: story:box-memory-default
kind: story
status: draft
title: A box's default memory fits the controllers it runs, and doctor says when it does not
summary: The 8 GiB default memory_max of a box is derived from controllers.max_working, or doctor reports a box whose cap is below what its controllers need
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: website/docs/reference/config.md
revision: 1
---
## Why

Agentbox friction log F42 (2026-11-16 00:42). `instances[].limits.memory_max` was unset, so
the box got 8 GiB (`DEFAULT_MEMORY`, `crates/conductor/src/placement.rs:2355-2356`, applied at
`:2474-2481`). 8 Claude sessions plus one full `cargo test` passed that. The OOM killer took
the runtime and all 8 controllers, and the records stayed `Running` (see
`story:controllers-reconciled-with-live-sessions`). The cgroup's `memory.max` was 8589934592.
The fix was `limits: {memory_max: 24G}`. The default is the same for 1 controller and for 8.

## Outcome

When `memory_max` is unset, a box gets a cap that grows with `controllers.max_working`, with a
documented per-controller share and a build allowance, up to substrate's
`MAX_EXEC_MEMORY_BYTES`. Doctor reports an explicit cap below that share.

## Acceptance

- With `max_working: 8` and no `memory_max`, the box request's memory is the documented
  formula's value, not 8 GiB. An explicit cap below it gets a doctor row. A placement test
  and a doctor test hold both.
