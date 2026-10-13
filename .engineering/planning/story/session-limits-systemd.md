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
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/confine/systemd.rs
- confidence: cited
  path: crates/conductor/tests/confine_systemd.rs
- confidence: cited
  path: docs/analysis/session-limits-run.md
- confidence: cited
  path: docs/config.md
revision: 14
---
## Why

Sessions share the host's CPU, memory and process table with no limit; the config's `limits` per
instance and per role (`spec/domains/config.yaml:152-241`) apply nowhere. This host delegates
`cpu`, `memory` and `pids` to the user manager (`systemctl show -p DelegateControllers
user@1000.service`: `cpu memory pids`; the same in its `cgroup.controllers`), which is what the
spike used (`docs/analysis/session-box-spike.md`).

## Acceptance

- The `systemd` provider implements `story:session-envelope-seam`'s trait: each instance has a user
  slice `conductor-<prefix>.slice`, or `conductor.slice` without a prefix, carrying the instance's
  `limits`; `start` runs the session in a unit `conductor-<session name>.service` inside it, through
  `systemd-run --user --unit … --slice … -p MemoryMax= -p CPUWeight= -p TasksMax=` from the role's
  `limits`, with the spike's PTY and FIFO launcher; `prompt` writes to the FIFO; `stop` is
  `systemctl --user stop`; `resume` starts a new unit running `claude --resume <id>`. Tests in
  `crates/conductor/tests/confine_systemd.rs` read each command line through a fake
  `systemd-run`/`systemctl` on `PATH`, for a controller and for conductor, with and without a
  prefix.
- `measure` returns the unit's cgroup path and `memory.max`, `cpu.weight` and `pids.max` read from
  that cgroup's files (absent where a file reads `max`), as `spec/domains/observation.yaml`
  defines; a test with a fixture cgroup directory reads them.
- `available` reads the user manager's delegated controllers from
  `/sys/fs/cgroup/user.slice/user-<uid>.slice/user@<uid>.service/cgroup.controllers` (not
  `systemctl --user show`, which reports `Delegate=no` on this host), and is available when `cpu`,
  `memory` and `pids` are listed; a test with a fixture file holds both answers.
- One recorded live run on this host (`docs/analysis/session-limits-run.md`): a controller started
  through the provider has `memory.max`, `cpu.weight` and `pids.max` equal to its role's configured
  `limits` (and the slice to the instance's), answers a SendMessage from another session, appears
  in `claude agents --json`, and resumes after a stop; a worker sub-agent's processes sit in its
  controller's cgroup; conductor's own session started with `session start --role conductor` has
  its role's limits (in a restart window agreed with conductor); no `conductor-*` unit is left after
  cleanup.
- The `limits.*` rows of `docs/config.md` name the systemd provider as their reader.

## Notes

- `io` is not delegated here (`decision-blocker:host-io-delegation`); only cpu, memory and pids.
- After `story:session-envelope-seam` (the trait; `docs/config.md` is shared and the seam edits it
  first).

## Scope

`crates/conductor/src/confine/systemd.rs` (new), `crates/conductor/src/confine/mod.rs`,
`crates/conductor/tests/confine_systemd.rs` (new), `docs/analysis/session-limits-run.md` (new),
`docs/config.md`.
