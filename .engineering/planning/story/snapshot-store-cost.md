---
format: aep.planning-md/3
id: story:snapshot-store-cost
kind: story
status: implemented
title: A snapshot reads the store in proportion to what changed, not all of it
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:45:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T15:45:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T15:45:11Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: snapshot-store-cost

## Outcome

A snapshot of the instance completes in minutes, however long the instance has run.

## Context

On 2026-10-15, snapshot 01a14133 was still Collecting after 37 minutes. It held no lock and waited on none, its main thread used CPU steadily, and it had read 5.07 GB and written 2.09 GB. Snapshots that day took 1 h or more, against 2.5 min for a young instance. The store's tenant groups grow with every observation, and the snapshot appears to re-read them.

## Acceptance

- The snapshot's read and write bytes are recorded per collector as an observation of the snapshot.
- A measured change brings one snapshot of the live instance under 5 minutes and under 500 MB read, recorded as evidence with the before and after figures.
- A test over a fixture store with 10× the observations shows that read bytes grow with the new observations, not with the whole store.
- A Collecting snapshot whose process is gone is failed by the next snapshot's start, or by the watch, with the reason `process gone`, so a killed run never stays Collecting. Evidence: snapshot 01a142ac of 2026-10-16 ended with SIGKILL (exit 137) at about 1 h and stayed Collecting.

## Out of Scope

Compacting or archiving old observations, unless the measurement shows it is the cause.

## Ambiguities

- Which collector reads the most is `inferable` from the first acceptance item's figures.
