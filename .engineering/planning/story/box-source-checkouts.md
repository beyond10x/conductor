---
format: aep.planning-md/3
id: story:box-source-checkouts
kind: story
status: draft
title: A boxed runtime clones its sources' missing checkouts, and doctor names the ones still missing
summary: The pod's clone-at-start and its doctor count extend to a substrate box, which now writes its checkouts root and reaches its forges
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-forge-work
- informed_by: story:pod-source-checkouts
scope:
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
revision: 1
---
## Why

Agentbox friction logs F13 and F17 (2026-11-15). Nothing clones at a box placement: the
operator cloned all 8 repositories on the host by hand. `conductor doctor` said ready while
every snapshot failed. Read at v1.13.0:

- Only a runtime alone in a pod clones the repositories a source includes
  (`crates/conductor/src/collect/checkouts.rs:1-10`). `story:box-forge-work` leaves cloning out
  on purpose ("the operator clones them on the host"). That story's own changes removed the
  reasons: the box now writes the checkouts root (`checkouts-<instance>`) and reaches each
  forge through `forge_<host>`.
- Doctor counts included repositories that have no checkout only in an instance pod
  (`crates/conductor/src/doctor.rs:2238-2245`). `story:box-forge-work` added rows for the box's
  forge aperture, git identity and trust (`doctor.rs:1652-1700`), which cover the rest of F17.
  It added no row for missing checkouts.

## Outcome

- A runtime in a box clones each included repository that has no checkout, at its start and at
  each `repositories` collector pass, as a pod's runtime does.
- `conductor doctor` prints, for a boxed instance, the included repositories that have no
  checkout under its checkouts root.

## Acceptance

- A boxed instance whose `gitlab` source includes a repository with no checkout has one after
  its runtime starts, and before that start doctor names the repository as having no checkout.
  Tested against a fake forge and a fake substrate.
