---
format: aep.planning-md/3
id: story:guard-store-migrate
kind: story
status: draft
title: The guard denies conductor store migrate, a write that passes as a read
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 2
---
## Why

The guard denies a controller every `conductor` write command, but not `conductor store migrate`:
its group is missing from the guard's list of `conductor` command groups in
`crates/conductor/src/guard/rules.rs`, and `tests/guard_rules.rs` treats it as a read because its
only option is `--format`. It copies the store into a new layout, so it writes the state directory.
Found by the docs overhaul's security review, 2026-10-12 (`website/docs/security/limits.md`).

## Acceptance

- The guard denies `conductor store migrate` to a controller and to conductor, as it denies every
  other write command, naming the command; a case in `tests/guard_rules.rs` holds it, and the
  guard's read-command list is derived so that a new write command cannot pass as a read.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`.
