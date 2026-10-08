---
format: aep.planning-md/3
id: story:instance-storage-quota
kind: story
status: draft
title: An instance's trees and builds stay under a storage quota
relations:
- decomposes: epic:multi-instance-host
scope:
- confidence: cited
  path: crates/conductor/src/quota.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 4
---
## Why

Worktree builds are the host's disk hog; quotas would enforce what the host ledger only polices
(`specification:multi-instance-host`, findings 6, 7, 8). Blocked by
`decision-blocker:project-quota-filesystem`: this host's `/` has no `prjquota`.

## Acceptance

Once the operator chose option A or B:
- an instance's managed trees and build directories carry the instance's project quota id, set
  with `setquota -P` (substrate's quota launcher is not used for files outside its workspaces),
  with the limit from `instances[].storage_quota` (spec first);
- a test on a loop-mounted XFS image with `pquota` writes past the limit and observes `EDQUOT`.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/quota.rs` (new),
`crates/conductor/tests/quota.rs` (new).
