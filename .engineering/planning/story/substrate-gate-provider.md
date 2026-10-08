---
format: aep.planning-md/3
id: story:substrate-gate-provider
kind: story
status: draft
title: Large gates run as substrate execs
relations:
- decomposes: epic:session-confinement
- depends_on: story:substrate-client
- depends_on: story:substrate-tool-server
scope:
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/gate.rs
- confidence: cited
  path: crates/conductor/src/substrate.rs
- confidence: cited
  path: crates/conductor/tests/gate.rs
- confidence: cited
  path: docs/analysis/substrate-gate-run.md
revision: 8
---
## Why

Substrate fits builds: a read-only toolchain root, no network, a `/scratch` for `target/` and exact
usage counters (docs/analysis/2026-10-09-substrate-benefits.md § 3). The epic's "a build or test can run as a substrate exec". This story
delivers the command; routing every session's gates through it is not claimed here and would be a
story of its own after a recorded run shows the cost of the one-core cap.

## Acceptance

Once `upstream-blocker:substrate-exec-in-checkout` is cleared:
- Spec first: `gate run --tree <path> -- <argv>` in `crates/conductor/cli.yaml`.
- A recorded run (`docs/analysis/substrate-gate-run.md`): `conductor gate run --tree <a managed
  tree> -- cargo test --offline -p conductor` exits with the same status as the plain run in the
  same tree.
- The same run prints the exec's wall time, CPU time and memory peak: a test with a fake substrate
  checks the printed lines (`crates/conductor/tests/gate.rs`).
- A test with a fake substrate that refuses: the refusal's name is printed, the exit is non-zero,
  and nothing runs outside substrate.

## Scope

`crates/conductor/cli.yaml`, `crates/conductor/src/gate.rs` (new), `crates/conductor/src/substrate.rs`,
`crates/conductor/tests/gate.rs` (new), `docs/analysis/substrate-gate-run.md` (new). After
`substrate-client` and `substrate-tool-server` (shared `cli.yaml` and `substrate.rs`; the tool
server is not blocked upstream, so it goes first).
