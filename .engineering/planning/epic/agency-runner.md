---
format: aep.planning-md/3
id: epic:agency-runner
kind: epic
status: draft
title: conductor dispatches through Agency's API
summary: Work items and runs through Agency, events read back, Send and Report, approvals and credential grants through Agency; then the default runner
relations:
- depends_on: epic:cross-repository-waves
revision: 1
---
# Epic: agency-runner

## Outcome

conductor dispatches through Agency's API: a dispatch becomes a work item on the repository's work
input and a run Agency starts; conductor reads `/agency/events` and keeps its own records of
decisions and dispatches. Messages to Agency-run agents go through `Send` and `Report`, a run's
approvals reach conductor's decision classes through Agency, and credentials are Agency grants.
Every Active repository is then dispatched through this runner; `claude-code-session` stays a
runner an instance may name.

## Acceptance

The two-repository scenario of `epic:cross-repository-waves` runs through Agency, each dispatch's
state equals its work item's, and `conductor records check` exits 0; one takt `ApprovalRequired`
is answered through Agency and recorded as a conductor decision; then 7 consecutive days with 0
controller sessions spawned, every dispatch ending `done` or `failed` with a recorded outcome.

## Depends on outside this store

Agency M0 to M3 and the gaps G1 to G7 of design § 3.2 (Agency, Loom). Agency's side of this work is
its own `epic:conductor-on-agency`.

## Source

`docs/design/conductor-on-agency.md` § 4 steps 6 to 8, § 6 D5, D8.
