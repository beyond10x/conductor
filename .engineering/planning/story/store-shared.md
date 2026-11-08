---
format: aep.planning-md/3
id: story:store-shared
kind: story
status: implemented
title: one shared database per deployment, opened by the daemon and every instance runtime through Eventlog's sqlite and postgres adapters
summary: daemon.store names one URL; the daemon hands it to every runtime it starts; every event names its instance; store migrate reads today's tree once; no copy per instance, no sync, no restore
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
scope:
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/Cargo.toml
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: inferred
  path: crates/conductor/src/store
- confidence: inferred
  path: crates/conductor/src/store.rs
- confidence: cited
  path: crates/conductor/src/store/migrate.rs
- confidence: inferred
  path: crates/conductor/tests/fixtures/store
- confidence: inferred
  path: crates/conductor/tests/guard_rules.rs
- confidence: inferred
  path: crates/conductor/tests/snapshot_store_cost.rs
- confidence: inferred
  path: crates/conductor/tests/store.rs
- confidence: inferred
  path: crates/conductor/tests/store_tree.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/domains/store.yaml
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T00:13:20Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T00:13:20Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T10:59:44Z", actor: "human:timo", revision: 19, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-08: "with this multi-user, multi-operator thing, we of course need some shared state database. ... this complexity with instance level state synced to the operator's machine, it doesn't make any sense. ... it's just one database and it will be shared." Design §8, §13 row 13. Supersedes `story:store-sqlite` and `story:store-sync` (archived).

## Acceptance

- `daemon.store` is one URL, `sqlite:<path>` (one host: the file under the daemon's state directory) or `postgres://…` with its password as `secret:<name>`; the daemon opens it through `eventlog-sqlite` or `eventlog-postgres` and refuses to start without it; `conductor doctor` names the backend and whether it answers.
- `conductor instance serve` receives the URL from the daemon and opens the same store; every event carries its instance name; two runtimes and the daemon writing at once see one ordered log per instance (a test runs two instance runtimes against one SQLite file and one against a Postgres container when `CONDUCTOR_TEST_POSTGRES_URL` is set, and skips the latter by name otherwise).
- Every view the dashboard, the TUI and the CLI read comes from that one database through the daemon's API; no per-instance copy, no `cadence.sync`, no `instance restore`, no `store/events` route exist.
- `conductor store migrate` reads today's eventlog tree once into the database and is the only reader of the tree afterwards; the per-instance `store:` config key is removed, and `config migrate` drops it.
