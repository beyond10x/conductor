---
format: aep.planning-md/3
id: story:dashboard-stale-while-revalidate
kind: story
status: draft
title: The dashboard never blocks a read on a recompute
revision: 1
---
# Story: dashboard-stale-while-revalidate

## Outcome

Every dashboard endpoint answers at once once it has answered once, however often the store changes.

## Context

story:dashboard-fast memoises each answer by the store's fingerprint. On the live host on 2026-10-15:
- warm reads took 0.012 s;
- the first `/api/board` after a store change took 14 s;
- while a snapshot was writing, `/api/board` and `/api/repositories` timed out at 30 s and at 60 s, at load ~24.

The store is written almost constantly, so a read recomputes, and blocks, nearly every time.

## Acceptance

- A stale answer is served at once, with `X-Conductor-Age`, while one background recompute per endpoint runs.
- Recomputes are debounced per endpoint.
- Before an endpoint's first answer, a read gets 503 with Retry-After.
- A dashboard read never waits on the store's writer lock.
- Tests use an injectable slow computation: a stale read takes under 100 ms, a single recompute runs, the debounce holds, and a 503 is returned before the first answer.

## Out of Scope

Making the board's computation itself faster.

## Ambiguities

None.
