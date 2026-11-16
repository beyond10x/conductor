---
format: aep.planning-md/3
id: story:disk-recovery-unblocks-dispatch
kind: story
status: draft
title: A dispatch blocked on disk is unblocked when free disk recovers
summary: When the disk item of a fall is withdrawn, each dispatch blocked on that disk shortfall is reported unblocked and its controller is told or resumed
tags:
- from-finding
relations:
- decomposes: epic:disk-protocol
- informed_by: story:disk-item-per-fall
scope:
- confidence: inferred
  path: crates/conductor/src/build.rs
- confidence: inferred
  path: crates/conductor/src/host.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: inferred
  path: crates/conductor/src/run/dispatch.rs
revision: 1
---
## Why

Agentbox friction logs F44 and F45 (2026-11-16):

- F44: a backend controller reported `blocked disk` at 02:58. The disk was freed at 03:00
  (101G free), and the controller stayed blocked until 03:5x, with no recheck.
- F45: stopping that controller did not bring it back. The cycle respawns only for open,
  unblocked dispatches, so the operator also had to run `dispatch report-unblocked` for
  DSP-20261115-01 at 04:2x.

`story:disk-item-per-fall` (implemented) withdraws the operator's disk item once free disk is
back above `disk_clear`. Nothing does the same for the dispatches the fall blocked.

## Outcome

When a fall's disk item is withdrawn, each dispatch blocked on that disk reason is reported
`unblocked`. Its live controller gets one message saying so. With no live controller, the next
cycle starts one as for any open dispatch.

## Acceptance

- A dispatch reported `blocked disk` during a fall is `unblocked` once free disk passes
  `disk_clear`, and its controller is messaged or restarted within one cycle. A run test with
  a fake disk reading holds it.
