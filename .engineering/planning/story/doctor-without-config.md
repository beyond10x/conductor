---
format: aep.planning-md/3
id: story:doctor-without-config
kind: story
status: implemented
title: Doctor needs no forge CLI without a config file
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:47:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T13:47:19Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T14:20:53Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

A user ran `conductor doctor` on v0.7.0 with no config file and was told `gh: missing` and `not
ready`, though their repositories are not on GitHub. Without a config file doctor checked the
built-in instance, whose placeholder source is on GitHub (`crates/conductor/src/config.rs`,
`built_in`), so `gh` counted as needed. v0.8.0 did the same.

## What

Without a config file no source is named, so doctor needs neither forge CLI: `gh` and `glab` are
`not needed` until an instance names a `github` or `gitlab` source (`conductor setup --apply`
writes one from the repository's `origin`).

## Acceptance

- With no config file and neither `gh` nor `glab` on `PATH`, `conductor doctor` prints
  `gh: not needed` and `glab: not needed` and exits 0 when every other program is there
  (`tests/doctor.rs`, `doctor_needs_no_forge_cli_without_a_config_file`).
- A config file with a `github` source still needs `gh`, and one with a `gitlab` source `glab`.
- `website/docs/getting-started.md` says so, and its sample shows both as not needed.

## Scope

`crates/conductor/src/doctor.rs` (`read_config`), `crates/conductor/tests/doctor.rs`,
`website/docs/getting-started.md`.
