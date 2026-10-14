---
format: aep.planning-md/3
id: story:guard-latency-quiet-host
kind: story
status: implemented
title: The guard's allow latency and denial recording are measured on a quiet host after guard-all-tools
scope:
- confidence: cited
  path: .github/workflows/
- confidence: cited
  path: crates/conductor/tests/guard_latency.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T02:15:09Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-14T02:15:09Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-14T02:51:36Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Wave 04's `story:guard-all-tools` (commit `1f8bd04`) was measured on a loaded host (load average
22–30): `tests/guard_latency.rs` gave the allowed Write p50 1.238 ms before and 1.693 / 1.453 ms
after, and denials not recorded within `RECORD_DEADLINE` rose from 9 of 50 to 43 of 50. Neither
number can be compared under that load. An unrecorded denial still denies, but it is missing from
`conductor guard guard-decisions`, which conductor-dev reads to find faults (design § 12).

## Acceptance

A `workflow_dispatch` job under `.github/workflows/` builds in release, on one runner, the binary
before `1f8bd04` and the binary on `main`. It runs `guard_latency` against each
(`cargo test --release -p conductor-cli --test guard_latency -- --ignored --nocapture`,
`crates/conductor/tests/guard_latency.rs:7-11`) and prints the allowed Write p50 and p95 and the
unrecorded-denial count for each. One dispatched run's numbers go in this story's evidence. If the
allow path is slower by more than 10 % or more than 5 of 50 denials go unrecorded, the evidence
names that, and a story for the fix is drafted citing the run.

## Scope

`.github/workflows/` (a new `workflow_dispatch` workflow; `check.yml` stays as it is),
`crates/conductor/tests/guard_latency.rs`.
