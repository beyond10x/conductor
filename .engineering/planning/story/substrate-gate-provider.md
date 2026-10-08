---
format: aep.planning-md/3
id: story:substrate-gate-provider
kind: story
status: draft
title: Large gates run as substrate execs
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/substrate.rs
revision: 4
---
## Why

Substrate fits builds better than sessions: a read-only toolchain root, no network, a `/scratch`
for `target/` and exact usage counters (docs/analysis/2026-10-09-substrate-benefits.md § 3). It cannot run an exec in an existing checkout yet,
and caps each exec at one core.

## Acceptance

- Once the upstream blocker is cleared: `task check` (or `cargo test`) can run as a substrate exec
  through `b10x-substrate-sdk`, with the usage recorded; a refusal is reported by its name.

## Scope

`crates/conductor/src/` (a substrate provider), `Cargo.toml`, `Taskfile.yml`.
