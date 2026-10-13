---
format: aep.planning-md/3
id: epic:forge-neutral
kind: epic
status: draft
title: Conductor works equally well on GitLab and needs GitHub only for a GitHub source
revision: 1
---
## Why

Conductor reads only GitHub. A `gitlab` source is accepted in the config but "the collectors
record nothing for this source" (`spec/domains/config.yaml`, `GitLabSource`); `gh` is a required
program (`crates/conductor/src/doctor.rs:24`); the watch reads CI through `gh` for the first
`github` owner (`crates/conductor/src/watch.rs:241`); the guard knows `gh` writes and not `glab`
ones (`crates/conductor/src/guard/rules.rs:1781`). A team trying conductor (2026-10-13) keeps its
repositories on a self-hosted GitLab and asked to drop the GitHub dependency; the operator asked
for conductor to be independent of GitHub and to work equally well with GitLab (2026-10-13).

## Outcome

An instance whose repositories live on a GitLab forge, gitlab.com or self-hosted, gets the same
snapshot rows, board, workspace, watch lines and guard as a GitHub instance, through `glab`; and
`gh` is needed only by an instance with a `github` source.

## Stories

`story:gitlab-source-host`, `story:gitlab-repositories`, `story:gitlab-merge-requests`,
`story:gitlab-workspace`, `story:watch-gitlab-ci`, `story:guard-glab-writes`,
`story:gh-optional`, `story:profiles-forge-neutral`, `story:gitlab-guide`.
