---
format: aep.planning-md/3
id: story:red-ci-owner
kind: story
status: draft
title: The watch finds the owner of every red check and times it
relations:
- decomposes: epic:red-ci-ownership
- depends_on: story:pr-check-names
revision: 1
---
## Why

Rules item 125 by hand (epic:red-ci-ownership): conductor has to notice an unowned red check, find
or spawn the owner and time it, every cycle, from `gh` output.

## Acceptance

- Spec first: thresholds `red_ack` (one watch cycle by default), `red_fix` (4h) and `red_stale`
  (24h); a red record per repository and target (`main` or a pull request) with its first-seen time,
  its red checks by name, its owner (a dispatch id or none) and its state (unowned, owned,
  acknowledged, late, stale); `ess specify validate` exits 0.
- The watch prints `red ci: <repo> <main|#n> unowned` once when a red target has no open dispatch
  in that repository, `... late` past `red_fix` without a new head, and `... stale` past
  `red_stale` with no push; `conductor status` lists every red target with owner and age. Tests with
  fake `gh` hold each line.
- `.agents/conductor.md` says an `unowned` line is answered in that cycle by dispatching (or
  spawning) the repository's controller with "make CI green"; `.agents/repo-controller.md` says
  red CI comes ahead of new units, is acknowledged with `started red-ci`, and that nothing else
  merges while `main` is red. Profile tests hold both.

## Scope

`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `crates/conductor-model`,
`crates/conductor/src/watch.rs`, `crates/conductor/src/status.rs`, `crates/conductor/tests/watch.rs`,
`crates/conductor/tests/status.rs`, `.agents/conductor.md`, `.agents/repo-controller.md`,
`crates/conductor/tests/profiles.rs`, `docs/config.md`.
