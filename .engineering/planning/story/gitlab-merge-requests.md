---
format: aep.planning-md/3
id: story:gitlab-merge-requests
kind: story
status: draft
title: A snapshot records GitLab merge requests and pipelines
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gitlab-repositories
scope:
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/tests/collect_gitlab.rs
- confidence: cited
  path: crates/conductor/tests/fixtures/gitlab
revision: 6
---
## Why

Pull requests, their checks, merged pull requests, releases and `main` workflow runs come from
GitHub only (`crates/conductor/src/collect/github.rs`), so a GitLab instance's board shows no open,
failing or merged work, no CI and no releases for `conductor repository shipped`.

## Acceptance

- A snapshot of an instance with a `gitlab` source records each open merge request as a pull
  request row, failing when its head pipeline failed; a test in
  `crates/conductor/tests/collect_gitlab.rs` reads one passing and one failing row.
- It records each merge request merged within the collector's window as a merged pull request row;
  a test in the same file reads one.
- It records each release published within the window as a release row; a test in the same file
  reads one.
- It records the newest default-branch pipeline as a workflow run, its status mapped by one
  function in `collect/gitlab.rs` to the conclusions the GitHub collector uses; a test in the same
  file reads a `failed` pipeline as `Failure` and a `success` one as `Success`.

## Notes

- Depends on `story:gitlab-repositories` (shares `collect/gitlab.rs`, `collect/mod.rs`, the test
  file and fixtures). `story:watch-gitlab-ci` reuses the status mapping and depends on this story.

## Scope

`crates/conductor/src/collect/gitlab.rs`, `crates/conductor/src/collect/mod.rs`,
`crates/conductor/tests/collect_gitlab.rs`, `crates/conductor/tests/fixtures/gitlab`.
