---
format: aep.planning-md/3
id: story:store-postgres-proven
kind: story
status: draft
title: The postgres store is held by CI and one live run; sqlite stays the default
scope:
- confidence: cited
  path: crates/conductor/src/store.rs
- confidence: cited
  path: crates/conductor/tests/store_shared.rs
- confidence: cited
  path: website/docs/concepts/records-and-store.md
revision: 4
---
## Outcome

A deployment that names `daemon.store: postgres://…` is held by CI and by one recorded live run,
and a docs page says how to set it up. sqlite stays the default for every placement, a box
included: a box deployment has a persistent disk, and its store is a sqlite file on it.

## Why

`crates/conductor/src/store.rs` opens `postgres://` through the eventlog postgres adapter and
`story:store-shared` is implemented, but the Postgres case of `crates/conductor/tests/store_shared.rs`
runs only when `CONDUCTOR_TEST_POSTGRES_URL` is set and no workflow sets it. Nothing records
conductor running on Postgres outside that test. The operator asked for Postgres to work without
becoming the default (2026-11-10).

## Acceptance

- CI starts a Postgres service container, sets `CONDUCTOR_TEST_POSTGRES_URL`, and the Postgres
  cases of the store suite run and pass on every pull request; a run in which they were skipped
  fails the job.
- One live run is recorded as evidence: conductord and one instance runtime in a box open the
  same Postgres database, a loop pass writes events, `conductor status` reads them, and the
  daemon's journal shows no store error.
- `conductor store migrate` from a sqlite store to a Postgres store and back leaves the same
  events, held by a test.
- The docs name Postgres as an option beside sqlite, with the password as `secret:<name>`, and
  say sqlite is the default for a host and for a box.

## Not in this story

Making Postgres the default for any placement. The cluster placement question
(`decision-blocker:cluster-placement-and-store`).
