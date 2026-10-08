---
format: aep.planning-md/3
id: story:quota-support-report
kind: story
status: draft
title: conductor doctor says whether the trees' filesystem supports project quotas
relations:
- depends_on: story:install-prerequisites
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
revision: 2
---
## Why

Split from `instance-storage-quota` (design critic, round 1): whether a host can enforce storage
quotas is worth reporting whatever the operator decides on `decision-blocker:project-quota-filesystem`.

## Acceptance

- `conductor doctor` (story `install-prerequisites`) prints the filesystem type and mount options of
  the directory holding `checkouts.trees`, and `project quotas: yes|no`; a test with a fixture
  mount table.

## Scope

`crates/conductor/src/doctor.rs` (from `install-prerequisites`), `crates/conductor/tests/doctor.rs`.
