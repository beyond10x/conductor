---
format: aep.planning-md/3
id: story:build-run-backfill
kind: story
status: implemented
title: A disk request that fits runs while a larger one ahead of it cannot
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T01:54:09Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T01:54:09Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T01:54:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: build-run-backfill

## Outcome

The host's build queue keeps moving when its head cannot fit: smaller requests that fit run, and a head that cannot fit is refused at once rather than after a reclaim walk.

## Context

Conductor's NEED of 2026-10-15: 12 requests were Requested and none Running at 26G free. The floor is `thresholds.disk_admit` 15 GiB plus reclaim's MARGIN of 5 GiB (gc/reclaim.rs:83), about 21.5 GB.

build.rs:316-372 answers only the head of the queue (`ahead == 0`). A head whose shortfall no live reservation covers is refused, but only after a reclaim, and reclaim runs at most once every 300 s. So the queue drains about one request per walk, and the 2G and 4G requests behind an 8G head that cannot fit wait 40 minutes or more.

## Acceptance

- When the head's shortfall is larger than every live reservation together, so it can only be refused, a request behind it that fits beside the floor and the live reservations is granted at once (backfill).
- A request behind is never granted while the head is waiting for live reservations to release (`short <= holding`), so the head does not starve.
- When a reclaim run that finished within the last 300 s freed nothing, a head that does not fit is refused at once, naming that run, instead of waiting for the next walk.
- Tests with a fake disk reading:
  - an 8G head at 5G available is refused at once, and a 2G request behind it is granted;
  - a head waiting on a live 6G reservation keeps a 2G request behind it waiting;
  - no request is granted past the floor.

## Out of Scope

Sizing reservations from recorded peaks (story:reserve-from-peak).

## Ambiguities

None.
