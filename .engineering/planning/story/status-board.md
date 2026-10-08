---
format: aep.planning-md/3
id: story:status-board
kind: story
status: draft
title: conductor status and STATUS.md
revision: 1
---
# `conductor snapshot publish-board` and STATUS.md

## What

`snapshot publish-board` renders `STATUS.md` from the newest complete snapshot and records
`PublishBoard`, in `crates/conductor/src/status.rs` only. `task status` runs
`snapshot start-snapshot` and then this. The board has:

1. **Deviation rows**, one per repository with at least one deviation
   (PR and issue deviations count only for repositories marked Active, `story:repository-marks`):
   - red `main` workflow;
   - PR with a failing check;
   - PR older than 14 days;
   - unreleased commits;
   - dirty primary checkout;
   - behind `main`;
   - `no checkout` (`local_checkout` false);
   - not in the catalog;
   - planning store not on `aep.project/5`;
   - specification missing with no opt-out;
   - specification refused;
   - specification `format` below the newest;
   - synthesis refusals above 0;
   - a session bound to it other than its controller (`story:spec-review-closure`);
   - a session bound to it.
2. **Specifications:** one row per repository, with presence, format, validation, scenarios,
   refusals and conformance status.
3. **Open blockers**, by repository and kind.
4. **Sessions.**
5. **Free disk.**
6. **Delta** against the previous complete snapshot.

The delta is defined as:
- repositories whose deviation set changed, each with the deviations added and removed;
- the change in five counts: open PRs, PRs with failing checks, non-green workflows, open blockers,
  specifications present.

Free disk is shown but is not part of the delta.

The header names the command, the snapshot id and its start time.

## Overview table

The operator: "I do need to know the status across all my repos (issues, PRs, last_version, things
merged into remote main which are not covered by last tag/release)". Besides the deviation rows,
`STATUS.md` carries one row per non-archived repository, every repository, deviating or not:

| column | from |
|---|---|
| open issues | `open_issues` (`story:collect-repositories`) |
| open PRs, of them failing | `story:collect-prs-and-ci` |
| last release tag and its date | `latest_release`, `latest_release_at` (`story:collect-repositories`, which gains the date) |
| commits on `main` after that tag | `unreleased_commits` |
| red `main` workflows | `story:collect-prs-and-ci` |
| controller | the running controller from the `Controllers` view, or none |

Conductor has maintained a hand-built version of this table in `STATUS.md`.

Depends on `story:collect-repositories`, `story:collect-prs-and-ci`, `story:collect-sessions`,
`story:collect-blockers`, `story:collect-specifications` and `story:repository-marks`.

## Scope

- `STATUS.md`
- `crates/conductor/src/status.rs`
- `crates/conductor/tests/fixtures/status`

## Acceptance

- `task status` completes in under 60 s against an instance.
- `publish-board` against a failed snapshot is refused with `SnapshotNotComplete`.
- With recorded fixtures for two snapshots that differ only in free disk, the delta section reads
  `no change`.
- With fixtures that differ in one PR's check turning red, the delta lists exactly that repository,
  with `+failing PR`, and the failing-PR count `+1`.
- A fixture repository with `local_checkout` false renders `no checkout` in its row.
- The overview table has one row per non-archived repository of the newest complete snapshot, in
  name order.
