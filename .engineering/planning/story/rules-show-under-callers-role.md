---
format: aep.planning-md/3
id: story:rules-show-under-callers-role
kind: story
status: draft
title: rules show resolves an item number under the caller's role heading
scope:
- confidence: cited
  path: crates/conductor/src/rules.rs
revision: 2
---
## Why

Reported by a trial instance on 2026-11-10: `conductor rules show 13` printed the first item 13 of `rules.md`, under the heading of another role, and not item 13 under the caller's role heading. Item numbers repeat per heading, so a session that asked for its new rule read another role's rule.

## Acceptance

- `conductor rules show <n>` resolves the number under the caller's role heading; a case with the same number under two headings shows the caller's.
- Where the caller's role cannot be told or the number is under several headings and none is the caller's, the command refuses and names the headings that hold it.
