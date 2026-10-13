---
format: aep.planning-md/3
id: story:doctor-without-config
kind: story
status: draft
title: Doctor needs no forge CLI without a config file
revision: 1
---
## Why

A team engineer ran `conductor doctor` on v0.7.0 with no config file and was told `gh: missing`
and `not ready`, though the team has no repository on GitHub. Without a config file doctor checked
the built-in instance, whose placeholder source is on GitHub (`crates/conductor/src/config.rs`,
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
