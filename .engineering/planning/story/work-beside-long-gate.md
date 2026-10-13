---
format: aep.planning-md/3
id: story:work-beside-long-gate
kind: story
status: draft
title: A controller works on its next units while a long gate runs
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 2
---
## Why

A controller waited about three hours with nothing to do while its release gate ran, until it was
stopped; the next pass of work started only when the operator asked for it (2026-10-13).

## Acceptance

- Conductor's profile tells it, when a controller's gate is expected to run longer than an hour, to
  dispatch the repository's next wave units beside it within the build-slot budget (in a separate
  tree), and the controller profile tells a controller to propose that work instead of waiting; a
  test over both profiles holds the wording.

## Scope

`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
