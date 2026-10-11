---
format: aep.planning-md/3
id: story:guard-latency-quiet-host
kind: story
status: draft
title: The guard's allow latency and denial recording are measured on a quiet host after guard-all-tools
scope:
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_latency.rs
revision: 2
---
## Why

Wave 04's `story:guard-all-tools` (commit `1f8bd04`) was measured on a loaded host (load average
22–30): `tests/guard_latency.rs` gave the allowed Write p50 1.238 ms before and 1.693 / 1.453 ms
after, and denials not recorded within `RECORD_DEADLINE` rose from 9 of 50 to 43 of 50. Neither
number can be compared under that load. An unrecorded denial still denies, but it is missing from
`conductor guard guard-decisions`, which conductor-dev reads to find faults (design § 12).

## Acceptance

- On a quiet host (load average under 2), `guard_latency` run on the binary before `1f8bd04` and on
  `main` gives the allowed Write p50 and p95 and the unrecorded-denial count for each; the numbers
  go in this story's evidence.
- If the allow path is slower by more than 10 % or more than 5 of 50 denials go unrecorded, a fix
  lands with the same measurement after it.

## Scope

`crates/conductor/src/guard.rs`, `crates/conductor/src/guard/rules.rs`,
`crates/conductor/tests/guard_latency.rs`.
