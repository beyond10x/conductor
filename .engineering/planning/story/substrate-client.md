---
format: aep.planning-md/3
id: story:substrate-client
kind: story
status: draft
title: Conductor reaches substrate through one client
relations:
- decomposes: epic:session-confinement
- depends_on: story:substrate-tool-spike
- depends_on: story:session-envelope-seam
- depends_on: story:instance-usage-metrics
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/substrate.rs
- confidence: cited
  path: crates/conductor/src/usage_metrics.rs
- confidence: cited
  path: crates/conductor/tests/substrate_client.rs
revision: 4
---
## Why

Both substrate stories need one client; two stories creating it would collide (design and
parallel-safety critics, round 1). It is a library, not a provider: the harness stays in the
`systemd` provider, and substrate runs only commands (design critic, round 2).

## Acceptance

- `crates/conductor/src/substrate.rs`, on the dependency the spike chose (daemon SDK or in-process
  host): connect, run one exec, read its usage record. Tests with a fake substrate
  (`crates/conductor/tests/substrate_client.rs`): an exec's exit and output come back; its usage
  record is parsed; a refusal comes back as a named error.
- Its doctor probe: substrate reachable or loadable, the cgroup root delegated, the version. A test
  with the fake reachable and one with it absent.
- `instance-usage-metrics` reads a substrate exec's usage through this client: a test with a fixture
  usage body.

## Scope

`Cargo.toml`, `crates/conductor/Cargo.toml`, `crates/conductor/src/substrate.rs` (new),
`crates/conductor/src/usage_metrics.rs`, `crates/conductor/src/lib.rs`,
`crates/conductor/tests/substrate_client.rs` (new). After `substrate-tool-spike`,
`session-envelope-seam` and `instance-usage-metrics` (`usage_metrics.rs`, `lib.rs`).
