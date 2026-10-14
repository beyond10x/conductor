---
format: aep.planning-md/3
id: epic:measured-self-improvement
kind: epic
status: draft
title: Conductor measures its KPIs and improves itself from what it measures
revision: 1
---
## Why

The retrospective of conductor's first week (`docs/design/self-improvement.md`) measured where the loop loses time or needs a
person: 0.245 operator touches per merged PR, 137.6 session-hours of idle controllers, 134.6 hours
waiting on background jobs, 148 usage-limit errors, 236 guard denials, a 22-hour p90 from merge to
release. Nothing measures these continuously, so nothing tells conductor-dev what to improve next or
whether a change helped.

## Outcome

Conductor measures twelve KPIs every day from its store, records, forge snapshot and transcripts,
shows them against their targets on the board and the dashboard, turns friction into findings with
evidence, and conductor-dev turns findings into stories and checks a week later that the KPI moved.
What the loop changes alone and what stays with the operator is the table in `docs/design/self-improvement.md` § 5.

## Approval

The operator approves `docs/design/self-improvement.md` before any story of this epic is delivered.
