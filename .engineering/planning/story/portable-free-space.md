---
format: aep.planning-md/3
id: story:portable-free-space
kind: story
status: implemented
title: Free disk space is read without GNU df
tags:
- first-user-review
scope:
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T19:55:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T19:55:35Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. `snapshot start-snapshot` failed: `df -B1 --output=avail` → "df: invalid option -- B"
(exit 64). The same call is in `crates/conductor/src/snapshot.rs` (~1423), `crates/conductor/src/dashboard.rs`
(~754) and `crates/conductor/src/config.rs` (the disk reading). The user's workaround was a PATH
shim translating GNU flags.

## Acceptance

- Free space on a path is read through `statvfs` (a Rust crate such as `rustix` or `nix`; no
  external `df`), in one function every caller uses.
- A test reads the free space of a temporary directory and checks it against `statvfs` directly.
- Nothing in `crates/` runs `df`.

## Scope

`crates/conductor/src/snapshot.rs`, `crates/conductor/src/dashboard.rs`, `crates/conductor/src/config.rs`, `crates/conductor/Cargo.toml`.
