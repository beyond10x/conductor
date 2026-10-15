---
format: aep.planning-md/3
id: story:store-readers-hold-no-writer-lock
kind: story
status: draft
title: Long-running readers never hold a store's writer lock while idle
revision: 1
---
# Story: store-readers-hold-no-writer-lock

## Outcome

A snapshot, a guard hook and a build run never wait on a store lock that `conductor watch run --follow` or `conductor dashboard serve` holds while it sleeps.

## Context

On the live host, 2026-10-15 ~18:4xZ:
- Snapshot 01a140a9 held `state/tree/.lock` and waited ~57 minutes on `state/observations/.lock`, according to /proc/locks.
- That lock was held, flock WRITE, by `conductor watch run --follow` while it slept between polls, and before that by the dashboard-fast build of `dashboard serve`, through its kept store handle.
- Behind the snapshot, every guard hook timed out ("the store did not take it within 2 s"), and so did `conductor build run`. It holds `host.lock` while it waits on the tree lock, so the host's whole build queue stopped.

## Acceptance

- The watch and the dashboard open each store read-only, or take its writer lock only for the length of one read or one append, never across a sleep or between requests.
- Test: while a `watch run --follow` and a `dashboard serve` are running and have each polled or served once, a store append from another process succeeds within 2 s.
- `build run` never waits on the store's tree lock while it holds `host.lock`.

## Out of Scope

The eventlog crate's lock design itself.

## Ambiguities

None.
