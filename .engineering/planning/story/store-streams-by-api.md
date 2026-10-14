---
format: aep.planning-md/3
id: story:store-streams-by-api
kind: story
status: draft
title: The store lists its streams through eventlog's API, in any tree layout
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

eventlog 0.9.0 adds an opt-in tree layout, `eventlog-tree/3` (`TreeLayout::Hashed`), whose stream
directories are hashed rather than `tenants/<t>/streams/<type>/`. `crates/conductor/src/store/migrate.rs`
(lines 403-410) probes that path directly, so a store in the new layout would show no streams
(eventlog's report to conductor, 2026-10-14). Default stores stay on `/2`.

## Acceptance

- `store/migrate.rs` lists streams through `eventlog-tree`'s own API instead of walking the
  directory layout; a test over a fixture store in each layout (`/2` and `/3`) lists the same
  streams.

## Scope

`crates/conductor/src/store/migrate.rs`, `crates/conductor/tests/store_tree.rs`, `Cargo.toml`.
