---
format: aep.planning-md/3
id: story:resumable-gates
kind: story
status: draft
title: A long gate reruns only what failed
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

A repository's pre-release gate ran 8–18 hours per attempt and failed four times on test-harness
defects, never on product code; each fix restarted the whole suite from its first step, so a release
prepared on 2026-10-11 was still in its gate on 2026-10-13 (conductor's reports, 2026-10-13).

## Acceptance

- The controller profile and conductor's dispatch rules require a resumable gate for any gate over
  an hour: after a failure, the controller reruns only the failed steps and the steps that depend on
  them, and keeps one full run for the release commit; conductor's charter template states it, and a
  test over `.agents/repo-controller.md` holds the wording.

## Scope

`.agents/repo-controller.md`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
