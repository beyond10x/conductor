---
format: aep.planning-md/3
id: story:snapshot-store-cost
kind: story
status: draft
title: A snapshot reads the store in proportion to what changed, not all of it
revision: 1
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

## Out of Scope

Compacting or archiving old observations, unless the measurement shows it is the cause.

## Ambiguities

- Which collector reads the most is `inferable` from the first acceptance item's figures.
