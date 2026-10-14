---
format: aep.planning-md/3
id: story:confined-tutorial
kind: story
status: draft
title: The public site teaches running conductor confined on Linux, by default
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-forge-credentials
revision: 1
---
## Why

The public guide "Run conductor isolated" (`website/docs/guides/run-isolated.md`) says conductor has
no operating-system sandbox; engineers ask for one before they adopt it (the operator,
2026-10-14). Once `epic:session-confinement` ships on Linux, the site needs a tutorial that takes an
engineer from install to a confined instance and shows the box holding.

## Acceptance

- A guide `website/docs/guides/run-confined.md` (Linux) walks from a fresh install to a confined
  instance: prerequisites (`bwrap`, systemd user session; `conductor doctor`'s `systemd fence
  (bwrap)` row), `confinement.mode: strict` in the config, starting conductor and a controller,
  and three checks the reader runs to see the box: a write outside the session's places fails, the
  unit's limits are listed by `systemctl --user show`, and a forge write without the bot route is
  refused; it says plainly what is not confined (reads, network) and that macOS and Windows come
  later. `run-isolated.md` points at it, and the getting-started page links it.
- Every command in the guide was run on a clean Linux account against the released binary, and the
  run is recorded in `docs/analysis/confined-tutorial-run.md`; `task docs-check` and the site build
  pass.
- `confinement.mode` defaults to `strict` on Linux when `bwrap` and a systemd user session are
  present (`report` otherwise), stated in `docs/config.md`; a config test holds the default.

## Scope

`website/docs/guides/run-confined.md` (new), `website/docs/guides/run-isolated.md`,
`website/docs/getting-started.md`, `docs/config.md`, `crates/conductor/src/config.rs`,
`crates/conductor/tests/config.rs`, `docs/analysis/confined-tutorial-run.md` (new).
