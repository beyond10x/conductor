---
format: aep.planning-md/3
id: story:substrate-gate-provider
kind: story
status: draft
title: Large gates run as substrate execs
relations:
- decomposes: epic:session-confinement
- depends_on: story:substrate-client
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/substrate.rs
revision: 5
---
## Why

Substrate fits builds: a read-only toolchain root, no network, a `/scratch` for `target/` and exact
usage counters (docs/analysis/2026-10-09-substrate-benefits.md § 3). The epic's "builds and tests run as substrate execs". The daemon cannot
run an exec in an existing checkout yet, and every exec is capped at one core.

## Acceptance

Once `upstream-blocker:substrate-exec-in-checkout` is cleared:
- Spec first: `gate run --tree <path> -- <argv>` in `crates/conductor/cli.yaml`.
- A recorded run in `docs/analysis/`: `conductor gate run --tree <a managed tree> -- cargo test
  --offline -p conductor` exits as the plain run does, and its usage (wall time, CPU, memory peak)
  is recorded in the observation store.
- A test with a fake substrate that refuses: the refusal's name is printed, the exit is non-zero,
  and nothing runs outside substrate.

## Scope

`crates/conductor/cli.yaml`, `crates/conductor/src/confine/substrate.rs`,
`crates/conductor/src/gate.rs` (new), `crates/conductor/tests/`, `docs/analysis/`. After
`substrate-client`.
