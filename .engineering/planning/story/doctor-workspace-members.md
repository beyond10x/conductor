---
format: aep.planning-md/3
id: story:doctor-workspace-members
kind: story
status: draft
title: conductor doctor names a workspace member whose checkout is absent
revision: 1
---
## Why

Split from `story:workspace-from-config` (scoped 2026-10-12). `conductor doctor` takes no input and
reports only programs on PATH (`crates/conductor/cli.yaml:69-73`, `src/doctor.rs:325`), so a
workspace member whose checkout is missing is found only when a snapshot skips it.

## Acceptance

- `conductor doctor --instance <name>` (spec first: an input and a member row type beside
  `Prerequisite` in `spec/domains/config.yaml`) also lists each member of the instance's workspace
  whose checkout is absent, by name, and exits 1 when there is one. Test with a fixture workspace
  whose one member points nowhere.
- Without `--instance`, doctor behaves as today. Test.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model/`, `crates/conductor/cli.yaml`,
`crates/conductor/src/cli.rs`, `crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`.
