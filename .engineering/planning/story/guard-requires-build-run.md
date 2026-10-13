---
format: aep.planning-md/3
id: story:guard-requires-build-run
kind: story
status: implemented
title: The guard denies a build that does not run under conductor build run
relations:
- decomposes: epic:disk-protocol
- depends_on: story:build-run
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T17:12:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T18:12:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`epic:disk-protocol`, step 5: a protocol nobody has to follow is a rule. Sessions start builds
with plain `cargo`, `task` and `npm` and nothing reserves their disk.

## What

The guard denies a controller's or conductor-dev's Bash command that runs `cargo build|test|check|
clippy|nextest|install`, `task check|gate|install|ui`, or `npm run build|test`, unless the command
is `conductor build run … -- <that command>`. The denial names the exact `conductor build run`
form with a default reservation for that command. Conductor's own session is not held (it runs no
builds). `cargo fmt` and `rustfmt` stay allowed.

## Acceptance

- `crates/conductor/tests/guard_rules.rs`: each listed command denied for a controller with the
  `conductor build run` form in the reason; the same command under `conductor build run --reserve 8G
  --` allowed; `cargo fmt --check` allowed; conductor's Bash unaffected.
- The guard's page and `.agents/repo-controller.md` name the rule.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`,
`website/docs/security/the-guard.md`.
