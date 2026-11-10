---
title: Disk
sidebar_position: 6
description: How the conductor binary runs disk as a protocol in five steps (reserve, reclaim, watch, escalate, enforce), so that no build waits on a message and no session has to remember to free space.
lede: Every build, test or gate reserves its disk from one host ledger, and the binary frees, pauses, refuses and reports on its own.
source: docs/design/conductor.md sections 7 and 9; spec/domains/host.yaml; crates/conductor/src/build.rs, host.rs, gc/reclaim.rs, watch.rs, guard/rules.rs; crates/conductor/roles/controller.md, maintainer.md; crates/conductor/tests/guard_rules.rs, profiles.rs
---

Several sessions build on one machine, and a build that runs out of disk stops everyone's work.
Conductor grants no build slot in conversation. A session's part is one command, which wraps the
build, test or gate of the repository it works in:

```console
$ conductor build run -- task check
```

The `cargo` and `task` commands on this page are those of the repositories conductor manages, not
steps to install conductor.

## 1. Reserve

`conductor build run [--reserve auto|<size>] [--repository <repo>] -- <command>` asks the host
ledger for the command's expected peak. A size is whole GiB, `8G`. `auto`, also what no
`--reserve` means, is the largest peak the ledger recorded for the same repository and command in
the last 14 days plus 50 %, at least 1G, and 4G when none is recorded.

The ledger lives in the host directory, `host/` beside the config file, and is shared by every
instance of that file. A `--state-dir` does not move it. Its lock is `host.lock`. The binary grants
at once when free disk, less the live reservations, less the floor (`thresholds.disk_admit` plus
reclaim's margin), covers the size.

Requests are answered in arrival order. A request waits behind an earlier one even when it alone
would fit, unless every request ahead lacks more than the live reservations hold. Then one that
fits is granted past them. At most `thresholds.build_slots` (3) runs hold a grant at once, every
instance's together. A request waits for a slot and is never refused for it. A request records the
harness session it runs in, and is withdrawn, exit 2, when that session stops while it waits.

`conductor host grants` lists the ledger: each reservation, its command, its size and the peak it
measured.

## 2. Reclaim

When the size does not fit, `conductor host reclaim` frees what needs no decision, in order,
reading free disk after each step:

1. finished managed trees, through `worktree finish` and `worktree gc --apply`, then the cargo
   `target/` of stale ones, whose trees are kept;
2. the build output of each idle managed tree: no live session, no process and no write to its
   `target/` in it; then every other idle cargo target under the trees and the cache, oldest
   first: no process and no cargo lock holds it, nothing wrote it for 180 minutes (90 while less
   than 50G are free) and it holds no `.keeper-hold` file. A target of another user is kept; a
   process of another user is not seen. A target is moved aside as `.conductor-gc-<pid>-<count>`
   in its own directory and then removed, so a build that starts meanwhile loses nothing; one a
   reclaim that died left is removed by the next;
3. build output in worktree archives;
4. stale scratch and caches;
5. observations of snapshots past `retention.snapshots`.

A tree that is dirty or unmerged and untouched for `thresholds.tree_stale` (48 h) is stale: its
`target/` goes and it is listed `stale, kept`. Each step is recorded with the path it removed and
the bytes it freed. One reclaim walks the disk at a time, `host.reclaim.lock`. A tree a live
session works in keeps everything but an idle cargo target; a running gate's target, and the
instance's records, state, checkouts and trees are not touched, however a symbolic link leads
into them. A scratch entry that is a link is removed as a link. The watch also reclaims once each time free disk falls under `thresholds.disk_admit`.

## 3. Watch the run

The command runs in its own process group. Every `thresholds.watchdog_every` (10 s) a watchdog
reads free disk: under `thresholds.gate_stop` it stops the group and reclaims, at
`thresholds.gate_resume` it resumes it. The peak of the run's `target/`, else its working
directory, is recorded, and the reservation is released when the command exits or is killed.
`build run` exits with the command's status.

SIGTERM, SIGINT or SIGHUP to `build run` is forwarded to the command's whole process group. A
group with a process left 10 seconds later is killed. The run exits with 128 plus the signal. A
request still waiting is withdrawn (`Withdrawn` in `conductor host grants`), and its command never
starts.

### What `build run` sets

| for | sets | key |
|---|---|---|
| every command | `nice -n 19` and the idle I/O class; `CARGO_BUILD_JOBS` and `MAKEFLAGS=-j`: the cores less the reserve, over the build slots, at least 1; a higher `CARGO_BUILD_JOBS` in the environment is lowered to it | `thresholds.cpu_reserve` (2) |
| a cargo-based command | `CARGO_TARGET_DIR`: the repository's one target, `<cache>/target/<key>`, held under its lock for the whole command; a value the environment sets is kept and takes no lock | |
| a cargo-based command | `RUSTC_WRAPPER=sccache`, the repository's `SCCACHE_DIR` and `SCCACHE_SERVER_PORT` (4300–5299), one server per repository that ends itself when idle; without `sccache` on `PATH`, nothing, and one line says so | `thresholds.repo_cache`, `repo_cache_size` (2G) |
| a cargo-based command | `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` set to `line-tables-only`, unless the environment or the manifest sets `debug` | `thresholds.lean_debuginfo` |
| a short command | a short-lane slot beside the regular ones, at most 4 jobs: at least 3 runs in 14 days with a p90 at most `short_lane_max`; a run over twice that is marked `lane_overrun` and leaves the lane for 24 hours | `thresholds.short_lane_slots` (1), `short_lane_max` (3m) |

## 4. Escalate

When reclaim cannot reach the size, `conductor build run` prints the shortfall and what is left by
path and size, exits 2, and does not start the command. It records one operator item, a class-H
request that `conductor decision hands-todo` lists, with the one command only a person can run.
Each instance records one item per fall of free disk, `falls/<prefix>.json` in the host directory.
The fall ends when free disk is back at `thresholds.disk_clear`, which withdraws the item. A
shortfall that only scratch and caches leave records no item. The watch prints `disk floor not
reached: <n>G short` once per fall.

A session that is refused reports the refusal to conductor and does not retry it in a loop.
Conductor answers a `[RESOURCE <repo>] disk` or `build-slot` with "use conductor build run".

## 5. Enforce

The [guard](/docs/security/the-guard) denies a controller's build, test or gate that does not run
under `conductor build run`: `cargo build`, `test`, `check`, `clippy`, `nextest`, `install`, `run`,
`bench`, `doc`, `rustc` and `fix`, the tasks `check`, `gate`, `install`, `regen`, `docs-generate`
and `ui`, and `npm ci`, `npm test` and the `build` and `test` scripts. The denial gives the command to run:

```text
`cargo test -p example` builds, tests or gates outside the host's disk ledger: run it as
`conductor build run --reserve auto -- cargo test -p example`, which reserves its disk first;
a refusal names the shortfall: report it, do not retry
```

`cargo fmt`, `rustfmt`, `cargo metadata` and `cargo tree` build nothing and stay allowed. Conductor's own
session runs no builds, and the rule does not hold its Bash.
