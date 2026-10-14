---
format: aep.planning-md/3
id: story:test-fixtures-no-nested-git
kind: story
status: draft
title: Tests leave no git repository under target
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

Tests leave git repositories under `target/tmp` (for example
`target/tmp/guard_rules/guard-profile-configured/root/probe/.git`). `worktree gc` keeps a tree that
holds a nested `.git`, because removing it would delete what Git neither lists nor archives, so a
finished wave tree is retained until someone deletes `target/` by hand (wave 32, 2026-10-14).

## Acceptance

- Every test that creates a git repository does so in a temporary directory removed when the test
  ends; after `cargo test`, `find target/tmp -name .git` prints nothing; a test in
  `crates/conductor/tests/isolated_home.rs` scans the test sources for fixture repositories created
  under `target/tmp`.

## Scope

`crates/conductor/tests/common/mod.rs`, `crates/conductor/tests/guard_rules.rs`,
`crates/conductor/tests/isolated_home.rs`.
