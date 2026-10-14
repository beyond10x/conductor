---
format: aep.planning-md/3
id: story:governed-at-depth
kind: story
status: draft
title: trust and spawn find governed repositories at any depth under the checkouts root
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

The operator, 2026-10-14, from a second instance: its seven governed repositories sit two levels
below `checkouts.root` (`<root>/<group>/<repo>`). `conductor trust` (`crates/conductor/src/trust.rs`,
`paths()`) marks only git checkouts directly under the root, so `task trust` marked 52 unrelated
checkouts and none of the governed ones; `conductor spawn` (`spawn.rs`, `checkout()`) refuses a path
whose real path is outside the root, so a folder of symlinks is no way around it.

## Acceptance

- `conductor trust` marks exactly the checkouts of the repositories the instance governs (its
  workspace members and charters), found at any depth under `checkouts.root`, plus the records
  directory, and no other checkout; a test over a fixture root with governed repositories at
  `<root>/<group>/<repo>` and unrelated ones at depth 1 holds it.
- `conductor spawn` starts a controller in a governed repository found at depth 2; a test with a
  fake `claude` holds it. The workspace file (`conductor config workspace`) lists such members.

## Scope

`crates/conductor/src/trust.rs`, `crates/conductor/src/spawn.rs`, `crates/conductor/src/config.rs`,
`crates/conductor/tests/trust.rs`, `crates/conductor/tests/spawn.rs`.
