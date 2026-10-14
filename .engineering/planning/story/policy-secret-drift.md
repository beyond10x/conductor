---
format: aep.planning-md/3
id: story:policy-secret-drift
kind: story
status: draft
title: A stale CI policy secret is seen and sent to its owner, not the operator
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

CI reads the Gates policy from an organization secret, and every class-C baseline move or enrolment
changes the policy file, so each needs the operator to refresh the secret. Nine policy commits
landed after the secret was last set (2026-10-12 to 2026-10-14); CI ran on a stale policy, and a
pull request failed on it. The operator, 2026-10-14: a decision conductor takes should reach CI
with no operator step.

## Acceptance

- The snapshot records, per instance, the time the policy file last changed on its repository's
  default branch and the `updated_at` of the organization secret that carries it (read with the
  bot's API route); the board flags `policy secret stale` when the file is newer; a test over
  fixtures holds the flag.
- `.agents/conductor.md` says a stale policy secret is dispatched to the policy repository's
  controller as a request to make CI read the policy without a person (for example from the
  repository through the bot App), and only reaches the operator if that needs a credential only
  they hold; a profile test holds the sentence.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/collect/`,
`crates/conductor/src/status.rs`, `crates/conductor/tests/`, `.agents/conductor.md`.
