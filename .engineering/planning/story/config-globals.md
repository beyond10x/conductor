---
format: aep.planning-md/3
id: story:config-globals
kind: story
status: active
title: conductor.config/2 with globals, precedence, operators, config show --effective and PUT /v1/config
summary: global defaults, instance and role overrides; operators and grants; a validated, committed, recorded config write; config migrate from conductor.config/1
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:daemon-reconcile
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 4}
---
## Why

Operator, 2026-11-07: global settings plus per-instance settings; self-improvement tweaks settings. 2026-11-08: several operators of the instance cluster. Design §9, §5.3.

## Acceptance

- `conductor.config/2` has `daemon` (port, TLS listener, cadences, the secrets source, `operators[]` with grants), `globals` and `instances[]`; a key set on an instance or a role overrides the global; `conductor config show --effective --instance <i>` prints every key with the level it came from (a test covers all three levels); `roles.<role>.kind` and `placement` are keys at every level.
- `conductor config migrate` reads a `conductor.config/1` file once: `confinement.*` and `dashboard_port` are dropped with a note each, `host.roles` becomes `globals.roles`, `confinement.provider: systemd` becomes `placement: {daemon: local, adapter: systemd}`; the daemon refuses a `/1` file otherwise, naming the command.
- `PUT /v1/config` validates, writes the file, commits it to the config repository with the operator and reason, records `ConfigChanged` in the daemon's store and triggers a reconciliation; an invalid body changes nothing and names the key.
- The file never holds a secret: a value under a secret-bearing key must be `secret:<name>` (validation refuses anything else).
