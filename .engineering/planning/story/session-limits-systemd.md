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
- depends_on: story:session-envelope-seam
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/confine.rs
revision: 5
---
## Why

Per-instance and per-session CPU, memory and process limits (architecture-decision-record:external-confinement); plain Linux gives them with
proportional CPU sharing across all cores, which substrate's one-core-per-exec cap does not (docs/analysis/2026-10-09-substrate-benefits.md
§ 1, § 2). Replaces `instance-cgroup-limits`.

## Acceptance

- The `systemd` provider behind `story:session-envelope-seam`, using the way `session-box-spike`
  chose: each instance has a user slice `conductor-<prefix>.slice` with the instance's `limits`;
  each session runs in its own unit or scope inside it with its role's `limits`.
- Its measurement is the cgroup path and the limits read back from the unit.
- Its doctor probe: a user manager is reachable and `cpu`, `memory` and `pids` are delegated.
- Tests read the command lines, properties and probe through a fake `systemd-run`/`systemctl`.
- One recorded live run on this host, in `docs/analysis/`: a controller started through the provider
  shows its cgroup and limits (`systemctl --user show`), answers a SendMessage from another session,
  appears in `claude agents --json`, and resumes with `claude --resume` after a stop.

## Scope

`crates/conductor/src/confine/systemd.rs` (new), `crates/conductor/tests/`, `docs/analysis/`. After
`session-envelope-seam`.
