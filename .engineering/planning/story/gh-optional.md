---
format: aep.planning-md/3
id: story:gh-optional
kind: story
status: draft
title: conductor runs without gh for a workspace with no GitHub repository
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/src/collect/github.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: website/docs/getting-started.md
revision: 2
---
## Why

Engineers of a team trying conductor v0.4.0 (2026-10-13) ran `conductor doctor`; every posted run
exited 1, and `gh` was missing on all four that posted output. One asked to drop the dependency:
the team keeps no repository on GitHub. `gh` is one of the eight `PROGRAMS` in
`crates/conductor/src/doctor.rs:24`, but only the GitHub collectors (`crates/conductor/src/collect/
github.rs`, and the GitHub rows of `repositories.rs` and `specifications.rs`) and the watch's CI
line call it.

## Acceptance

- With a workspace whose members name no GitHub repository, `conductor doctor` reports a missing
  `gh` as `not needed: no GitHub repository in the workspace` and exits 0 when every other program
  is present; a test in `crates/conductor/tests/doctor.rs` with a fixture workspace and a `PATH`
  without `gh` holds it.
- With such a workspace, a snapshot completes without `gh` on `PATH`, recording no GitHub rows; a
  test in `crates/conductor/tests/` holds it.

## Notes

- With at least one GitHub member, `gh` stays required, as now.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/src/collect/mod.rs`,
`crates/conductor/src/collect/github.rs`, `crates/conductor/tests/doctor.rs`, `README.md`,
`website/docs/getting-started.md`.
