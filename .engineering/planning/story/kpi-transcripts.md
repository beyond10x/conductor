---
format: aep.planning-md/3
id: story:kpi-transcripts
kind: story
status: draft
title: K3, K7, K8, K9, K10 and K11 are computed from session transcripts, counts only
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 2
---
## Why

Idle time, waits, session loss, guard friction, corrections and tokens live only in the harness transcripts. (`docs/design/self-improvement.md`.)

## Acceptance

- A transcript reader counts per session and day without keeping or printing message text; over fixture transcripts it reproduces each count; it reads only the projects of the instance's sessions.

## Instrument rules

- The gateway's sessions (the operator's Remote Control session) are read like the others, and each
  relay through it is a round trip (K13).
- Usage is read from the final record of each message id (the maximum over its records): sub-agents
  write a streaming snapshot first.
- Merged PRs are mapped to sessions by repository and time, or by the PR the dispatch records; the
  baseline's reported merges covered only 36 %.

## Scope

`crates/conductor/src/kpi/transcripts.rs` (new), `crates/conductor/tests/kpi_transcripts.rs`.
