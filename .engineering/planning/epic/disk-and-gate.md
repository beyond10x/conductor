---
format: aep.planning-md/3
id: epic:disk-and-gate
kind: epic
status: draft
title: Disk is reclaimed by a command and CI is the only gate
revision: 1
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
