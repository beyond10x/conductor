---
format: aep.planning-md/3
id: story:gitlab-source-host
kind: story
status: implemented
title: A gitlab source names its host
relations:
- decomposes: epic:forge-neutral
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/grouped_instance.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/config.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:42:54Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:42:54Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:42:54Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

`conductor.config.GitLabSource` holds only `group` and `exclude`, and says the binary does not read
that forge (`spec/domains/config.yaml`). A self-hosted GitLab needs its host named, and the
collector, workspace and watch stories of `epic:forge-neutral` read the source.

## Acceptance

- `spec/domains/config.yaml` gives `GitLabSource` an optional `host` (absent: `gitlab.com`);
  `ess specify validate --path spec` passes and the regenerated `crates/conductor-model` and
  `website/docs/reference/config.md` match (`task check`'s model and docs checks).
- `conductor config show` prints a `gitlab` source's `host`, `gitlab.com` when the file omits it;
  a test in `crates/conductor/tests/config.rs` holds both, and the existing gitlab case in
  `crates/conductor/tests/grouped_instance.rs` is updated to the new line.

## Notes

- Spec first: every other story of the epic reads `host`, so this lands first. The spec sentence
  "the collectors record nothing for this source" changes in `story:gitlab-repositories`, when it
  stops being true.
- `story:gh-optional` and `story:gitlab-workspace` also land on `spec/domains/config.yaml`, the
  generated model and reference page; both depend on this story.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/config.md`,
`crates/conductor/src/config.rs`, `crates/conductor/tests/config.rs`,
`crates/conductor/tests/grouped_instance.rs`.
