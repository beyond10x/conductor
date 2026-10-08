---
format: aep.planning-md/3
id: story:instance-cgroup-limits
kind: story
status: draft
title: Each instance's sessions run under CPU, memory and process limits
relations:
- decomposes: epic:multi-instance-host
- depends_on: story:substrate-session-spike
- depends_on: story:instance-scoped-tasks
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: spec/domains/config.yaml
revision: 4
---
## Why

Design item 5(b) of `specification:multi-instance-host` (scope critic, round 1): per-instance cgroup
limits so one instance cannot starve another. Whether it runs through substrate or a plain systemd
user scope depends on story `substrate-session-spike`.

## Acceptance

- Spec first: `instances[].limits` (`cpu_weight`, `memory_max`, `pids_max`).
- Every session an instance starts runs inside the instance's cgroup with those limits (through
  substrate if the spike says go, else `systemd-run --user --scope -p MemoryMax=… -p CPUWeight=…
  -p TasksMax=…`); a test reads the started command line and the scope's properties from a fake
  `systemd-run`.

## Scope

`spec/domains/config.yaml`, `Taskfile.yml`, `.agents/conductor.md`,
`crates/conductor/tests/taskfile.rs`. After `substrate-session-spike`, `instance-scoped-tasks`.
