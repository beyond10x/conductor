---
format: aep.planning-md/3
id: story:views-open-as-reader
kind: story
status: draft
title: Every read-only command opens the store as a reader
revision: 2
---
# Story: views-open-as-reader

## Outcome

Every read-only command of the `conductor` CLI (each view: `session leases`, `decision requests`, `status`, `host grants` and the rest) opens the store as a reader, so it never waits on a writer's lock and never keeps a writer waiting.

## Context

Conductor's NEED of session c43e6a10, 2026-10-16 ~15:22Z: `conductor session leases` ran for more than 2 min at session start, and `timeout 90 conductor session leases --format jsonl` exited 124 with no output. The view opens through `state::open_existing` (crates/conductor/src/session.rs:201), which is the writer path (store.rs:159): it takes the tree's writer lock and replays a 221 MB store, so it waits behind any snapshot or watch write.

`grep` finds 47 call sites of `open_existing` in 19 modules, against 13 of `open_reader` (store.rs:175, which the dashboard and, since a6588d4f, `host grants` use). Story store-readers-hold-no-writer-lock promised this for the dashboard and the watch only.

## Acceptance

1. Each command that writes nothing opens the store with `open_reader`. The set of such commands is `crate::cli::writes_records`' complement plus the views of the spec. A test walks the CLI tree and fails for a view that opens with the writer path.
2. A test holds the tree's writer lock, runs `session leases`, `decision requests` and `status`, and each answers within 5 s.
3. Under the live instance, `conductor session leases` answers in under 5 s while a snapshot runs (recorded as evidence).

## Out of Scope

Making the replay itself cheaper (peak memory of the replay).

## Ambiguities

None.
