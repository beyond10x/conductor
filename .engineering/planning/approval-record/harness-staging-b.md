---
format: aep.planning-md/3
id: approval-record:harness-staging-b
kind: approval-record
status: draft
title: 'The operator chose B: conductor stages its programs through substrate, no harness work'
relations:
- decides: upstream-blocker:harness-gate-staging
revision: 1
---
## The question

Where conductor's gate gets the host programs (`ess`, `aep`, `claude`) and the machine's cargo
config inside a substrate exec (`upstream-blocker:harness-gate-staging`; conductor's escalation of
2026-10-09). Harness is inactive, superseded by loom, so new work there is class O.

Options put to the operator: (A) a scoped harness wave and a patch release; (B) conductor stages its
programs itself through substrate, with no harness change; (C) staging built into loom first.

## The decision

The operator, 2026-10-09, to conductor-dev: "B".

## What follows

`story:substrate-client` gains the staging acceptance; `upstream-blocker:harness-gate-staging` is
cleared with this record cited; nothing is dispatched to harness.
