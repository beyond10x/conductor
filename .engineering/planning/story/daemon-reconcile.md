---
format: aep.planning-md/3
id: story:daemon-reconcile
kind: story
status: implemented
title: conductor daemon serve reconciles instances from the config and lists them; the registry file goes
summary: the daemon starts and stops instance runtimes so the running set equals the config; GET /v1/instances; one Reconciliation per action
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
scope:
- confidence: inferred
  path: .agents/conductor-supervisor.md
- confidence: inferred
  path: .agents/conductor-teacher.md
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: inferred
  path: crates/conductor/src/cli.rs
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/confine/systemd.rs
- confidence: inferred
  path: crates/conductor/src/daemon
- confidence: inferred
  path: crates/conductor/src/dashboard.rs
- confidence: inferred
  path: crates/conductor/src/dashboard/configure.rs
- confidence: inferred
  path: crates/conductor/src/registry.rs
- confidence: inferred
  path: crates/conductor/src/run/unit.rs
- confidence: inferred
  path: crates/conductor/src/session.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: inferred
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/instance.yaml
revision: 23
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T10:59:44Z", actor: "human:timo", revision: 23, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Design §4: one daemon on the control host owns the config and makes the running instances follow it; today the operator installs `conductor_<instance>-run.service` by hand per instance and the registry `host/instances.json` is written by `conductor init`.

## Acceptance

- `conductor daemon serve` reads the config, starts the runtime of every instance that is absent, stops the runtime of every instance gone from the config, restarts one whose placement or binary changed, and records one `Reconciliation` per action in the deployment's store (`daemon.store`, `story:store-shared`); a pass with nothing changed records nothing (a test counts).
- Reconciliation runs on a config file change, every `daemon.cadence.reconcile` (default 60 s), and when a runtime stops answering.
- `GET 127.0.0.1:<daemon_port>/v1/instances` lists every instance with placement, state, binary version and last seen, from the config and what answers; `host/instances.json`, the registry types and the prefix check against the file are deleted.
- `conductord.service` installs with `conductor daemon --install`; `--uninstall` stops it; `conductor run --install` and the per-instance run unit are deleted.
