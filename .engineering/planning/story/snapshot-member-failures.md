---
format: aep.planning-md/3
id: story:snapshot-member-failures
kind: story
status: draft
title: A member a collector cannot read is a recorded failure, not a failed snapshot
relations:
- supersedes: story:specifications-owner-nonmembers
revision: 1
---
## Why

Split from `story:workspace-from-config` (scoped 2026-10-12). A snapshot keeps one snapshot-wide
`failure_reason` (`crates/conductor/src/snapshot.rs:95`, `:197-199`) and `spec/domains/observation.yaml`
has no failure observation, so one member whose `git fetch` fails still fails the whole snapshot.

## Acceptance

- A new observation (spec first, `spec/domains/observation.yaml`) records a member the blockers or
  specifications collector could not read, with the member's name, the collector and the error's
  first line; the snapshot completes with the other members. Test: three members, one whose fetch
  fails; the snapshot completes with two members' rows and one recorded failure.
- A view lists those failures for a snapshot. Test.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model/`, `crates/conductor/src/snapshot.rs`,
`crates/conductor/src/store/`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/src/collect/specifications.rs`, `crates/conductor/tests/`.
