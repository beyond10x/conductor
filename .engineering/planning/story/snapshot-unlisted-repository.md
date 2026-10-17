---
format: aep.planning-md/3
id: story:snapshot-unlisted-repository
kind: story
status: draft
title: A repository missing from the workspace file is a recorded row, not a failed snapshot
revision: 2
---
# Story: snapshot-unlisted-repository

## Outcome

A new repository of the instance that `.engineering/workspace.yaml` does not list yet costs conductor one row marked "not in workspace", not an hour-long snapshot that fails at the end.

## Context

A conductor NEED (2026-10-15). The snapshot started ~02:2xZ failed at ~03:3xZ, after four collectors had run, with `specifications: repository thalamus is not archived, and no member of <records>/.engineering/workspace.yaml has it as its source`. thalamus had been created at 01:48Z. It was fixed by hand with `conductor config workspace` (records 2ec5520).

story:snapshot-member-failures and story:snapshot-absent-member cover a member that cannot be read, but not a repository that is no member at all.

## Acceptance

- Before the first collector, the snapshot compares the source listing with the workspace members. Each repository that is neither archived nor a member is named on one standard-error line, `repository <name>: not in workspace, run conductor config workspace`.
- The specifications collector records such a repository as a row in state `not-in-workspace` and continues, so the snapshot completes. Test: a source listing with one unlisted repository; the snapshot exits 0 with that row.
- `conductor watch` prints `workspace stale: <repo>` once per repository when the newest snapshot holds a `not-in-workspace` row. A test drives this with a fixed clock.

## Out of Scope

Running `conductor config workspace` automatically. The workspace file is conductor's to write.

## Ambiguities

None.
