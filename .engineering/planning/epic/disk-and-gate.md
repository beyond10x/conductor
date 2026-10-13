---
format: aep.planning-md/3
id: epic:disk-and-gate
kind: epic
status: implemented
title: Disk is reclaimed by a command and CI is the only gate
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 4}
---
## Why

On 2026-10-12 the host's free disk gated every build: conductor refused or queued conductor-dev's
build slot four times (rules for disk admission in the conductor profile), and waves 06–08 were
built only in CI. The operator chose cleanup rules as the disk control
(`approval-record:project-quota-c`); the dashboard measures what is reclaimable
(`crates/conductor/src/dashboard/waste.rs:4-11`), but nothing reclaims it, and a cleanup-only
session does it by hand. CI became the merge gate in wave 06, but it rebuilds from scratch on every
push (`.github/workflows/check.yml` has no cache) and `AGENTS.md` still asks for a local gate.

## Outcome

Disk is reclaimed by a command, CI is fast enough to be the only gate, and the repository's guide
says so.

## Stories

`story:portable-tree-sizes`, `story:conductor-gc`, `story:ci-build-cache`, `story:agents-ci-gate`.
