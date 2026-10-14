---
format: aep.planning-md/3
id: story:retro-loop
kind: story
status: draft
title: Submitted retro reports feed the KPIs and friction findings
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:retro-spec
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): submitted reports feed the KPIs and friction findings.

## Acceptance

- the loop's daily run reads issues labelled `retro` from the configured repository (a fake forge), parses their YAML blocks into a `user sessions` series per KPI;
- a friction signature reported by three distinct authors becomes a finding citing the issue numbers; two authors do not;
- a report whose YAML block does not parse is skipped and named once.

## Scope

`crates/conductor/src/kpi/`, tests.
