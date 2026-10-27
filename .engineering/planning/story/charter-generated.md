---
format: aep.planning-md/3
id: story:charter-generated
kind: story
status: draft
title: conductor charter render writes a repository's charter from the store, and the daemon renders a missing one before a spawn
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

Charters are hand-written by the conductor session (`crates/conductor/src/spawn.rs:135-141` reads
them; no code writes one). Dispatch v1 of `conductor run` spawns from the charters that exist.
Without generation a new repository or a changed goal still needs a person or a model to write
prose.

## Outcome

`conductor charter render --repository <repo>` writes `charters/<repo>.md` from the config, the
goals and servings, the repository's decisions of the last 30 days, its last hand-over path, its
`merge:` setting and the controller profile path; the daemon renders a missing charter before a
spawn and re-renders when a goal or decision for the repository changed.

## Acceptance

1. Spec first: a `Charter` view in `spec/domains/dispatch.yaml` with those fields; `ess specify
   validate` exits 0.
2. `conductor charter render --repository <repo>` over a fixture store writes the sections Goal,
   Scope, Boundaries, Standing decisions, Resume point, Merge, Profile; a test reads each back; a
   hand-written section `## Operator notes` survives a re-render.
3. `conductor run`'s dispatch v1 renders a missing charter before a spawn (test).

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model`, new `crates/conductor/src/charter.rs`,
`crates/conductor/src/run/dispatch.rs`, `crates/conductor/tests/charter.rs`.
