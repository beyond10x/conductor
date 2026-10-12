---
format: aep.planning-md/3
id: story:workspace-from-config
kind: story
status: implemented
title: conductor config workspace writes the instance's workspace from its config
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/init.rs
- confidence: cited
  path: crates/conductor/tests/config_workspace.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-11T23:59:51Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-11T23:59:51Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-11T23:59:51Z", actor: "human:timo", revision: 7, decided_on: {"asserted":{"test_result":1}}}
---
## Why

After the instance's records moved out of the product checkout, `records/.engineering/workspace.yaml`
kept member sources relative to its old place (`../../<repo>`). From the records they resolved to a
directory under the instance directory that does not exist, so the blockers collector failed to
spawn `git fetch` in the first member and every snapshot failed with it, for about two days, until
conductor reported it (2026-10-10). The paths were repaired by hand. This story writes the file from
the config; `story:snapshot-absent-member` makes a missing member harmless.

Narrowed 2026-10-12 after scoping: doctor's member report is `story:doctor-workspace-members`, a
typed per-repository failure record is `story:snapshot-member-failures`.

## Acceptance

- A new `conductor config workspace` (spec first: its input beside `ShowConfig` in
  `spec/domains/config.yaml`, a `local` callable and `[config, workspace]` command in
  `crates/conductor/cli.yaml`, the clap variant in `src/cli.rs`) writes
  `<records>/.engineering/workspace.yaml` (`version: aep.workspace/1`, `members:` of `name` and
  `source`), replacing the file if it exists, with every `source` relative to that `.engineering/`
  directory. It reads no network:
  - a `github` source: one member per git checkout directly under `checkouts.root` (the spec's
    `GitHubSource` has no `exclude:`);
  - a `local` source: one member per git checkout directly under its `path`, minus its `exclude:`;
  - a `gitlab` source: none (the binary does not read that forge, design § 4);
  - plus the records directory itself as a member, as the hand repair has it.
  Test: a config with two checkouts under a temp `checkouts.root` and one local source gives three
  members plus the records, and `aep plan workspace members` resolves each to `present`.
- `--check` writes nothing and exits 1 naming each member that differs from the file. Test.
- `conductor init` writes the same file in the skeleton it creates. Test.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model/`, `crates/conductor/cli.yaml`,
`crates/conductor/src/cli.rs`, `crates/conductor/src/config.rs`, `crates/conductor/src/init.rs`,
`crates/conductor/tests/config_workspace.rs`, `website/docs/reference/`, `docs/config.md`.
