---
format: aep.planning-md/3
id: story:dispatch-hygiene
kind: story
status: draft
title: A blocked report has a reason, and a repository has one open dispatch
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

Conductor's history review (2026-10-06 to 14): 33 blocked spans totalling 179 h carry no reason,
and one repository had three dispatches open at once.

## Acceptance

- Spec first: `dispatch report-blocked` refuses an empty or whitespace reason; `dispatch
  send-dispatch` to a repository with an open dispatch refuses unless `--supersedes <id>` names it,
  which closes that one as superseded. Tests hold both refusals and the supersede.

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model`, `crates/conductor/src/dispatch.rs`,
`crates/conductor/tests/dispatch.rs`.
