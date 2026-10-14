---
format: aep.planning-md/3
id: story:guard-latency-findings
kind: story
status: draft
title: The guard's denials are recorded on a dispatched run, and the allow path's growth is explained
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

The first dispatched guard-latency run (https://github.com/beyond10x/conductor/actions/runs/38501293066,
2026-10-14, one `ubuntu-latest` runner, before = `d6fb77b3^`, after = `main`) measured:

| side | allowed Write p50 | p95 | denials unrecorded within the deadline |
|---|---|---|---|
| before | 2052.6 µs | 2140.6 µs | 50 of 50 |
| after | 2264.6 µs | 2356.2 µs | 50 of 50 |

The allow path is 10.3 % slower, just over the story's 10 % bound. Every denial went unrecorded on
both sides, so that count comes from the measurement or the runner, not from guard-all-tools.

## Acceptance

- The cause of 50 of 50 unrecorded denials on the runner is found and named (the deadline
  `RECORD_DEADLINE` in `src/guard.rs` against the runner's store, the test's own wait, or the
  state directory the test passes); either the test or the guard is fixed so a dispatched run
  records at least 45 of 50 on `main`, and the run's numbers go in this story's evidence.
- A second dispatched run states whether the +10 % allow-path difference repeats; if it does,
  the allow path of a conductor-role write is profiled and the step that grew is named.

## Scope

`crates/conductor/src/guard.rs`, `crates/conductor/tests/guard_latency.rs`,
`.github/workflows/guard-latency.yml`.
