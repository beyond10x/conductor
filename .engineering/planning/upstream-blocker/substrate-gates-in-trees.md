---
format: aep.planning-md/3
id: upstream-blocker:substrate-gates-in-trees
kind: upstream-blocker
status: open
title: Substrate cannot run a gate in a managed worktree at full speed
relations:
- blocks: story:substrate-gate-provider
- blocks: story:substrate-tool-server
revision: 2
---
## What would clear it

All three, each observed in docs/analysis/substrate-tool-spike.md:
- an exec may use more than one core (`cpu.max` is clamped to one period,
  `crates/substrate-host/src/process.rs:3056-3074`; a cold `cargo test -p conductor-cli` took
  184.7 s wall on 1 of 20 cores);
- an adopted linked worktree can reach its repository's common git dir through a declared bind
  (`git status` exits 128 today);
- the in-process host keeps `.substrate-apertures` out of the adopted directory's parent.

Owner: the substrate repository; asked 2026-10-09.

## Cleared

2026-10-10 by substrate 0.7.12 (https://github.com/beyond10x/substrate/releases/tag/0.7.12, its
CHANGELOG `## [0.7.12]`):
- the exec CPU ceiling is configurable: `HostConfig::exec_cpu_cores`, daemon `--exec-cpu-cores N`
  (the default stays 1; a consumer sets it);
- a linked worktree's Git common directory declared as a read-only root at its own host path lets
  `git status` and `git log` run inside an exec, writes to it refused;
- aperture run state no longer goes into the workspace root: `HostConfig::aperture_root`, and the
  daemon keeps it beside its state database.

`story:substrate-client` sets `exec_cpu_cores`, the common-directory root and `aperture_root` when it
adopts a managed tree.
