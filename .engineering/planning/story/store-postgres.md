---
format: aep.planning-md/3
id: story:store-postgres
kind: story
status: archived
title: the postgres store backend for a shared, long-lived database
summary: 'store: postgres://… with the password as a secret; conformance against the same tests'
relations:
- decomposes: epic:architecture-next
- depends_on: story:secrets-source
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-11-07T23:41:19Z", actor: "human:timo", revision: 2}
---
## Why

Design §6.1 and §6.2 arrangement 2: an organization that already runs a database keeps every instance's records there; no mirror needed.

## Acceptance

- `postgres://` implements the `Store` trait; the store suite passes against it; the connection URL names its password as `secret:<name>`.
- An instance and the daemon opening the same database see the same events; the daemon skips the mirror for such an instance.
- `conductor store migrate` works between any two backends.
