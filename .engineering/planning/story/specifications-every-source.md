---
format: aep.planning-md/3
id: story:specifications-every-source
kind: story
status: implemented
title: The specifications view has a row per repository of every source
tags:
- first-user-review
scope:
- confidence: cited
  path: crates/conductor/src/collect/specifications.rs
- confidence: inferred
  path: crates/conductor/tests/collect_stores.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T19:48:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T19:48:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. The specifications view was empty for a repository with `ess/system.yaml` and
`ess/ess-inputs.yaml`: `crates/conductor/src/collect/specifications.rs` `collect_from` (~171)
enumerates repositories only through `gh repo list <owner>` for `github:` sources, so `local:`
sources never get a row.

## Acceptance

- The specifications collector takes the repository list the repositories collector uses (every
  source), keeping `gh` only for the archived flag; each repository gets one row with presence
  Present, OptedOut or Missing.
- A test with a `local:` source holding one repository with a specification and one without.

## Scope

`crates/conductor/src/collect/specifications.rs`, `crates/conductor/src/collect/repositories.rs`, `crates/conductor/tests/`.
