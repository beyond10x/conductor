---
format: aep.planning-md/3
id: story:status-board
kind: story
status: implemented
title: conductor status and STATUS.md
relations:
- decomposes: epic:conductor-autonomy
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/tests/fixtures/status
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T09:16:55Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-13T09:16:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-13T09:16:56Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
# `conductor snapshot publish-board` and STATUS.md

## Why

Conductor maintains a hand-built status table in the records' `STATUS.md`, and "what is everyone
doing" has no quick answer. The operator: "I do need to know the status across all my repos
(issues, PRs, last_version, things merged into remote main which are not covered by last
tag/release)". The `Board`, `PublishBoard` and `Boards` view are already specified
(`spec/domains/observation.yaml`); `crates/conductor/src/status.rs` is a stub.

## What

`snapshot publish-board` renders a board file (the instance's `STATUS.md` by default, `--path`
otherwise) from a complete snapshot and records `PublishBoard`, in `crates/conductor/src/status.rs`.
`task status` runs the snapshot collectors (`crates/conductor/src/collect/`) and then
`publish-board`. The file has, in this order:

1. **Header:** the command, the snapshot id and its start time.
2. **Deviations**, one row per repository with at least one of: a red `main` workflow; a PR with a
   failing check; a PR older than 14 days; unreleased commits; `no checkout` (`local_checkout`
   false); a specification missing or refused.
3. **Overview**, one row per non-archived repository, in name order: open issues, open PRs (of them
   failing), last release tag and its date, commits on `main` after that tag, red `main` workflows,
   the running controller or none.
4. **Specifications:** one row per repository with presence and validation.
5. **Open blockers**, by repository and kind.
6. **Sessions.**
7. **Delta** against the previous complete snapshot: the repositories whose deviation set changed,
   each with the deviations added and removed, and the change in five counts (open PRs, PRs with
   failing checks, non-green workflows, open blockers, specifications present); `no change` when
   none changed.

## Acceptance

- `publish-board` over a fixture complete snapshot writes a file whose seven section headings
  appear in the order above, and whose header names the snapshot id; a test in
  `crates/conductor/tests/status.rs` holds it.
- `publish-board` against a failed snapshot is refused with `SnapshotNotComplete` and writes no
  file.
- With fixtures for two snapshots that differ only in one PR's check turning red, the delta lists
  exactly that repository with `+failing PR` and the failing-PR count `+1`; with two identical
  fixtures it reads `no change`.
- A fixture repository with `local_checkout` false renders `no checkout` in its deviation row.
- `Taskfile.yml` has a `status` task that runs the collectors and then `snapshot publish-board`; a
  test in `crates/conductor/tests/taskfile.rs` holds it.

## Notes

- Shares `Taskfile.yml` and `crates/conductor/tests/taskfile.rs` with `story:self-arming-restart`,
  which depends on this story and lands after it on those files.
- Out of this story, as follow-ups: repository marks (Active/archived) filtering PR deviations, and
  free disk on the board.

## Scope

`crates/conductor/src/status.rs`, `crates/conductor/tests/status.rs`,
`crates/conductor/tests/fixtures/status`, `Taskfile.yml`, `crates/conductor/tests/taskfile.rs`.
