---
format: aep.planning-md/3
id: story:hostile-listing-path-fails-collector
kind: story
status: draft
title: One refused project path in a forge listing fails the whole repositories collector
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

A security review of the pod's clone (2026-11-12, `review-result:pod-checkouts-security-round-1`,
finding 4) read that one project path with a `..` step or a newline in a GitLab listing fails
the whole `repositories` collector for every source of the instance
(`crates/conductor/src/collect/gitlab.rs`, the path check near line 610 at `917848c1e`). The
check was there before the review's commits. A forge that answers one bad path, by fault or on
purpose, leaves the instance with no repository facts at all for that pass, also for its
sources on other forges.

## Acceptance

- A listed project whose path a check refuses is left out with one member failure that names
  the source and the refused path with its control characters escaped; the other projects of
  that listing and every other source are collected in the same pass. A test lists one bad
  and two good projects on one source beside a second source and reads both the facts and the
  member failure.

## Notes

- Found by a review, pre-existing, not part of the reviewed change: it waits in the backlog.
- The fix of that review's finding 1 leaves a `.git` step out the same way; read what it built
  before changing the check.
