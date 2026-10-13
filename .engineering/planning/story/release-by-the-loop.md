---
format: aep.planning-md/3
id: story:release-by-the-loop
kind: story
status: draft
title: Merged work is released by the loop, and merged-unreleased age is on the board
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

p90 22 hours from merge to release and 44 merged PRs never released. (`docs/design/self-improvement.md`.)

## Acceptance

- The board lists each repository's merged-unreleased PRs with their age; the controller profile releases a wave's merged set within the wave unless its charter says releases are decided; tests over a snapshot fixture and the profile test.

## Scope

`crates/conductor/src/status.rs`, `.agents/repo-controller.md`, tests.
