---
format: aep.planning-md/3
id: story:dashboard-memory-bound
kind: story
status: implemented
title: The dashboard's memory stays bounded however long it runs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T11:24:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T11:24:24Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T11:24:25Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: dashboard-memory-bound

## Outcome

`conductor dashboard serve` runs for days with a resident size that tracks the size of what it serves, not its uptime.

## Context

On 2026-10-16 the 0.15.0 dashboard of the first instance held 2.5 GB resident and 3.7 GB of swap after about 7 hours, using half a core. The second instance's dashboard, with a small store, held 9 MB. After a restart, the first reached 750 MB within 30 s.

The handle that dashboard-fast keeps per store (crates/conductor/src/dashboard/cache.rs `Stores`) replays the whole tree into memory. Every reopen after a write may build a new replay before the old one is dropped. The memo (`Memo`, keyed by request target) has no bound on its keys.

## Acceptance

- Resident memory is reported by the health view: the dashboard's own RSS, and the bytes held by its store handles and its memo.
- A reopened store handle replaces the old one, and the old one is dropped before the next reopen. The memo keeps at most N entries per endpoint (query strings included), evicting the oldest.
- A test with a fixture store and 1000 writes between requests shows that the memo's entries and the store handles stay at their bounds.
- On the live instance, the dashboard's RSS after 24 h is within 2× its RSS 10 minutes after start, recorded as evidence.

## Out of Scope

Making a replay of the whole store cheaper (story:snapshot-store-cost).

## Ambiguities

None.
