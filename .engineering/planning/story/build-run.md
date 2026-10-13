---
format: aep.planning-md/3
id: story:build-run
kind: story
status: implemented
title: conductor build run reserves, reclaims, runs under a disk watchdog and releases
relations:
- decomposes: epic:disk-protocol
- depends_on: story:disk-protocol-spec
- depends_on: story:disk-reclaim
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T17:12:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T18:12:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`epic:disk-protocol`, steps 1-3: every build reserves its peak, runs under a disk watchdog and
releases what it reserved. Today a build slot is a message to conductor, the watchdog is a shell
loop each session writes, and nothing records a peak.

## What

`conductor build run --reserve <size> [--repository <repo>] -- <command> [args…]`:
reserves `<size>` in the host ledger (reclaiming first when it does not fit; refusing with the
shortfall when reclaim cannot reach it), runs the command in its own process group, pauses it
(`SIGSTOP` to the group) under `gate_stop` and runs reclaim, resumes it (`SIGCONT`) over
`gate_resume`, measures the peak of the directories it writes (the tree's `target/`, or the
working directory), records the peak with the grant, releases the grant when the command exits or
is killed, and exits with the command's status. Supersedes `story:gate-run`'s watchdog.

## Acceptance

- `crates/conductor/tests/build_run.rs` with a fake free-disk reading and a fake command:
  - fits: granted, the command runs, the grant is released with its peak, the command's exit code
    is returned;
  - does not fit, reclaim reaches it: one reclaim run is recorded, then granted;
  - does not fit, reclaim falls short: refused with the shortfall, exit 2, the command never runs;
  - a reading under `gate_stop` while running pauses the group, one over `gate_resume` resumes it;
  - the command killed: the grant is released;
  - two runs at once each reserve, and the second sees the first's reservation.

## Scope

`crates/conductor/src/build.rs` (new), `crates/conductor/src/host.rs` (new, the ledger),
`crates/conductor/src/cli.rs`, `crates/conductor/src/lib.rs`, `crates/conductor/tests/build_run.rs`,
`crates/conductor/tests/cli_tree.rs`.
