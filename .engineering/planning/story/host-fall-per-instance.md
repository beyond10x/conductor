---
format: aep.planning-md/3
id: story:host-fall-per-instance
kind: story
status: implemented
title: A disk fall is recorded and cleared per instance
relations:
- decomposes: epic:multi-instance-host
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T02:05:02Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T02:05:02Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T02:05:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: host-fall-per-instance

## Outcome

A disk fall is recorded per instance, and only the instance that recorded it clears or withdraws it.

## Context

docs/design/roles-and-surfaces.md § 5a, I6. `host.fall` names one instance (host.rs:88), so one instance's watch can clear another instance's fall. The ledger itself stays shared by design.

## Acceptance

1. A fall is kept per instance, as `<host>/falls/<prefix>.json`.
2. A watch clears only its own instance's fall; the dashboard shows every instance's fall.
3. A test with two instances shows that A's watch leaves B's fall in place.

## Out of Scope

Moving the ledger.

## Ambiguities

None.
