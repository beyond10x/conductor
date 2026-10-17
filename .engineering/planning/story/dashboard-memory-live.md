---
format: aep.planning-md/3
id: story:dashboard-memory-live
kind: story
status: active
title: The live dashboard's memory stays bounded and its health view reads it right
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T01:12:33Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T01:12:34Z", actor: "human:timo", revision: 4}
---
# Story: dashboard-memory-live

## Outcome

On the live instance, the dashboard's resident plus swapped memory stays within twice its size 10 minutes after start, over 24 hours. Its health view reports the same resident size the kernel does.

## Context

story:dashboard-memory-bound shipped in 0.16.1: it bounds the memo per endpoint and drops the old replay before a reopen. Its fixture test holds. Measured live on 2026-10-16 at 22:53Z, the 0.16.5 dashboard of the first instance, 1 h 42 min after start:
- `/proc/<pid>/status` showed VmRSS 480 MB and VmSwap 1.2 GB, about 1.7 GB in all;
- `/api/health` `.memory` reported `rss_bytes` 11 MB, with 1 store handle and 14 memo entries.

So two defects:
1. **Memory still grows** outside the memo and the store handle. Suspects: the stale-while-revalidate recompute threads, each holding a replay; snapshot views built per request; the allocator not returning freed replays (glibc arenas). The 221 MB store is replayed per recompute.
2. **The health view misreads RSS**, for example statm pages read as KB, or the wrong process or field.

## Acceptance

1. `/api/health` `.memory.rss_bytes` equals VmRSS from `/proc/self/status` within 5 %, and a `swap_bytes` field reports VmSwap. A test reads both.
2. The growth is found with evidence: heap profile or allocation counts per endpoint over a replay of the live store copy (the measurement fixture of story:snapshot-store-cost). The cause is fixed with a test that fails before.
3. Live evidence: RSS plus swap after 24 h is within 2× the 10-minute reading, recorded on this story.

## Out of Scope

Making the replay itself smaller (peak memory of the replay).

## Ambiguities

None.
