---
format: aep.planning-md/3
id: story:gitlab-workspace
kind: story
status: implemented
title: conductor config workspace writes a gitlab source's checkouts
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gitlab-source-host
- depends_on: story:gitlab-repositories
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/tests/config_workspace.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/config.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:05Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:05Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:06Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Why

`conductor config workspace` writes no member for a `gitlab` source (`spec/domains/config.yaml`,
`WriteWorkspace`), so aep's workspace views see none of a GitLab instance's checkouts.

## Acceptance

- For a `gitlab` source, `conductor config workspace` writes one member per git checkout under
  `checkouts.root` whose `origin` remote is on the source's host and under its group, but its
  `exclude` entries. A test in `crates/conductor/tests/config_workspace.rs` with four fixture
  checkouts (on the group; on another host with the same group path; on another group of the same
  host; on the group but excluded) finds exactly the first, with a `glab` and a `gh` on `PATH` that
  fail when called, so no network is read.

## Notes

- The `WriteWorkspace` comment in `spec/domains/config.yaml` changes with it, which regenerates
  `crates/conductor-model` and `website/docs/reference/config.md`: shared with
  `story:gitlab-source-host`, on which it depends, and with `story:gh-optional` and
  `story:gitlab-repositories`, after which it lands.
- The hand-written page `website/docs/concepts/instances-and-config.md` is `story:gitlab-guide`'s.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/config.md`,
`crates/conductor/src/config.rs`, `crates/conductor/tests/config_workspace.rs`.
