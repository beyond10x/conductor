---
format: aep.planning-md/3
id: story:sweep-legacy-waiters
kind: story
status: active
title: The sweep leaves a waiter of an older binary alone
relations:
- decomposes: epic:disk-protocol
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T23:04:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T23:04:25Z", actor: "human:timo", revision: 3}
---
## Why

Conductor reported on 2026-10-14: the v0.10.6 sweep refused live `build run` waiters started on
v0.10.5, which hold no run lock while they wait, so they looked dead; six requests were refused
between 22:43 and 22:53Z.

## What

The sweep closes a request or a granted grant only when its run lock file exists and nobody holds it
(a run of 0.10.6 or later that died), or when it has no lock file and is older than three hours (a
waiter of an older binary waits at most an hour).

## Acceptance

- `crates/conductor/tests/build_run.rs`: `a_young_request_with_no_run_lock_file_is_left_to_its_waiter`;
  `a_request_or_grant_whose_run_is_gone_is_closed_by_the_next_run` with free lock files still closes
  both.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/tests/build_run.rs`.
