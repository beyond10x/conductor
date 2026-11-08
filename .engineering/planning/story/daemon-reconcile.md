---
format: aep.planning-md/3
id: story:daemon-reconcile
kind: story
status: draft
title: conductor daemon serve reconciles instances from the config and lists them
summary: the daemon starts and stops instance runtimes so the running set equals the config; GET /v1/instances
relations:
- decomposes: epic:architecture-next
revision: 1
---
## Why

Design §3: one daemon on the control host owns the config and makes the running instances follow it; today the operator installs `conductor_<instance>-run.service` by hand per instance and the registry `host/instances.json` is written by `conductor init`.

## Acceptance

- `conductor daemon serve` reads the config, starts the run unit of every instance that is absent, stops the unit of every instance gone from the config, and records one `reconciliation` event per action in its own store; a pass with nothing changed records nothing (a test counts).
- Reconciliation runs on a config file change, every `daemon.cadence.reconcile` (default 60 s), and when a runtime stops answering.
- `GET 127.0.0.1:<daemon_port>/v1/instances` lists every instance with placement, state, binary version and last seen; `host/instances.json` is written from the same data and marked derived.
- `conductord.service` installs with `conductor daemon --install`; `--uninstall` stops it and leaves the instance units running.
