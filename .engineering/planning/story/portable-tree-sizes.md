---
format: aep.planning-md/3
id: story:portable-tree-sizes
kind: story
status: draft
title: The dashboard measures tree sizes without GNU du
tags:
- session-review
revision: 1
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
