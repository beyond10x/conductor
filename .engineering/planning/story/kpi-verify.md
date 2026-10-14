---
format: aep.planning-md/3
id: story:kpi-verify
kind: story
status: draft
title: A week after a story ships, the KPI it named is read again and recorded
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:kpi-spec
revision: 1
---
## Why

Without a check, the loop cannot tell a change that helped from one that did not. (`docs/design/self-improvement.md`.)

## Acceptance

- `conductor kpi verify` records, for each story implemented 7 days ago that names a KPI, the KPI's value then and now in a `VerifyFinding`; one that did not move reopens its finding (test over a fixture).

## Scope

`crates/conductor/src/kpi.rs`, tests, `.agents/conductor-dev.md`.
