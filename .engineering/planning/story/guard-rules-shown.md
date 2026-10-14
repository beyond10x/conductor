---
format: aep.planning-md/3
id: story:guard-rules-shown
kind: story
status: draft
title: Each role's guard rules are in its start reading, and every denial names the allowed form
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

236 guard denials in a week cost ~135k tokens of recovery (`docs/design/self-improvement.md`, F5, I4); sessions learn the rules by being denied.

## Acceptance

- `conductor guard rules --role <role>` prints the role's rules as a short table (what is denied, the allowed form); conductor, controller and conductor-dev start reading includes it; every denial reason ends with the allowed form; a test per role and a test that every denial reason has one.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/cli.rs`, `.agents/*.md`, tests.
