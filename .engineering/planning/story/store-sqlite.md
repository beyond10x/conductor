---
format: aep.planning-md/3
id: story:store-sqlite
kind: story
status: archived
title: the store is one SQLite file per instance, opened through Eventlog's port; the tree is migrated once
summary: Eventlog's SQLite adapter as the only backend; conductor store migrate from the tree; no store trait of conductor's own
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 3}
- {from: "active", to: "archived", at: "2026-11-08T00:13:20Z", actor: "human:timo", revision: 4}
---
## Why

Operator, 2026-11-07: "the state store should somewhat be abstracted"; 2026-11-08: "remote -> sqlite, local too - with sync". ADRs 0041, 0053 and 0063 in atlas make Eventlog the shared persistence port and forbid a second implementation; conductor's store is an eventlog tree today (`docs/design/conductor.md` §10). Design §8.

## Acceptance

- Every store open in `crates/conductor` goes through Eventlog's port with its SQLite adapter; the instance's file is `<runtime>/store.sqlite`, the daemon's `<daemon state>/store.sqlite`; a test asserts no other store implementation is linked.
- `conductor store migrate` reads today's tree once, appends every event in order into the SQLite file, verifies the count and the wire bytes (ADR 0063), and leaves the tree beside it unread; a second run is a no-op that says so.
- The full test suite passes against the SQLite backend; the records directory projections (`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`) are unchanged.
- The guard still answers an allowed call without opening the store (`AGENTS.md` rule 6); `tests/guard_latency.rs` holds its bound.
