---
format: aep.planning-md/3
id: story:doctor-minimum-versions
kind: story
status: implemented
title: conductor doctor refuses an aep, ess or worktree older than its build's pins
relations:
- depends_on: story:doctor-verdict
scope:
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/build.rs
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:42:56Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:42:57Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:42:57Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

A team member's `conductor doctor` (v0.4.0, 2026-10-13) printed `aep: present, aep 0.63.1` with no
warning while conductor's release pinned aep 0.71: without a `b10x.toml` in the directory or above
it, doctor compares no version (`crates/conductor/src/doctor.rs:251`, `examine(pins)`), so an old
`aep` that cannot open a current planning store passes.

## Acceptance

- Without a `b10x.toml`, `conductor doctor` compares `aep`, `ess` and `worktree` with the minimum
  versions built into the binary from the product repository's own `b10x.toml` at build time, and
  reports an older one as `too old` and exits 1; a test in `crates/conductor/tests/doctor.rs` holds
  three cases, a fake `aep` printing `aep 0.63.1`, a fake `ess` printing `ess 0.58.0` and a fake
  `worktree` printing `worktree 0.17.0`, each `too old` against the pins of the repository's
  `b10x.toml`.
- The `about` line of `conductor doctor` in `crates/conductor/cli.yaml` says the comparison is
  with the pin file or else the build's own pins, and the generated `website/docs/reference/cli.md`
  matches (`task check`'s docs check).

## Notes

- Shares `crates/conductor/src/doctor.rs` and `crates/conductor/tests/doctor.rs` with
  `story:doctor-verdict`, on which it depends.
- `crates/conductor/build.rs` is new; `crates/conductor/Cargo.toml` names it.
- `story:version-flag` also regenerates `website/docs/reference/cli.md`; it depends on this story.
- The pin wording in `README.md`, `website/docs/getting-started.md` and
  `website/docs/guides/troubleshoot.md` is `story:gitlab-guide`'s.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/build.rs` (new), `crates/conductor/Cargo.toml`,
`crates/conductor/tests/doctor.rs`, `crates/conductor/cli.yaml`, `website/docs/reference/cli.md`.
