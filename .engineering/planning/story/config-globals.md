---
format: aep.planning-md/3
id: story:config-globals
kind: story
status: draft
title: conductor.config/2 with globals, precedence, config show --effective and PUT /v1/config
summary: global defaults, instance and role overrides; a validated, committed, recorded config write
relations:
- decomposes: epic:architecture-next
- depends_on: story:daemon-reconcile
revision: 1
---
## Why

Operator, 2026-11-07: global settings plus per-instance settings; self-improvement tweaks settings. Design §7.

## Acceptance

- `conductor.config/2` has `daemon`, `globals` and `instances[]`; a key set on an instance or a role overrides the global; `conductor config show --effective --instance <i>` prints every key with the level it came from (a test covers all three levels).
- A `conductor.config/1` file is read unchanged for one release: `confinement.provider` maps to `placement: host`, `dashboard_port` is ignored with a warning.
- `PUT /v1/config` validates, writes the file, commits it to the config repository with caller and reason, records `config-changed` in the daemon's store and triggers a reconciliation; an invalid body changes nothing and names the key.
- The file never holds a secret: a value under a secret-bearing key must be `secret:<name>` (validation refuses anything else).
