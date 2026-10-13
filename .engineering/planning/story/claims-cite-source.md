---
format: aep.planning-md/3
id: story:claims-cite-source
kind: story
status: draft
title: A news or status claim cites the source it was checked against
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 2
---
## Why

A news item conductor sent named the wrong issue as fixed and had to be corrected; conductor's own
fault log names reading a suite's completion from its steps file without the runner's exit line
(2026-10-12). `story:watch-unsent-report` is adjacent but does not cover it.

## Acceptance

- Conductor's profile requires every news or status claim about a release, an issue or a gate to
  cite the source it was read from (the release page, the issue, the runner's exit line), and the
  news cron prompt tells it to check each claim against that source before sending; a test over the
  profile holds the rule.

## Scope

`.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
