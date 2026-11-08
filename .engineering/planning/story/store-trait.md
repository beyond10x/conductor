---
format: aep.planning-md/3
id: story:store-trait
kind: story
status: draft
title: 'the store is a trait with the tree and sqlite backends chosen by store:'
summary: append, read since, indexed views; tree:// (today) and sqlite:// backends
relations:
- decomposes: epic:architecture-next
revision: 2
---
## Why

Operator, 2026-11-07: "the state store should somewhat be abstracted, because we might want to put that into a database." ADR 0041, 0053 and 0063 in atlas make Eventlog the shared persistence port with file, SQLite and PostgreSQL adapters and forbid a second implementation; conductor's store is already an eventlog tree (docs/design/conductor.md §10). Design §6.1.

## Acceptance

- The store opens through Eventlog's persistence port; the tree adapter behaves as today (the full test suite passes against it); a conformance test runs every store test against the tree and the SQLite adapter.
- `store:` is a global and an instance key; default `tree://<state>/tree`; `conductor store migrate --to sqlite://…` copies an instance's events in order and verifies the count; wire bytes are unchanged between adapters (ADR 0063).
- No store trait of conductor's own exists after this story (a test asserts the crate depends on Eventlog's port for every store open).
- The records directory projections (`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`) are unchanged.
