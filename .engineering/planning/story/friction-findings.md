---
format: aep.planning-md/3
id: story:friction-findings
kind: story
status: draft
title: Friction signatures become findings with evidence
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

A KPI past target, or the same guard denial, correction kind or tool error recurring, is friction nobody files today. (`docs/design/self-improvement.md`.)

## Acceptance

- The daily run raises a finding for a KPI past its target two days running and for a signature seen in three sessions, with its evidence ids; a finding already open is not raised twice (tests over fixtures).

## Scope

`crates/conductor/src/kpi/findings.rs` (new), tests.
