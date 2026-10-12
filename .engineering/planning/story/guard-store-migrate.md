---
format: aep.planning-md/3
id: story:guard-store-migrate
kind: story
status: draft
title: The guard denies conductor store migrate, a write that passes as a read
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 5
---
## Why

The guard denies a controller every `conductor` write command, but not `conductor store migrate`:
its group is missing from the guard's list of `conductor` command groups (`GROUPS`,
`crates/conductor/src/guard/rules.rs:636`; `READ_LEAVES` at `rules.rs:662` is the allowlist), and
`tests/guard_rules.rs` treats it as a read because its only option is `--format`. It copies the
store into a new layout, so it writes the state directory. Found by the docs overhaul's security
review, 2026-10-12 (`website/docs/security/limits.md`).

A hand-kept group list will miss the next new group the same way, so the check walks the CLI
definition itself (`conductor_cli::cli::Cli` through clap's `CommandFactory`, as
`crates/conductor/tests/cli_tree.rs` already does).

## Acceptance

- Every leaf of the `conductor` CLI declares in its definition whether it writes (one list or
  attribute in `crates/conductor/src/cli.rs`, independent of its arguments), and a test in
  `crates/conductor/tests/guard_rules.rs` walks the CLI tree and fails when a leaf is undeclared or
  when the guard's read leaves differ from the leaves declared as not writing; `store migrate` is
  declared as writing and is denied to a controller.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`.
