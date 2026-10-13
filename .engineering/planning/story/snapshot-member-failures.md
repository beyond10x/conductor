---
format: aep.planning-md/3
id: story:snapshot-member-failures
kind: story
status: implemented
title: A member a collector cannot read is a recorded failure, not a failed snapshot
relations:
- supersedes: story:specifications-owner-nonmembers
- depends_on: story:conductor-upgrade
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/blockers.rs
- confidence: cited
  path: crates/conductor/src/collect/specifications.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/store/observations.rs
- confidence: cited
  path: crates/conductor/src/store/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
- confidence: cited
  path: crates/conductor/tests/member_failures.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:52Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:04Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:37Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}}
---
## Why

Split from `story:workspace-from-config` (scoped 2026-10-12). A snapshot keeps one snapshot-wide
`failure_reason` (`crates/conductor/src/snapshot.rs`) and `spec/domains/observation.yaml` has no
record of a member a collector could not read, so one member whose `git fetch` fails still fails
the whole snapshot, and nothing says which member it was.

## Acceptance

- Spec first, in `spec/domains/observation.yaml`, validated before any code: a record
  `MemberFailureObservation` (snapshot id, member, collector, the error's first line), a command
  `RecordMemberFailure`, and a view `MemberFailures` listing a snapshot's failures; the command and
  the view are placed in the `snapshot` group of `spec/components.yaml`, so
  `conductor snapshot member-failures --snapshot-id <id>` prints them (`cli_tree.rs` holds
  `cli.rs` to that group).
- The blockers and the specifications collectors record a member they cannot read as a member
  failure and go on with the other members; the snapshot completes with the other members' rows.
- Tests in `crates/conductor/tests/member_failures.rs`, three workspace members each, one of which
  cannot be read:
  - blockers collector: a fake `git` whose fetch fails for one member with `fatal: could not read
    from remote repository` on its first line; the snapshot is `Completed`, the other two members'
    blocker rows are there, and `conductor snapshot member-failures` prints one row with that
    member's name, `blockers` and that first line;
  - specifications collector: one member whose `git ls-tree` fails; the snapshot is `Completed`,
    the other two members' specification rows are there, and the view prints one row with that
    member's name, `specifications` and the error's first line;
  - a snapshot with no failure prints no rows.
- A member whose fetch fails is recorded as a member failure, and its blockers are still read from
  its checkout as last fetched, so its blocker rows are in the snapshot beside the failure (the
  same behaviour `story:fetch-without-prompts` (d) tests for a timeout and a refused fetch).
- In `crates/conductor/tests/collect_stores.rs`, this story rewrites
  `a_fetch_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command` and
  `an_ess_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command`, which today
  assert `SnapshotState::Failed` and no blockers: each now expects a `Completed` snapshot with one
  recorded member failure naming the repository, the collector and the command's first error line
  (and, for the fetch, that member's blocker rows as last fetched).
- `website/docs/reference/cli.md` is regenerated with the new command and view.

## Notes

- After `story:conductor-upgrade` (shares `cli.rs`, `crates/conductor-model`,
  `website/docs/reference/cli.md`).
- Before `story:fetch-without-prompts` (which records its timeout through this record, and edits
  `collect/blockers.rs` and `tests/collect_stores.rs` after this story's rewrite of the two cases
  above) and `story:snapshot-duration`, which edits the same files: the `snapshot`
  group of `spec/components.yaml`, `spec/domains/observation.yaml`, the model,
  `store/observations.rs` (`ENTITIES` grows by one in each), `store/snapshot.rs` (one storage port
  each), `snapshot.rs`, `cli.rs` and `reference/cli.md`.
- `story:dashboard-health` serves these rows with a fix text.

## Scope

`spec/domains/observation.yaml`, `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/src/snapshot.rs`, `crates/conductor/src/store/observations.rs`,
`crates/conductor/src/store/snapshot.rs`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/src/collect/specifications.rs`, `crates/conductor/src/cli.rs`,
`website/docs/reference/cli.md`, `crates/conductor/tests/member_failures.rs` (new),
`crates/conductor/tests/collect_stores.rs`.
