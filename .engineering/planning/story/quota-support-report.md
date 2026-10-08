---
format: aep.planning-md/3
id: story:quota-support-report
kind: story
status: draft
title: conductor doctor says whether the trees' filesystem supports project quotas
relations:
- depends_on: story:install-prerequisites
- decomposes: epic:session-confinement
- depends_on: story:session-envelope-seam
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
revision: 3
---
## Why

Split from `instance-storage-quota` (design critic, round 1): whether a host can enforce storage
quotas is worth reporting whatever the operator decides on `decision-blocker:project-quota-filesystem`.
The epic's doctor line on disk.

## Acceptance

- `conductor doctor` prints, for the filesystem holding `checkouts.trees`: its mount point, type and
  mount options, and `project quotas: yes` when the options contain `prjquota` or `pquota`, else
  `project quotas: no`.
- Tests with fixture mount tables: ext4 with `prjquota` → yes; xfs with `pquota` → yes; ext4
  `rw,noatime` → no; the trees root on a nested mount picks that mount, not `/`.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`. After
`install-prerequisites` and `session-envelope-seam` (both change `doctor.rs`).
