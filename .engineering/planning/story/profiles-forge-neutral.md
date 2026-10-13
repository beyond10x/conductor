---
format: aep.planning-md/3
id: story:profiles-forge-neutral
kind: story
status: implemented
title: The profiles name glab beside gh
relations:
- decomposes: epic:forge-neutral
- depends_on: story:guard-glab-writes
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:42:59Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:42:59Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:42:59Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

The conductor and controller profiles speak of GitHub only: `gh`, pull requests and Actions
(`.agents/conductor.md`, `.agents/repo-controller.md`). A controller of a GitLab repository is told
to use a tool that does not reach its forge.

## Acceptance

- Each of `.agents/conductor.md` and `.agents/repo-controller.md` contains the sentence "On GitLab,
  a pull request is a merge request, `gh` is `glab` and a workflow run is a pipeline.", and every
  line of either that names a `gh` command names its `glab` counterpart on the same line; a test in
  `crates/conductor/tests/profiles.rs` holds both.

## Notes

- The controller profile says `gh` is read-only for a controller; the `glab` counterpart is held by
  `story:guard-glab-writes`, on which this story depends so the profile does not promise what the
  guard does not hold.

## Scope

`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
