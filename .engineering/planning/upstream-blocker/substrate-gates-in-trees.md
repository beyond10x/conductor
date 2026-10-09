---
format: aep.planning-md/3
id: upstream-blocker:substrate-gates-in-trees
kind: upstream-blocker
status: open
title: Substrate cannot run a gate in a managed worktree at full speed
relations:
- blocks: story:substrate-gate-provider
- blocks: story:substrate-tool-server
revision: 1
---
## What would clear it

All three, each observed in docs/analysis/substrate-tool-spike.md:
- an exec may use more than one core (`cpu.max` is clamped to one period,
  `crates/substrate-host/src/process.rs:3056-3074`; a cold `cargo test -p conductor-cli` took
  184.7 s wall on 1 of 20 cores);
- an adopted linked worktree can reach its repository's common git dir through a declared bind
  (`git status` exits 128 today);
- the in-process host keeps `.substrate-apertures` out of the adopted directory's parent.

Owner: the substrate repository; asked in conductor-dev's request CD-20261009-01 to conductor, 2026-10-09.
