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
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/dashboard
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/usage_metrics.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 9
---
## Why

The host is shared; nothing attributes load to an instance or a session, and the operator's
complaint about host load had to be investigated by hand (`specification:multi-instance-host`,
finding 7). Today every session shares one cgroup (`session-3.scope`, design critic round 2), so
per-session numbers come from the processes, until `instance-cgroup-limits` gives each instance a
cgroup of its own.

## Acceptance

- The dashboard shows, per instance and per session: CPU time, resident memory, io bytes and
  process count, with the read time. Source today: the session's process tree from `/proc`
  (`/proc/<pid>/stat`, `status`, `io`, children through `/proc/<pid>/task/*/children`), the
  session's pid from `claude agents --json`; a session in a cgroup of its own (later) reads that
  cgroup's v2 files instead; a session run under substrate reads `GET /v1/metrics`. Each source is
  tested with fixtures (a fixture `/proc` tree, a fixture cgroup directory, a fixture metrics body).
- The watch prints one line when an instance's resident memory or CPU passes a config threshold
  (spec first); a test with fixture readings.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/usage_metrics.rs` (new),
`crates/conductor/src/dashboard/`, `crates/conductor/src/watch.rs`, `crates/conductor/src/lib.rs`,
`crates/conductor/tests/usage_metrics.rs` (new), `crates/conductor-model/`.
After `instance-session-names` (shared `config.yaml`, `watch.rs`, the generated crate).
