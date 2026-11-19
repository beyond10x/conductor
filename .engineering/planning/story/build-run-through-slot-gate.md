---
format: aep.planning-md/3
id: story:build-run-through-slot-gate
kind: story
status: active
title: build run admits a boxed build through slot gate, with one slot ledger
summary: Under SLOT_GATE, conductor build run takes its CPU share from slot's FIFO jobserver and does not count a second build slot of its own
tags:
- from-finding
relations:
- decomposes: epic:disk-and-gate
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: inferred
  path: crates/conductor/tests/build_run.rs
- confidence: cited
  path: docs/design/slot-integration.md
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T09:36:29Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-18T09:36:29Z", actor: "human:unknown", revision: 3}
---
## Why

Agentbox friction log F40 (2026-11-15). Builds in the box ran outside the host's `slot` pool.
slot 0.17.0's client needs a Unix socket or a host TCP port, and the box has neither. slot
0.18.0 added `slot gate`, a FIFO jobserver handed into the box while the build stays there
(agentbox !32). But `conductor build run` still admits a build with its own build slots and
job share (`crates/conductor/src/build.rs:2086-2105`, the grant loop about `:842-1110`). A box
build is therefore counted twice: once in conductor's host ledger and once in slot's pool.

`docs/design/slot-integration.md` is a proposal agreed on 2026-11-12. It says "No store record
yet" and predates `slot gate`.

## Outcome

When `SLOT_GATE` names a gate, `build run` runs the command through `slot run` on that gate.
It takes no conductor build slot and sets no job count of its own. It keeps its disk
reservation and watchdog. The design document's § 9 questions this touches are answered in
it.

## Acceptance

- With `SLOT_GATE` set to a fake gate, `build run -- cargo build` runs the command under the
  gate and records a disk grant with no conductor build slot. Without it, the existing slot
  path is unchanged. Both are held in `tests/build_run.rs`.
