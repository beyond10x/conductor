---
format: aep.planning-md/3
id: story:daemon-reconcile
kind: story
status: active
title: conductor daemon serve reconciles instances from the config and lists them; the registry file goes
summary: the daemon starts and stops instance runtimes so the running set equals the config; GET /v1/instances; one Reconciliation per action
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 4}
---
## Why

Design §4: one daemon on the control host owns the config and makes the running instances follow it; today the operator installs `conductor_<instance>-run.service` by hand per instance and the registry `host/instances.json` is written by `conductor init`.

## Acceptance

- `conductor daemon serve` reads the config, starts the runtime of every instance that is absent, stops the runtime of every instance gone from the config, restarts one whose placement or binary changed, and records one `Reconciliation` per action in its own store; a pass with nothing changed records nothing (a test counts).
- Reconciliation runs on a config file change, every `daemon.cadence.reconcile` (default 60 s), and when a runtime stops answering.
- `GET 127.0.0.1:<daemon_port>/v1/instances` lists every instance with placement, state, binary version and last seen, from the config and what answers; `host/instances.json`, the registry types and the prefix check against the file are deleted.
- `conductord.service` installs with `conductor daemon --install`; `--uninstall` stops it; `conductor run --install` and the per-instance run unit are deleted.
