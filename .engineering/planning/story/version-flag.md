---
format: aep.planning-md/3
id: story:version-flag
kind: story
status: implemented
title: conductor --version prints the installed version
relations:
- depends_on: story:doctor-minimum-versions
scope:
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/tests/cli_tree.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:01Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:01Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:02Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`conductor --version` is refused ("unexpected argument '--version' found", 2026-10-13, after the
v0.5.0 install), so nobody can check which conductor is installed; a team trying v0.4.0 hit the
same. `ess`, `aep` and `worktree` all answer `--version`.

## Acceptance

- `conductor --version` prints `conductor <version>` from the crate version and exits 0; a test in
  `crates/conductor/tests/cli_tree.rs` holds it, and the tree comparison there names `--version`
  on the root as a harness flag beside `--help`, not a modelled command.

## Notes

- `ess-cli/1` declares no version global (`crates/conductor/cli.yaml` `globals`); asking ESS for
  one is a follow-up for conductor to pass on. Until then the flag is clap's own.
- The generated `website/docs/reference/cli.md` changes with it (`conductor-docs generate`).

## Scope

`crates/conductor/src/cli.rs`, `crates/conductor/tests/cli_tree.rs`,
`website/docs/reference/cli.md`.
