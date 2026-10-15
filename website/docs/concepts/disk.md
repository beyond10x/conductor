---
title: Disk
sidebar_position: 6
description: How the conductor binary runs disk as a protocol in five steps (reserve, reclaim, watch, escalate, enforce) so that no build waits on a message and no session has to remember to free space.
lede: Every build, test or gate reserves its disk from one host ledger, and the binary frees, pauses, refuses and reports on its own.
source: docs/design/conductor.md sections 7 and 9; spec/domains/host.yaml; crates/conductor/src/guard/rules.rs; .agents/conductor.md, .agents/repo-controller.md, .agents/conductor-dev.md; crates/conductor/tests/guard_rules.rs, crates/conductor/tests/profiles.rs
---

Several sessions build on one machine, and a build that runs out of disk stops everyone's work.
Conductor does not grant build slots in conversation. The `conductor` binary runs the disk as a
protocol in five steps, and a session's part is one command:

```console
$ conductor build run --reserve 8G -- task check
```

## 1. Reserve

`conductor build run --reserve <size> [--repository <repo>] -- <command>` asks the host ledger for
`<size>`, the command's expected peak, written in GiB with `G`. The ledger is shared by every
instance on the host, so two instances or two runs never grant the same free space. The binary
grants at once when free disk, minus the live reservations, minus the floor, covers the size;
nobody is asked.

| command | default reservation |
|---|---|
| `cargo build`, `test`, `check`, `clippy`, `nextest`, `install`, `run` | `8G` |
| `task check`, `gate`, `install`, `regen`, `docs-generate` | `8G` |
| `npm ci`, `npm run build`, `npm run test`, `task ui` | `2G` |

`conductor host grants` lists the ledger: each live reservation, its command, its size and the
peak it measured.

## 2. Reclaim

When the size does not fit, the binary reclaims before it answers. `conductor host reclaim` frees
only what needs no decision, in order, until the reservation fits:

- the build output of an idle managed tree (no live session works in it, no gate runs in it);
- build output left in worktree archives;
- caches past their age;
- snapshots past retention.

Each step is recorded with the path it removed and the bytes it freed. A tree a live session works
in, or a running gate's target, is never touched. Nor are the instance's records, state,
checkouts or trees, however a symbolic link leads into them: a scratch root that is itself a link is skipped,
and a stale scratch entry that is a link is removed as a link, never followed. The watch also runs a reclaim once each time free
disk falls under `thresholds.disk_admit`, so space is freed before anyone asks.

## 3. Watch the run

Once granted, the command runs in its own process group under a disk watchdog. Under
`thresholds.gate_stop` the group is paused and a reclaim runs; over `thresholds.gate_resume` it
resumes. The peak of the directories the command writes (the tree's `target/`, or the working
directory) is recorded with the reservation, and the reservation is released when the command
exits or is killed. `conductor build run` exits with the command's own status.

Stopping `conductor build run` with SIGTERM, SIGINT or SIGHUP stops the build with it. The signal
is forwarded to the command's whole process group; a group with a process left 10 seconds later is
killed. The peak is recorded, the reservation is released, and `conductor build run` exits with
128 plus the signal. A request still waiting in the queue is withdrawn instead (state `Withdrawn`
in `conductor host grants`), and its command never starts.

## 4. Escalate

When reclaim cannot reach the size, `conductor build run` refuses: it prints the shortfall and what
is left by path and size, exits 2, and never starts the command. The binary records one operator
item, listed by `conductor decision hands-todo`, naming the shortfall, what is left and the one
command only a person can run; a second refusal in the same fall records none. The watch prints
`disk floor not reached: <n>G short` once per fall.

A session that is refused reports the refusal to conductor and does not retry it in a loop.
Conductor answers a `[RESOURCE <repo>] disk` or `build` with "use conductor build run": there is no
slot to grant by message.

## 5. Enforce

The [guard](/docs/security/the-guard#bash-and-monitor) denies a controller's build, test or gate
that does not run under `conductor build run`, and the denial names the exact command to run
instead:

```text
`cargo test -p example` builds, tests or gates outside the host's disk ledger: run it as
`conductor build run --reserve 8G -- cargo test -p example`, which reserves its disk first;
a refusal names the shortfall: report it, do not retry
```

`cargo fmt`, `rustfmt`, `cargo metadata` and `cargo tree` build nothing and stay allowed.
Conductor's own session runs no builds, and its Bash is not held.

## Read next

- [The guard](/docs/security/the-guard): the rest of the shell rule.
- [The loop](./the-loop.md): the watch between cycles, and the disk floors it reports.
- [Instances and config](./instances-and-config.md): the thresholds named above.
