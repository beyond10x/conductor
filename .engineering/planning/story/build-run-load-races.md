---
format: aep.planning-md/3
id: story:build-run-load-races
kind: story
status: draft
title: Two in-process build_run cases pass under heavy load
revision: 2
---
# Story: build-run-load-races

## Outcome

Two build_run cases that run in process pass under heavy load, or the code defect they expose is fixed.

## Context

While fixing story:build-run-signal-tests-flaky (b8c13016, a fixture rewrote `readings.txt` non-atomically), two other cases also failed, under `stress --cpu 44 --io 4 --hdd 4 --hdd-bytes 256M` (load about 90). They failed both before and after that fix:
- `requests_are_granted_in_arrival_order_and_a_later_small_one_does_not_overtake` failed 1 of 4 runs, then 1 of 5: `timed out waiting until both 2G runs ran`.
- `the_ledger_lock_is_free_while_a_reclaim_runs` failed 2 of 7 runs: it panicked at build_run.rs:1994 because `try_lock` returned `None`.

Both run in process without the readings file, so that fix does not explain them. The second one may be a real defect: the story store-readers-hold-no-writer-lock promises the ledger lock is free while a reclaim runs.

## Acceptance

1. Each failure is reproduced under the same stress and its cause is named with evidence.
2. A code defect is fixed with a test that fails before; a test defect is fixed without raising timeouts, unless the evidence shows the work legitimately takes longer.
3. Under the same stress, 3 build_run runs in a row pass.

## Out of Scope

The withdraw cases (fixed in b8c13016).

## Ambiguities

None.
