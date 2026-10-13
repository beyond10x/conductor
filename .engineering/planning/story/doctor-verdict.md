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
revision: 4
---
## Why

In the team's `conductor doctor` runs (v0.4.0, 2026-10-13), three of the four posted outputs end on
`pins: no pins, no b10x.toml in /home/<user> or a directory above it`
(`crates/conductor/src/doctor.rs:348`), so pass or fail shows only in the exit code, and the line
prints the home path. One engineer read the output as "there are setup issues" without knowing
which.

## Acceptance

- `conductor doctor` ends with one verdict line: `ready` when it exits 0, otherwise `not ready: `
  followed by each program that failed, as `<program> (missing)`, `<program> (too old)` or
  `<program> (version unread)`, comma-separated; a test in `crates/conductor/tests/doctor.rs`
  reads the last line of a passing run, of a run with one missing and one unread program, and of a
  run with a pin file under which one program is too old.
- Without a pin file the pins line reads `pins: none`, with no path; the same test reads it.

## Notes

- Shares `crates/conductor/src/doctor.rs` and `crates/conductor/tests/doctor.rs` with
  `story:gh-optional` (before it) and `story:doctor-minimum-versions` (after it).
- The sample output in `website/docs/getting-started.md` is `story:gitlab-guide`'s.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`.
