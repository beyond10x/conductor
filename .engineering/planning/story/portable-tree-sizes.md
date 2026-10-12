---
format: aep.planning-md/3
id: story:portable-tree-sizes
kind: story
status: implemented
title: The dashboard measures tree sizes without GNU du
tags:
- session-review
relations:
- decomposes: epic:disk-and-gate
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-12T19:14:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Found while replacing `df`: `crates/conductor/src/dashboard/waste.rs` (~299, ~402) calls `du -B1`,
GNU-only, which fails on BSD userland as `df -B1` did.

## Acceptance

- Sizes are summed in Rust (walk the tree, `symlink_metadata`, count each inode once), or with a
  portable `du -k`; nothing in `crates/` spawns `du -B1`.
- A test compares the result with a known tree.

## Scope

`crates/conductor/src/dashboard/waste.rs`, `crates/conductor/tests/dashboard.rs`.
