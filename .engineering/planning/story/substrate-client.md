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
  path: crates/conductor-model
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
- confidence: cited
  path: docs/analysis/substrate-client-run.md
- confidence: cited
  path: spec/domains/config.yaml
revision: 7
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
- Staging, by the operator's choice B of 2026-10-09 (no harness change). Substrate binds
  directories only, read-only, one `read_only_roots` entry `{host_path, mount}` each, up to
  `MAX_READ_ONLY_ROOTS` (`substrate-wire/src/lib.rs:2929`); a single file cannot be bound (substrate
  ADR 0010, confirmed by its controller 2026-10-09). So the client builds a staging directory under
  the instance's cache: `stage/bin/` with copies of the host programs a role's gate needs (`ess`,
  `aep`, `claude`; the list is config, spec first in `spec/domains/config.yaml`) and `stage/cargo/`
  with a copy of the machine's `~/.cargo/config.toml`, refreshed when a source is newer, and binds
  both directories read-only. The exec's `PATH` includes the staged `bin`, and its cargo reads the
  staged config.
  Tests with the fake substrate: the exec request carries the two read-only roots; a program
  missing on the host refuses the exec by name; a newer source is copied again. A recorded run in
  `docs/analysis/substrate-client-run.md`: `cargo test --offline -p conductor-cli` inside an exec
  runs the tests that need `ess`, `aep` and `claude` (64 failed without them in
  `docs/analysis/substrate-tool-spike.md`), and `target/` stays near the host build's size.
- Its doctor probe: substrate reachable or loadable, the cgroup root delegated, the version. A test
  with the fake reachable and one with it absent.
- `instance-usage-metrics` reads a substrate exec's usage through this client: a test with a fixture
  usage body.

## Scope

`Cargo.toml`, `crates/conductor/Cargo.toml`, `spec/domains/config.yaml`, `crates/conductor-model/`,
`docs/analysis/substrate-client-run.md` (new), `crates/conductor/src/substrate.rs` (new),
`crates/conductor/src/usage_metrics.rs`, `crates/conductor/src/lib.rs`,
`crates/conductor/tests/substrate_client.rs` (new). After `substrate-tool-spike`,
`session-envelope-seam` and `instance-usage-metrics` (`usage_metrics.rs`, `lib.rs`).


