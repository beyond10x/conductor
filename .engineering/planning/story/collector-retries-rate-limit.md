---
format: aep.planning-md/3
id: story:collector-retries-rate-limit
kind: story
status: draft
title: A collector retries a rate-limited forge call
summary: one HTTP 429 failed a whole snapshot for 2 h
revision: 1
---
## Why

Observed 2026-11-07 on a second instance, its dispatch log line 103 (cycle 13:17Z): the whole snapshot failed on one `glab api groups/<group>/projects` HTTP 429. No retry, so the next snapshot came one `cadence.cycle` (2 h) later.

## Acceptance

- A collector call that answers HTTP 429 (or 502/503) is retried up to 3 times with backoff (honouring `Retry-After` when given, capped at 60 s); a test with a fake forge that answers 429 once records the snapshot complete.
- A collector that still fails after its retries fails only its own rows; the snapshot records the others and names the failed collector.
