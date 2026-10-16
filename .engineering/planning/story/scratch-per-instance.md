---
format: aep.planning-md/3
id: story:scratch-per-instance
kind: story
status: draft
title: Scratch is keyed by instance and repository
relations:
- decomposes: epic:multi-instance-host
revision: 2
---
# Story: scratch-per-instance

## Outcome

A controller's scratch and caches are keyed by its instance and repository, so two instances that track a repository of the same name never share them.

## Context

docs/design/roles-and-surfaces.md § 5a, I12. Scratch is `~/.cache/<repo>-*`, keyed by repository only (confine/places.rs:77; `repo-controller.md:146`). Reclaim also deletes there (gc/reclaim.rs:104).

## Acceptance

1. The scratch place is `<cache>/scratch/<repo>`, under the instance's own cache; the box and the profile name it.
2. Reclaim of scratch removes only the instance's own scratch.
3. A test with two instances and one repository name shows two distinct scratch directories.

## Out of Scope

Shared tool caches (sccache, go-build), which stay host-wide.

## Ambiguities

None.
