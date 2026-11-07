---
format: aep.planning-md/3
id: story:k5-tags-and-no-release-repositories
kind: story
status: draft
title: K5 counts a semver tag as a release and skips repositories that ship without releases
summary: K5.unreleased counts what no release object or decision can lower
revision: 2
---
## Why

Observed 2026-11-07 on an instance whose repositories are on a self-hosted GitLab: K5.unreleased
read 802 and nothing done there can lower it. Three repositories release by semver tag only, with
no forge Release object; five ship without releases on purpose (docs published on merge,
specifications and end-to-end tests consumed at `main`, two records repositories), which the
instance recorded as a decision in its store. Those five hold 741 of the merged pull requests.

Verified in the code: K5 counts a pull request released only by a forge release published after
its merge (`k5`, `crates/conductor/src/metric.rs:985`, over `forge.releases`); the collectors
record releases only (`crates/conductor/src/collect/github.rs:24`, `collect/gitlab.rs:38`), no tags,
and no input lets K5 skip a repository.

## Acceptance

- A semver tag (`v<major>.<minor>.<patch>`) pushed after a pull request's merge counts as its
  release for K5.median, K5.p90 and K5.unreleased, in a repository with no Release objects.
- A repository the config marks as shipping without releases (a per-repository key, with the
  reason) is left out of every K5 reading, and `metric compute` prints it as skipped once.
- A test over a snapshot fixture with one tag-only repository and one marked repository reads
  K5.unreleased counting neither's merged pull requests.
