---
format: aep.planning-md/3
id: story:box-tool-roots
kind: story
status: draft
title: An instance declares read-only tool directories and environment for its box
summary: Toolchains, tool caches and pinned tools on the host reach a boxed runtime's sessions through declared read-only roots and environment, not through copies in the cache root
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-runtime-starts
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: website/docs/guides/run-isolated.md
revision: 1
---
## Why

Agentbox friction logs F27, F30, F33 and F35 (2026-11-15). Five NEEDs from the box's
controllers were about tools:

- `aep` had no protocol cache, because `~/.cache/aep` is not mounted.
- There was no Rust toolchain, because `~/.cargo` and `~/.rustup` are not mounted.
- A repository's pinned `ess` 0.92.0 is fetched from github.com, which the box cannot reach.

Each was worked around by copying the tool or cache into the instance's cache root, which is
mounted writable. Then `AEP_CACHE_DIR`, `CARGO_HOME`, `RUSTUP_HOME` and `PATH` went into
`<state>/claude/settings.json`, and a running box was seeded through `/proc`.

Read at v1.13.0:

- The box's mounts are a fixed list (`crates/conductor/src/placement.rs` `mounts`, the table
  above it at about `:1476-1495`).
- No config key adds a directory to read (`spec/domains/config.yaml` has only `read_only_tools`,
  which is the guard's).
- Substrate bounds one start to 4 read-only roots (`placement.rs:1310`, substrate-wire
  `MAX_READ_ONLY_ROOTS`). The binary's and the harness's directories already take 2.

## Outcome

- An instance names extra host directories its box reads (for example a toolchain home and a
  tool cache), each with the variables to set in the box's sessions.
- Conductor mounts them read-only, adds the variables to the sessions' environment, and
  refuses before substrate is asked when the read-only roots would exceed the bound
  (`placement.mounts-exceeded`).
- Doctor names each declared directory that is absent.

## Acceptance

- An instance that declares `~/.cargo` with `CARGO_HOME` gets a box request that reads that
  directory and carries the variable. One declaration too many is refused at
  `conductor config validate` with `placement.mounts-exceeded`. Both are held in
  `tests/placement.rs`.

## Boundaries

- Network access to registries and download hosts is `story:box-registry-egress`.
- With 2 of 4 read-only roots already used, more than two tool directories need either a
  parent directory that holds them or a higher substrate bound, which is substrate's to give.
