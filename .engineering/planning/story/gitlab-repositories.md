---
format: aep.planning-md/3
id: story:gitlab-repositories
kind: story
status: implemented
title: A snapshot records a GitLab group's projects
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gitlab-source-host
- depends_on: story:gh-optional
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/src/collect/specifications.rs
- confidence: cited
  path: crates/conductor/tests/collect_gitlab.rs
- confidence: cited
  path: crates/conductor/tests/fixtures/gitlab
- confidence: cited
  path: crates/conductor/tests/grouped_instance.rs
- confidence: cited
  path: crates/conductor/tests/instance_sources.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/config.md
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:02Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:03Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:03Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

The repositories collector lists GitHub repositories only (`crates/conductor/src/collect/
repositories.rs`), and the specifications collector enumerates repositories from `gh repo list`
and `local` sources only (`crates/conductor/src/collect/specifications.rs:158`, `:269`); for a
`gitlab` source both record nothing, so the board has no rows for a GitLab instance.

## Acceptance

- A snapshot of an instance with a `gitlab` source records one repository row per project of the
  group and its subgroups (archived projects marked archived), with open issues, the newest release
  tag and its date, commits on the default branch after it and `local_checkout`, read with `glab
  api --hostname <host>`; a test in `crates/conductor/tests/collect_gitlab.rs` with a fake `glab`
  on `PATH` answering from fixtures reads two rows: `alpha` with open issues 3, release `v1.2.0`
  of 2026-10-01, 2 commits after it and `local_checkout` true; `beta` archived with
  `local_checkout` false.
- The same snapshot records one specification row per project; a test in the same file reads
  `alpha`'s row with presence `Present` and path `spec` (its checkout holds `spec/`) and `beta`'s
  with presence `Missing`.

## Notes

- The spec sentence "the collectors record nothing for this source" (`GitLabSource`,
  `spec/domains/config.yaml`) is replaced here by how the binary reads it.
- The existing gitlab cases in `crates/conductor/tests/grouped_instance.rs` (empty rows) and
  `crates/conductor/tests/instance_sources.rs` (the collectors under a gitlab source) are changed
  to the new rows, with a fake `glab` answering from fixtures.
- Depends on `story:gitlab-source-host` (`host`, the spec file) and `story:gh-optional`
  (`collect/mod.rs`); `story:gitlab-merge-requests` lands after it on `collect/gitlab.rs`,
  `collect/mod.rs` and the test file.
- Fixtures carry no 40/64-hex strings (the organization's secret scanner).

## Scope

`crates/conductor/src/collect/gitlab.rs` (new), `crates/conductor/src/collect/mod.rs`,
`crates/conductor/src/collect/repositories.rs`, `crates/conductor/src/collect/specifications.rs`,
`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/config.md`,
`crates/conductor/tests/collect_gitlab.rs` (new), `crates/conductor/tests/fixtures/gitlab` (new),
`crates/conductor/tests/grouped_instance.rs`.
