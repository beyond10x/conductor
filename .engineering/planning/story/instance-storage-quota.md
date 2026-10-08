---
format: aep.planning-md/3
id: story:instance-storage-quota
kind: story
status: draft
title: An instance's trees and builds stay under a storage quota
relations:
- depends_on: story:host-grants
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/quota.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 7
---
## Why

Worktree builds are the host's disk hog; quotas would enforce what the host ledger only polices
(`specification:multi-instance-host`, findings 6, 7, 8). Blocked by
`decision-blocker:project-quota-filesystem`: this host's `/` has no `prjquota`.

## Acceptance

Once the operator chose option A or B:
- an instance's managed trees and build directories carry the instance's project quota id, set
  with `setquota -P`, with the limit from `instances[].storage_quota` (spec first); unit tests check
  the commands conductor builds with a fake `setquota`;
- a manual check the operator runs once on the chosen filesystem: write past the limit in an
  instance's tree and observe `EDQUOT` (a test can't mount or reach the real quota without root;
  acceptance critic round 2). The commands and output go into `docs/analysis/`.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/config.rs`, `crates/conductor/src/quota.rs` (new),
`crates/conductor/tests/quota.rs` (new), `crates/conductor-model/`. After `host-grants`.
