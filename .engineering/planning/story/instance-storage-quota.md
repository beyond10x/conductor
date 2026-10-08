---
format: aep.planning-md/3
id: story:instance-storage-quota
kind: story
status: draft
title: An instance's trees and builds stay under a storage quota
relations:
- depends_on: story:host-grants
- decomposes: epic:session-confinement
- depends_on: story:quota-support-report
- depends_on: story:substrate-tool-server
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/quota.rs
- confidence: cited
  path: crates/conductor/tests/quota.rs
- confidence: cited
  path: docs/analysis/storage-quota-run.md
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
revision: 11
---
## Why

Worktree builds are the host's disk hog; quotas would enforce what the host ledger only polices
(`specification:multi-instance-host`, findings 6, 7, 8). The epic's disk line. Blocked by
`decision-blocker:project-quota-filesystem`: this host's `/` has no `prjquota`.

## Acceptance

Once that blocker is cleared with option A or B:
- an instance's managed trees and build directories carry the instance's project quota id, set
  with `setquota -P`, with the limit from `instances[].storage_quota` (spec first); unit tests check
  the commands conductor builds with a fake `setquota`;
- a manual check the operator runs once on the chosen filesystem: write past the limit in an
  instance's tree and observe `EDQUOT` (a test can't reach the real quota without root). The
  commands and output go into `docs/analysis/storage-quota-run.md`.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/config.rs`, `crates/conductor/src/quota.rs` (new),
`crates/conductor/tests/quota.rs` (new), `crates/conductor-model/`, `docs/config.md`,
`docs/analysis/storage-quota-run.md` (new). After `host-grants`, `quota-support-report` and
`substrate-tool-server`: they and the stories before them change `config.yaml`, `config.rs` and the
generated crate. This story is blocked on a decision, so it goes last; if the decision lands first,
the edge to `substrate-tool-server` is turned around.
