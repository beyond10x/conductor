---
format: aep.planning-md/3
id: story:status-current-snapshot
kind: story
status: active
title: conductor status reads the current snapshot only
summary: status and the dashboards answer in under 2 s on an instance with 50k tree files, reading the current snapshot's streams or an index, older snapshots on request
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: inferred
  path: crates/conductor/src/collect/usage.rs
- confidence: cited
  path: crates/conductor/src/dashboard
- confidence: inferred
  path: crates/conductor/src/host.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/store.rs
- confidence: inferred
  path: crates/conductor/src/store/board.rs
- confidence: cited
  path: crates/conductor/src/store/host.rs
- confidence: cited
  path: crates/conductor/src/store/observations.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_fast.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:16:46Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T02:16:47Z", actor: "human:timo", revision: 3}
---
## Why

`conductor status` reads every observation stream of every snapshot and the whole `HostGrant` stream on each call. On 2026-11-08 (conductor-teacher, relayed by the host supervisor) `conductor status` of beyond10x took over 120 s at 03:10, was stopped by `timeout 100` at 03:12, and took 43 s under strace at 03:14 (two other instances 5 s and 7 s; load 61). The strace counted 196,636 syscalls: 6,914 opens under `<state>/tree/tenants/conductor/streams/conductor.host.HostGrant…`, and about 4,300 under each of 14 snapshot tenants in `<state>/observations/tenants/<snapshot>/streams/conductor.observation.*`. File counts: beyond10x `<state>/tree` 52,457 and `<state>/observations` 19,338 (14 tenants); the two other instances 1,996 / 10,418 and 1,493 / 11,185. The supervisor and the dashboards read the same store, so a slow `status` stalls both.

## Acceptance

- `conductor status` answers in under 2 s on an instance whose `<state>/tree` holds 50k files and whose observations hold 14 snapshots, by reading the current snapshot's streams only, or an index; older snapshots are read only on request. A test with a fixture of that size holds the bound.
- The `HostGrant` stream is not read whole on each call: open grants come from an index or the newest state.
- The dashboard pages that read the same store (the board, sessions and host pages) answer their `/api/*` calls within the same bound on that fixture.

## Scope

`crates/conductor/src/status.rs`, the store readers it calls (`crates/conductor/src/store/`), the dashboard's API handlers (`crates/conductor/src/dashboard/`), tests.
