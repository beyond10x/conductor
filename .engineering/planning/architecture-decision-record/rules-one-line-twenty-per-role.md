---
format: aep.planning-md/3
id: architecture-decision-record:rules-one-line-twenty-per-role
kind: architecture-decision-record
status: accepted
title: Instance rules are one line each and at most 20 per role, and behaviour goes into code
relations:
- decides: story:rules-into-code
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-17T19:20:16Z", actor: "human:timo", revision: 2}
---
## Decision

The operator decided on 2026-10-17. Rules live in per-role files (all, conductor, controller, worker, maintainer). Each rule is one line, and a role has at most 20. A rule whose behaviour the binary can enforce moves into code and is then deleted. Nothing is dropped only for the cap: each over-cap item gets a home in code, config, a template or a profile. History stays in the decision log, never in the rule. conductor-dev adds no new rules; a change happens in code.

## Context

The old rules.md had grown to 98 KB, read at every session start. Workers never saw it, and its rules were routinely missed.
