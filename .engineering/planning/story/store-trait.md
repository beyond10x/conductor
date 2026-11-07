---
format: aep.planning-md/3
id: story:store-trait
kind: story
status: draft
title: 'the store is a trait with the tree and sqlite backends chosen by store:'
summary: append, read since, indexed views; tree:// (today) and sqlite:// backends
relations:
- decomposes: epic:architecture-next
revision: 1
---
## Why

Operator, 2026-11-07: "the state store should somewhat be abstracted, because we might want to put that into a database." Design §6.1.

## Acceptance

- A `Store` trait with append, read-since-id and the indexed views the eventlog offers today; the existing tree store implements it with no behaviour change (the full test suite passes against it).
- `sqlite://<path>` implements the same trait in one file; a conformance test runs every store test against both backends.
- `store:` is a global and an instance key; default `tree://<state>/tree`; `conductor store migrate --to sqlite://…` copies an instance's events in order and verifies the count.
- The records directory projections (`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`) are unchanged.
