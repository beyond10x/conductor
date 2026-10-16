---
format: aep.planning-md/3
id: epic:role-names
kind: epic
status: draft
title: Every level has one job and a clear name
revision: 2
---
# Epic: role-names

## Outcome

Every level has one job and a clear name:
- **maintainer:** today's conductor-dev, one per host, outside every instance;
- **director:** today's conductor session, one per instance;
- **lead:** today's repo-controller, which orchestrates and codes nothing;
- **workers:** planner, implementer, reviewer and releaser, available by default, each with its default tools and model tier.

Messages go one level up or down. Design: docs/design/roles-and-surfaces.md §§ 2–3, approved by the operator on 2026-10-16.

## Acceptance

1. Profiles, config roles, session names, Taskfile tasks and docs use the new names. The old names stay as aliases for one release, and `config validate` notes each old name it reads.
2. The maintainer is configured per host, not per instance (`instances[].roles` no longer holds it). It reads every instance's records and transcripts, and what it writes into the product never names an instance (the scan enforces it).
3. The four worker roles ship as agents with their profiles, default tools and model tiers. A lead starts one per unit.
4. Profiles shrink: each role's profile keeps only its own level's job. The conductor profile drops below 250 lines from 393, measured.
5. A message to a level other than one up or one down is refused by the guard and recorded.

## Out of Scope

The broker and `tool` (epic:agent-tools).

## Ambiguities

None. The operator approved the names as proposed.
