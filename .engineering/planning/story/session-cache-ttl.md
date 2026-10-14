---
format: aep.planning-md/3
id: story:session-cache-ttl
kind: story
status: draft
title: Spawned sessions keep their prompt cache for an hour, not five minutes
relations:
- decomposes: epic:usage-continuity
revision: 1
---
## Why

The controller-health probe (2026-10-14 11:31Z, 3 h window, 12 background sessions) found 70 % of
the sessions' cache writes use the 5-minute prompt-cache TTL, while the conductor-dev session runs
with a 1-hour TTL. A controller that waits on a build queue (median wait 15 min that hour) or a CI
run longer than 5 minutes pays its whole context again on its next turn.

## Acceptance

- The setting that selects a session's prompt-cache TTL is found and named in
  `docs/analysis/session-cache-ttl.md`, with the probe's before numbers; `conductor spawn` and
  `task conductor:start` start sessions with the 1-hour TTL when the harness offers it; a test in
  `crates/conductor/tests/spawn.rs` holds the flag or environment the spawn passes.
- A probe run a day later records the share of 5-minute cache writes and the cold-wake tokens in
  the same analysis page; the story is implemented only if the share fell.

## Scope

`crates/conductor/src/spawn.rs`, `Taskfile.yml`, `crates/conductor/tests/spawn.rs`,
`docs/analysis/session-cache-ttl.md` (new).
