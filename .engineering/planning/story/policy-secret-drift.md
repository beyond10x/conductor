---
format: aep.planning-md/3
id: story:policy-secret-drift
kind: story
status: implemented
title: A stale CI policy secret is seen and sent to its owner, not the operator
relations:
- decomposes: epic:loop-without-handwork
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T17:07:18Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T17:07:18Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-14T18:23:31Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

CI in one organization reads a policy file from forge secrets, and every class-C change to that
file needed the operator to refresh the secrets; nine commits reached no CI between 2026-10-12 and
2026-10-14. The operator, 2026-10-14: a decision conductor takes should reach CI with no operator
step, and conductor stays generic ("conductor is not only used within the Beyond 10x repository").

## Acceptance

- Spec first: an instance config key lists secret syncs, each a source (repository, branch, path),
  the forge secrets it feeds, and the command that publishes it (an organization's own tool, never
  built into conductor). The snapshot records, per sync, the last commit of the source path and the
  secrets' `updated_at`; the board flags `secret stale: <sync>` when the source is newer; a test
  over fixtures holds the flag.
- `.agents/conductor.md` says a stale secret is published by running the sync's command when
  `rules.md` grants it, else handed over as class H with the command's dry run; a profile test holds
  the sentence.

## Scope

`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `crates/conductor-model`,
`crates/conductor/src/config.rs`, `crates/conductor/src/collect/`, `crates/conductor/src/status.rs`,
`crates/conductor/tests/`, `.agents/conductor.md`, `docs/config.md`.
