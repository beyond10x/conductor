---
format: aep.planning-md/3
id: story:status-token-pool
kind: story
status: draft
title: conductor status shows each pool token's state, never its value
summary: The host's status lists each token of its pool as free, in use by which instance, or limited until when
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#206
relations:
- decomposes: epic:architecture-next
- depends_on: story:host-token-pool
scope:
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: inferred
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/status/federated.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: inferred
  path: crates/conductor/tests/federated_views.rs
- confidence: inferred
  path: spec/domains/token.yaml
revision: 3
---
## Why

Issue #206, item 4. Nothing an operator runs on the host says which token is free, in use or limited.
`conductor status` prints no token: its report (`crates/conductor/src/status.rs:368`) reads
snapshots, controllers and decisions, and its federated form, the view asked of conductord with no
instance selected (`crates/conductor/src/status/federated.rs:1-8`, `status.rs:267-288`), one row per
instance. The pool's rows exist only behind `GET /v1/instances/{name}/tokens`
(`crates/conductor/src/token.rs:1062-1071`, rows at `:510-557`), per instance, and the dashboard.

## Outcome

- `conductor status` with no instance selected, the host's view from conductord, prints one
  `tokens` section: per token of the host pool its id, owner, seat, kind and state, which is one of
  `free`, `in use by <instance> (<n> sessions)` (one entry per instance holding a lease), `limited
  until <time> (5-hour|weekly)`, and `withdrawn` for the last day. `--format json` carries the same
  rows. Never a value, nor the name of the secret that holds it.
- The rows come from one host route of conductord, modelled in `spec/` first (a view of the host
  pool) and bound in `crates/conductor/http.yaml`, as AGENTS.md rule 1 asks.

Decision (class C, controller, 2026-11-18): the section is in the host's (federated) status only; an
instance's own `status --state-dir` gains none, since the pool is the host's. Undone by adding it
there.

## Acceptance (tests that fail on the current code)

- `tests/federated_views.rs` `status_lists_each_pool_token_s_state_never_its_value`: three tokens,
  one free, one leased by two sessions of an instance, one limited until a set time; the text and the
  JSON name each state, and neither holds a token's value or secret name.

## Boundaries

- The pool and its leases are `story:host-token-pool`; the limits and their resets are
  `story:limit-aware-token-reassignment`.
- The dashboard's token page is not changed here.
