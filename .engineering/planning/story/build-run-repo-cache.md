---
format: aep.planning-md/3
id: story:build-run-repo-cache
kind: story
status: implemented
title: Build run gives each repository its own compile cache, shared by its worktrees
relations:
- decomposes: epic:disk-protocol
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T13:54:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T13:54:10Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T13:54:10Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: build-run-repo-cache

## Outcome

`conductor build run` gives each repository its own compile cache, shared by all of that repository's worktrees. A new tree stops recompiling every dependency, and no cache server is shared between repositories.

## Context

Measured on 2026-10-17 from the host ledger (`conductor host grants`), 08:46–11:34Z, while 3 build slots applied:
- Queue wait p50 12.5 min for runs whose p50 length is 0.7 min.
- In the same tree, the first run takes p50 8.1 min and later runs 1.1 min, so about 7 min is the cold penalty.
- External crates make up 75–100% of a fresh tree's first build.
- About 197 Rust worktrees were created in 24 h, each with an empty `target/`.
- About 12 slot-hours a day, roughly 17% of capacity, go to recompiling dependencies.

The filesystem is ext4, which has no reflink, so copying a warm target into a new tree is not an option. A shared `CARGO_TARGET_DIR` per repository is rejected for two reasons: the cross-tree correctness incidents recorded in several repositories' AGENTS.md, and cargo's build-dir lock, which would park a granted slot. A single global sccache server stalled builds on 2026-10-16, which is why the instance rules forbid a global wrapper. One server per repository, bounded by the slots, cannot recreate that stall: at most build_slots × jobs compilers run host-wide.

## Acceptance

1. Spec first: `thresholds.repo_cache` (`on`, `off`; default `on`) and `thresholds.repo_cache_size` (default `2G`) in `spec/domains/config.yaml`, validated with ess.
2. When the command is cargo-based (`cargo …`, or `task …`/`sh -c` running cargo, decided by `build run`'s existing command classifier or the first word), `build run` sets `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=<cache home>/sccache/<repo>`, `SCCACHE_SERVER_PORT=<port>` and `SCCACHE_CACHE_SIZE=<repo_cache_size>`, unless the environment already sets them. `<repo>` comes from `git rev-parse --git-common-dir` of the working directory, so every tree of a repository shares one cache. The port is derived stably from the repository name in a range that excludes 4226. Tests.
3. The repository's sccache server starts detached from the command's process group, with `SCCACHE_IDLE_TIMEOUT` set, so `build run`'s group signal never stops it and an idle server ends itself. Test.
4. If `sccache` is not on `PATH`, `build run` sets nothing and prints one line saying so; the build still runs. Test.
5. Each grant records the cache's hit and miss counts for its run (`sccache --show-stats` before and after), and `host grants --format json` shows them. Test.
6. Incremental compilation stays on for workspace crates, and the docs say that sccache skips incremental units.
7. Evidence: on this host, the first `cargo test -p <crate>` in a second fresh tree of a repository takes markedly less time than in the first. The story records both durations.

## Out of Scope

A shared target directory. Remote caches.

## Ambiguities

None.
