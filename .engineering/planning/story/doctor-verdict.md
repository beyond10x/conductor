---
format: aep.planning-md/3
id: story:doctor-verdict
kind: story
status: draft
title: conductor doctor ends with one ready or not-ready line
relations:
- depends_on: story:gh-optional
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
revision: 2
---
## Why

In the team's `conductor doctor` runs (v0.4.0, 2026-10-13), three of the four posted outputs end on
`pins: no pins, no b10x.toml in /home/<user> or a directory above it`
(`crates/conductor/src/doctor.rs:348`), so pass or fail shows only in the exit code, and the last
line prints the home path. One engineer read the output as "there are setup issues" without
knowing which.

## Acceptance

- `conductor doctor` ends with one verdict line: `ready` when it exits 0, otherwise `not ready:`
  followed by each missing or too-old program, and prints no path under the user's home for a
  missing pin file (`pins: none`); a test in `crates/conductor/tests/doctor.rs` reads the last line
  of both cases.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`.
