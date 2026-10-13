---
format: aep.planning-md/3
id: story:report-md-in-records
kind: story
status: draft
title: Conductor may write REPORT.md at the records root
relations:
- decomposes: epic:loop-without-handwork
revision: 3
---
## Why

An evaluation run on v0.8.0 (2026-10-13, finding V8-6) found that the guard's list of records paths
conductor may write leaves out `REPORT.md` at the records root, so conductor wrote its run report to
`docs/analysis/REPORT.md` instead of where the run asked.

## What

Conductor may write `<records>/REPORT.md` (Write, Edit and Bash), as it may `STATUS.md` and
`NORTHSTAR.md`; nothing else at the root is added.

## Acceptance

- `crates/conductor/tests/guard_rules.rs`: conductor's Write, Edit and Bash redirect (`… >
  <records>/REPORT.md`) of `<records>/REPORT.md` are allowed; of `<records>/OTHER.md` denied as
  today; a controller's Write, Edit and Bash redirect of `<records>/REPORT.md` denied.
- The guard's page lists `REPORT.md` with the other records files.

## Scope

`crates/conductor/src/guard/rules.rs` (conductor's records paths), `crates/conductor/tests/guard_rules.rs`,
`website/docs/security/the-guard.md`.
