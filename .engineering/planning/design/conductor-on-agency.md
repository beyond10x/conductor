---
format: aep.planning-md/3
id: design:conductor-on-agency
kind: design
status: draft
title: conductor on Agency's API
summary: conductor keeps cross-repository coordination; the sessions it drives become Agency-run Loom agents whose delivery is takt; eight Agency gaps, an eight-step migration, D1-D8 taken
relations:
- designs: epic:dispatch-runner
- designs: epic:loom-agent-runner
- designs: epic:cross-repository-waves
- designs: epic:agency-runner
revision: 1
---
## Document

[`docs/design/conductor-on-agency.md`](../../../docs/design/conductor-on-agency.md) is the design;
this record is its place in the plan. Status: proposed, 2026-10-26.

## Summary

conductor stays the coordinator across repositories (goals, the queue, waves across repositories,
approvals and decision classes, reporting, the host's disk while agents run on the same host). The
repository controller sessions it drives today become agents Agency runs through Loom as
`LoomAgent` `Command` resources whose program is takt. The design maps conductor's concepts onto
Agency's surfaces, names eight Agency gaps (G1 to G8) with a proposed owner each, and gives an
eight-step migration whose first step is `epic:dispatch-runner`.

## Decisions taken

D1 to D8 in § 6 of the document, each reversible; none needs the operator.
