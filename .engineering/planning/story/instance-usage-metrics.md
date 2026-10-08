---
format: aep.planning-md/3
id: story:instance-usage-metrics
kind: story
status: draft
title: The dashboard shows CPU, memory and io per instance and session
relations:
- informed_by: story:substrate-session-spike
- depends_on: story:instance-session-names
- decomposes: epic:session-confinement
- depends_on: story:session-limits-systemd
- depends_on: story:session-envelope-model
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
  path: crates/conductor/tests/usage_metrics.rs
- confidence: cited
  path: docs/analysis/usage-metrics-run.md
- confidence: cited
  path: spec/domains/observation.yaml
revision: 14
---
## Why

The host is shared; nothing attributes load to an instance or a session, and the operator's
complaint about host load had to be investigated by hand (`specification:multi-instance-host`,
finding 7). Today every session shares one cgroup (`session-3.scope`), so per-session numbers come
from the processes until `story:session-limits-systemd` gives each session a cgroup of its own.

## Acceptance

- Before any code: `claude agents --json` is checked for a pid per background session. If it has
  none, the unit stops and reports, and the `/proc` source is redrawn.
- Spec first, in `spec/domains/observation.yaml`: a session's usage (CPU time, resident memory, io
  bytes, process count, read time, source).
- Sources, each tested with fixtures (`crates/conductor/tests/usage_metrics.rs`):
  - the session's process tree from `/proc` (`/proc/<pid>/stat`, `status`, `io`, children through
    `/proc/<pid>/task/*/children`); a fixture `/proc` tree;
  - the session's cgroup v2 files when the envelope measurement names a cgroup; a fixture cgroup
    directory.
- The dashboard shows the usage per session and summed per instance: a render test with fixture
  readings finds each value in the output.
- One recorded run (`docs/analysis/usage-metrics-run.md`): the dashboard page for a live controller
  on this host, beside `systemctl --user show` or `ps` for the same session in the same minute.
- Not here: a threshold line in the watch (alerting is not in the epic), and substrate's usage
  (that reading belongs to `story:substrate-client`).

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model/`, `crates/conductor/src/usage_metrics.rs`
(new), `crates/conductor/src/dashboard/`, `crates/conductor/src/lib.rs`,
`crates/conductor/tests/usage_metrics.rs` (new), `docs/analysis/usage-metrics-run.md` (new). After
`session-limits-systemd` (cgroup per session) and `session-envelope-model` (shared
`observation.yaml` and generated crate).
