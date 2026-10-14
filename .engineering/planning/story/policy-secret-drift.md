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
- `.agents/conductor.md` says a stale policy secret is reported once per change with the policy
  commits it lacks and is sent to the controller of the repository whose CI reads the secret (the
  Gates scanner's), never to the operator as a routine step; how CI reads the policy without a
  secret is a class-O change to a contract between repositories, decided by the operator, and is
  not part of this story. A profile test holds the sentence.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/collect/`,
`crates/conductor/src/status.rs`, `crates/conductor/tests/`, `.agents/conductor.md`.
