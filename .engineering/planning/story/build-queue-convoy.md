---
format: aep.planning-md/3
id: story:build-queue-convoy
kind: story
status: implemented
title: Waiting build runs do not serialize the host
relations:
- decomposes: epic:disk-protocol
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T08:29:50Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-17T08:29:50Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-17T08:29:50Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: build-queue-convoy

## Outcome

Requests waiting in the host build queue no longer serialize the host. A waiting `build run` checks the ledger without `host.lock` and without reopening the host store, and takes the lock only when the ledger changed or its own answer may have. Admission keeps pace with releases while 20 or more requests wait on a disk saturated by builds.

## Context

A conductor NEED (2026-10-17, 06:2xZ).
- `/proc/locks` sampled every 5 s: the `host.lock` holder changes every sample, with 19–25 waiters. Each holder is in D state (`jbd2_log_wait_commit`, `balance_dirty_pages`).
- The ledger: 22 Requested, 2–3 Running, and about 5,800 grants in all (5,189 Released, 538 Refused, 40 Withdrawn). The host store `host/tree` is 183M.
- Effect: controllers' builds waited 20+ min while the load was 18.8 on 20 cores, and one controller made no commit in 63 min.

Cause:
- Every queued `build run` takes `host.lock` again after each pause (crates/conductor/src/build.rs wait loop, `Ledger::lock` in host.rs).
- Each pass first reopens the host store (`store::open`, replaying and verifying the whole tree).
- It then sweeps every grant ever made, probing a run lock for each.
- On a loaded disk one pass takes seconds, and N waiters take turns.

## Acceptance

1. A ledger write bumps a generation file, `host/ledger.gen`: a counter written atomically. A waiting run reads it without any lock. It takes `host.lock` only when the generation changed since its last pass, or `thresholds.watchdog_every` has passed (to sweep the dead). Test: with N waiters and no ledger change, the waiters make no locked passes during the pause.
2. A run opens the host store once and reuses the handle across its passes. A reopen happens only where the store needs one to see another writer's events. Test or measurement in the story's evidence: store opens per run.
3. Closed grants (Released, Refused, Withdrawn) older than 24 h are no longer read by `sweep`, `reserved`, `ahead_bytes` or `session_bound`. They are either compacted out of the live set or skipped by an index, and `host grants` still lists them. Test with 5,000 closed grants: a pass reads only the open ones.
4. A bench or test drives 20 concurrent waiters against a fake slow store and shows admission within one pause of a release. The report names the before and after numbers.
5. No change to the queue order, backfill or the refusal rules. The build_run, host_ledger and conformance suites stay green.
6. CPU admission: `thresholds.build_slots` (config spec first, default 3; 0 = unlimited). A request is granted only while fewer than `build_slots` disk grants are `Running` or `Granted`, host-wide. It waits in the same queue order, without the lock between passes, exactly like the disk wait, and is never refused for slots. `build run`'s "queued" line names the reason (disk|slots). Test: with slots=1 and one Running, a second request waits, then is granted on release.

## Out of Scope

Admission by measured CPU load, and a daemon for the ledger.

## Ambiguities

None.
