---
format: aep.planning-md/3
id: story:sweep-run-lock
kind: story
status: active
title: A sweep never takes a grant whose build run is still recording and releasing it
relations:
- decomposes: epic:disk-protocol
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T20:04:51Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T20:04:51Z", actor: "human:timo", revision: 3}
---
## Why

Conductor reported on v0.10.3 (2026-10-13): after a successful build, `build run` printed
HostGrantStateConflict on record-peak and release-grant, and the grant showed Released with reason
ProcessGroupGone and no peak. Another run's sweep had seen the command's process group gone and
released the grant while its own `build run` was still recording the peak and releasing it.

## What

Each `build run` holds a run lock (`<state>/host-runs/<grant>.lock`) from the grant to its release;
a sweep releases a running grant whose process group is gone only when no run holds its lock. A
grant of an older binary (no lock file) is swept as before.

## Acceptance

- `crates/conductor/tests/build_run.rs`:
  `a_grant_whose_run_still_holds_its_lock_is_not_swept_when_its_group_is_gone` (the sweep leaves it,
  the run's own peak and release are kept); the group-gone test still passes.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/src/build.rs`, `crates/conductor/tests/build_run.rs`.
