---
format: aep.planning-md/3
id: story:session-limits-systemd
kind: story
status: draft
title: Each instance and session runs under systemd limits
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-box-spike
- depends_on: story:session-envelope-model
- supersedes: story:instance-cgroup-limits
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/confine.rs
revision: 4
---
## Why

Per-instance and per-session CPU, memory and process limits (architecture-decision-record:external-confinement); plain Linux gives them with
proportional CPU sharing across all cores, which substrate's one-core-per-exec cap does not (docs/analysis/2026-10-09-substrate-benefits.md
§ 1, § 2). Replaces `instance-cgroup-limits`.

## Acceptance

- Using the way `session-box-spike` chose: each instance has a user slice
  `conductor-<prefix>.slice`; each session it starts runs in its own unit or scope inside it, with
  the role's `limits` from the envelope request (story `session-envelope-model`).
- The measurement is recorded: the cgroup path and the limits read back from the unit.
- Tests read the command lines and properties through a fake `systemd-run`/`systemctl`; one recorded
  live run on this host in `docs/analysis/`.

## Scope

`Taskfile.yml`, `.agents/conductor.md`, `crates/conductor/src/` (a `confine` module),
`crates/conductor/tests/`. After `session-box-spike`, `session-envelope-model`.
