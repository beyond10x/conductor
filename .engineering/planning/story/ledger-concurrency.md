---
format: aep.planning-md/3
id: story:ledger-concurrency
kind: story
status: implemented
title: The host ledger keeps concurrent writes and counts only what running builds have not written
relations:
- decomposes: epic:disk-protocol
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T18:58:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T18:58:49Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T19:06:35Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Conductor reported on v0.10.0 and v0.10.1 (2026-10-13), from several builds reserving at once:
`conductor build run` failed before its command started with `host release-grant: the store did not
keep it: … moved to version 5 after this handle read version 4`; the ledger refused seven
reservations at 55G free because every running build counted its full reservation although free
disk already held what it had written; and a refused reservation wrote absolute home paths into the
decision log and suggested an `rm -rf` to paste.

## What

- `start-run`, `record-peak` and `release` write under the ledger's lock, as `request` and `reserve`
  do.
- A running grant holds its reservation less the growth its run recorded; the watchdog records the
  growth since the run started (measured before the command starts) each time it passes a further
  GiB, without waiting on the lock.
- The shortfall item writes every path with the home directory as `~`, and names what to review,
  largest first, instead of a removal command.

## Acceptance

- `crates/conductor/tests/build_run.rs`: `runs_at_once_keep_every_ledger_write` (six runs at once,
  every grant released); `a_running_grant_holds_its_reservation_less_what_it_wrote` (a 1.5G
  reservation fits beside a 2G run that recorded 1.5G of growth at 2G available);
  `a_reservation_that_fits_runs_the_command_and_is_released_with_its_peak` records a fast command's
  growth; the shortfall item names no `rm -rf`.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/src/build.rs`,
`crates/conductor/tests/build_run.rs`.
