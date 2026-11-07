---
format: aep.planning-md/3
id: story:build-shared-target-dir
kind: story
status: draft
title: build run gives each repository one target directory, shared by its trees under a lock
summary: 12 per-tree target directories held 77 GB under the disk floor
revision: 1
---
## Why

A host monitor, 2026-11-07 13:49Z: 12 `target/` directories held 77 GB, all in use (one repository: 5 trees, 16 GB; another: 3 trees, 27 GB; a third: 25 GB under one `conductor build run -- cargo clippy`). Free space on `/` was 89 to 95 GB against a 120 GB floor from 13:29Z. Most of each tree's `target/` holds the same dependency artifacts.

The risk: `AGENTS.md` (Worktrees and builds) forbids `CARGO_TARGET_DIR` because a shared target can run another tree's test binaries and report them green. Dependency artifacts are hashed by source, so they do not collide; the uplifted binaries (`target/debug/<bin>`, which `CARGO_BIN_EXE_<bin>` points at) do. Cargo's own lock on the target directory serialises concurrent builds of one repository, but only per cargo invocation: tree A builds, tree B builds and overwrites `target/debug/<bin>`, then A's tests run B's binary.

## Acceptance

- `conductor build run` sets `CARGO_TARGET_DIR` to one directory per repository (under the instance's cache, keyed by the repository's origin), shared by all its trees; a variable the caller set is kept and named on standard error.
- `build run` holds a per-repository lock for the whole command, not only cargo's build step, so a test run never executes a binary another tree uplifted; concurrent runs of one repository queue, and the queue position is printed once.
- A test with two trees of one fixture repository whose binary prints its tree path runs `build run -- cargo test` in both at once; each test reads its own tree's path.
- `conductor gc` reclaims a shared target with no build running, and the per-tree `target/` of a tree that ran through `build run` stays empty.
- `AGENTS.md` Worktrees and builds says `build run` owns the target directory and why the lock makes it safe.
- Reading on the wave page: total `target/` bytes on the host before and after.
