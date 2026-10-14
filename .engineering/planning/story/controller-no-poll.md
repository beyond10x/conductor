---
format: aep.planning-md/3
id: story:controller-no-poll
kind: story
status: draft
title: Controllers never sleep or poll on a suite or CI run in their session
relations:
- decomposes: epic:deterministic-steps
revision: 1
---
## Why

The ess velocity study: polling in-session keeps context growing and burns usage while nothing happens; rules item 118 carries the rule until the profile does.

## Acceptance

- `.agents/repo-controller.md`: a suite or CI run starts in the background (`run_in_background` or a background `conductor build run`), the controller reports `blocked: waiting on <what>` and resumes on its end; the guard denies a controller's `sleep` loop around `gh run`/`gh pr checks`/`cargo test` with the background form; profile and guard tests.

## Scope

`.agents/repo-controller.md`, `crates/conductor/src/guard/rules.rs`, tests.
