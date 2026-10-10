---
format: aep.planning-md/3
id: story:workspace-from-config
kind: story
status: draft
title: The instance's workspace comes from its config, and a missing member does not fail a snapshot
revision: 1
---
## Why

After the instance's records moved out of the product checkout, `records/.engineering/workspace.yaml`
kept member sources relative to its old place (`../../<repo>`). From the records they resolved to a
directory under the instance directory that does not exist, so the blockers collector failed to
spawn `git fetch` in the first member and every snapshot failed with it, for about two days, until
conductor reported it (2026-10-10). The paths were repaired by hand.

## Acceptance

- `conductor init` and a new `conductor config workspace` (spec first, `crates/conductor/cli.yaml`)
  write the instance's `workspace.yaml` from the config: one member per repository of the instance's
  sources, its source the checkout under `checkouts.root`. Test: a two-repository config gives the
  two members with sources that resolve to the checkouts.
- `conductor doctor` reports a member whose checkout is missing, by name. Test with a fixture
  workspace whose one member points nowhere.
- The blockers and specifications collectors record a member they cannot read (a missing checkout,
  a fetch that fails) as a per-repository failure in the snapshot and go on with the others; the
  snapshot completes. Test: one missing member among three; the snapshot completes with two
  members' blockers and one recorded failure.

## Scope

`crates/conductor/cli.yaml`, `crates/conductor/src/init.rs`, `crates/conductor/src/config.rs`,
`crates/conductor/src/doctor.rs`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/src/collect/specifications.rs`, `crates/conductor/tests/`.
