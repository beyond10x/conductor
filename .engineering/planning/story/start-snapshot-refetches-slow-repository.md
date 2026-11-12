---
format: aep.planning-md/3
id: story:start-snapshot-refetches-slow-repository
kind: story
status: draft
title: The snapshot at a pod's start fetches a repository that does not answer several times
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

A run on a local cluster (2026-11-12, k3d, a forge double that held one repository's answers
for 900 s) showed the snapshot an instance pod takes at its start still collecting 263 s after
the start: `snapshot start-snapshot` fetched that one repository again about every 120 s, and
the double logged 3 held requests. Each fetch was stopped at its own 120 s bound and the
failure was in `GET /v1/instances/{name}/health` 123 s after the start, so nothing was wrong in
what a caller reads; but one repository that does not answer costs the start snapshot several
bounds, and the first complete snapshot of that start arrives that much later.

Not read from the code: which steps of one snapshot fetch the same checkout more than once
(the pod's clone-and-fetch pass and the collectors are the candidates).

## Acceptance

- Within one snapshot a checkout whose fetch ran into its bound is not fetched again: a test
  with a fetch that never answers counts one fetch of that checkout in the snapshot and reads
  the member failure once.
- The snapshot at a pod's start with one such repository and one that answers ends within one
  fetch bound plus the time the other takes; the test states the bound it measured against.

## Notes

- Found by a cluster run of the pod's clone, introduced with it, not a defect in what health
  answers: it waits in the backlog.
- The same run: the daemon's log labels the pod replacement of an upgrade (a changed pod
  digest) `Restart (Cadence): Succeeded`. A reader takes it for a scheduled restart. Name the
  cause in that line where this story touches the reconciler's log, or split it off.
