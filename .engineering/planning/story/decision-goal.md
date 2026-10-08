---
format: aep.planning-md/3
id: story:decision-goal
kind: story
status: draft
title: A decision names the goal and the value it rests on
revision: 1
---
# A decision names the goal and the value it rests on

## What

The operator, relayed by conductor: "when you are taking decisions, we do want to see whats the
reasoning behind your decisions ... based on what are our values/org-goals". Conductor already
writes a `goal` field by hand on its decisions.

Spec first (`spec/domains/decision.yaml`): a decision records `goal` (a `GoalId` from
`NORTHSTAR.md`, or none with a reason) and `rests_on` (the value, principle or objective the choice
follows, quoted from where it is written: `NORTHSTAR.md`, the design, or the operator's words).
Both are required on `record-conductor-decision`; a decision without `rests_on` is refused. The
`decisions/` export carries both.

Depends on `story:decision-commands`.

## Acceptance

- `ess specify validate --path spec` is valid with the two fields; a scenario refuses a conductor
  decision without `rests_on`.
- The `decision decisions --format jsonl` export carries `goal` and `rests_on` on every line.
