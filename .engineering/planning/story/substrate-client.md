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
revision: 1
---
## Why

Both substrate stories need one client; two stories creating `substrate.rs` and the dependency
collide (design and parallel-safety critics, round 1).

## Acceptance

- Spec first: `confinement.provider` gains `substrate`.
- `Cargo.toml` gains the substrate dependency the spike chose (daemon SDK or in-process host).
- `crates/conductor/src/confine/substrate.rs`: connect, run one exec, read its usage record.
- Its doctor probe: substrate reachable or loadable, the cgroup root delegated, the version.
  Tests with a fake substrate.
- `instance-usage-metrics` reads a substrate exec's usage through this client: a test with a fixture
  usage body.

## Scope

`Cargo.toml`, `crates/conductor/Cargo.toml`, `spec/domains/config.yaml`, `crates/conductor-model/`,
`crates/conductor/src/confine/substrate.rs` (new), `crates/conductor/src/usage_metrics.rs`,
`crates/conductor/tests/`. After `substrate-tool-spike`, `session-envelope-seam` and
`instance-usage-metrics`.
