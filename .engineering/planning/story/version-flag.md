---
format: aep.planning-md/3
id: story:version-flag
kind: story
status: draft
title: conductor --version prints the installed version
revision: 1
---
## Why

`conductor --version` is refused ("unexpected argument '--version' found", 2026-10-13, after the
v0.5.0 install), so nobody can check which conductor is installed without reading cargo's install
log; `ess`, `aep` and `worktree` all answer `--version`.

## Acceptance

- `conductor --version` prints `conductor <version>` from the crate version and exits 0; a test in
  `crates/conductor/tests/` holds it.

## Notes

- `tests/cli_tree.rs` holds the tree to exactly the specification's `cli:` block and the binding's
  globals, so the flag is modelled first: whether `ess-cli/1` (`crates/conductor/cli.yaml`) can
  declare a version global, else an exception named in `cli_tree.rs`; the generated reference page
  (`website/docs/reference/cli.md`) follows.

## Scope

`crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`, `crates/conductor/tests/cli_tree.rs`,
`website/docs/reference/cli.md`.
