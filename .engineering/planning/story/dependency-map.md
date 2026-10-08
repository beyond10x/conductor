---
format: aep.planning-md/3
id: story:dependency-map
kind: story
status: draft
title: Dependency map from manifests
revision: 1
---
# Dependency map from manifests

## What

Conductor asked to know which repository of the instance depends on which, read from manifests
rather than declared: Cargo `git` dependencies on the instance's own repositories (with the pinned
tag or rev), CI workflow and `Taskfile.yml` tool pins (`ess`, `aep`, `b10x-gates`, …). Where the
organization keeps a catalog repository that declares relations between repositories, the map is
an observation conductor compares with that catalog, not a copy of it (`AGENTS.md` rule 2: facts
live with their owner).

Spec first: a `Dependency` observation (from repository, to repository, kind, pinned version).

Depends on `story:snapshot-driver`.

## Acceptance

- A snapshot records one dependency per manifest edge in every local checkout; a test over two
  fixture checkouts finds a Cargo git pin and a Taskfile tool pin.
- The board lists pins below the target's newest release.
