---
format: aep.planning-md/3
id: story:work-beside-long-gate
kind: story
status: draft
title: A controller works on its next units while a long gate runs
relations:
- decomposes: epic:conductor-autonomy
- depends_on: story:resumable-gates
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 3
---
## Why

A controller waited about three hours with nothing to do while its release gate ran, until it was
stopped; the next pass of work started only when the operator asked for it (2026-10-13).

## Acceptance

- Conductor's profile tells it, when a controller's gate is expected to run longer than an hour, to
  dispatch the repository's next wave units beside it within the build-slot budget (in a separate
  tree), and the controller profile tells a controller to propose that work instead of waiting; a
  test in `crates/conductor/tests/profiles.rs` holds the wording of both.

## Notes

- Shares `.agents/conductor.md`, `.agents/repo-controller.md` and
  `crates/conductor/tests/profiles.rs` with the other profile stories of `epic:conductor-autonomy`;
  it depends on `story:resumable-gates` so it lands after it on those files.

## Scope

`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
