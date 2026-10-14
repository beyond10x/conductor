---
format: aep.planning-md/3
id: story:conductor-context-budget
kind: story
status: draft
title: Conductor runs 4 hours before its hand-over threshold
summary: Measure what fills conductor's context and cut the largest source
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

On 2026-10-14 conductor reached its hand-over threshold four times: restarts at about 16:35Z (273k), 17:27Z (200k, about 40 minutes after the previous start), 18:52Z (202k) and 20:34Z (204k, about 100 minutes after the previous start). Each restart re-reads the start of session with a cold cache. The controller-health run of 16:20Z read 34.8M input tokens for conductor over 3 hours (199 turns), 99 % of it the context re-read. Neither the share of the start reading nor the share of message traffic per turn is measured.

## Acceptance

- `conductor health` (or the health report) prints, per conductor session, the context after its first turn and the context added per hour by each source: tool output, messages received, its own output; read from the session transcript.
- One measured change brings conductor's time from start to `thresholds.context_handover` to at least 4 hours on a day with 8 working controllers, recorded as evidence with the before and after figures from two health reports.

## Scope

`crates/conductor/src/health.rs` or the controller-health skill's report, `.agents/conductor.md`, tests.
