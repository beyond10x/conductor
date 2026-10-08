---
format: aep.planning-md/3
id: story:instance-usage-metrics
kind: story
status: draft
title: The dashboard and the watch show CPU, memory and io per instance and session
relations:
- decomposes: epic:multi-instance-host
- informed_by: story:substrate-session-spike
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: crates/conductor/src/dashboard
- confidence: cited
  path: crates/conductor/src/usage_metrics.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 6
---
## Why

The host is shared; nothing attributes load to an instance or a session, and the operator's
complaint about host load had to be investigated by hand (`specification:multi-instance-host`,
finding 7).

## Acceptance

- The dashboard shows, per instance and per session: CPU time, memory current and peak, io bytes
  and process count, with the read time. Two sources, each tested: the session's cgroup v2 files
  (fixture cgroup directories), and substrate's `GET /v1/metrics` response for a session that runs
  under substrate (a fixture response body in the shape of substrate's `run-a-command` guide § 5).
- The watch prints one line when an instance's memory or CPU passes a config threshold (spec
  first); a test with fixture readings.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/usage_metrics.rs` (new),
`crates/conductor/src/dashboard/`, `crates/conductor/src/watch.rs`,
`crates/conductor/tests/usage_metrics.rs` (new). After `instance-session-names` (shared
`watch.rs`, `config.yaml`).
