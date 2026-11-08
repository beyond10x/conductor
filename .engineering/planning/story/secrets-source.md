---
format: aep.planning-md/3
id: story:secrets-source
kind: story
status: implemented
title: the daemon resolves secret:<name> through the secrets crates, keychain locally and the Secrets service remotely, and hands them to boxes as slots
summary: no secret in config, env or argv; secrets-keychain and secrets-remote behind secrets-federation; names only on the API
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:config-globals
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:26Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:26Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-08T19:46:04Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Design §9: the config names secrets, never holds them; a placement receives them as sealed slots. Operator, 2026-11-08: "with local OS keychain - remotely something else of the b10x arsenal". The `secrets` workspace (0.6.0) already has the storage port, the OS keychain backend (`secrets-keychain`), the service backend (`secrets-remote`) and the router (`secrets-federation`); conductor writes no trait of its own. ADR 0110 (proposed): an agent process holds no credential for an external system.

## Acceptance

- `daemon.secrets` names a namespace per source (`keychain` for the control host, `remote: <url>` for a deployment) routed by `secrets-federation`; the daemon refuses to start without one; `conductor doctor` names the source and whether it answers.
- Every `secret:<name>` in the effective config resolves at box open (`ResolveSecrets`); a missing name is a named refusal before anything starts.
- An agent's box receives one slot, the model token; forge writes go through the gates route; `GET /v1/secrets` lists names, sources and the instances that use them, never values; no log line carries a value (a test greps the daemon's log after a start).
- `conductor secret put <name>` writes to the keychain through the same crates, so the operator never edits a secrets file by hand.
