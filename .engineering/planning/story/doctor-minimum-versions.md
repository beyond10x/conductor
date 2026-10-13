---
format: aep.planning-md/3
id: story:doctor-minimum-versions
kind: story
status: draft
title: conductor doctor refuses an aep, ess or worktree older than its build's pins
relations:
- depends_on: story:doctor-verdict
scope:
- confidence: cited
  path: crates/conductor/build.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
revision: 2
---
## Why

A team member's `conductor doctor` (v0.4.0, 2026-10-13) printed `aep: present, aep 0.63.1` with no
warning while conductor's release pinned aep 0.71: without a `b10x.toml` in the directory or above
it, doctor compares no version (`crates/conductor/src/doctor.rs:251`, `examine(pins)`), so an old
`aep` that cannot open a current planning store passes.

## Acceptance

- Without a `b10x.toml`, `conductor doctor` compares `aep`, `ess` and `worktree` with the minimum
  versions built into the binary from the product repository's own `b10x.toml` at build time, and
  reports an older one as `too old: <found> < <minimum>` and exits 1; a test in
  `crates/conductor/tests/doctor.rs` with a fake `aep` printing `aep 0.63.1` on `PATH` holds it.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/build.rs`, `crates/conductor/tests/doctor.rs`.
