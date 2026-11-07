---
format: aep.planning-md/3
id: story:secrets-source
kind: story
status: draft
title: the daemon resolves secret:<name> references from an age file and passes them as slots
summary: no secret in config or env; the daemon's secrets source; names only on the API
relations:
- decomposes: epic:architecture-next
- depends_on: story:config-globals
revision: 2
---
## Why

Design §7: the config names secrets, never holds them; a placement receives them as sealed slots. ADR 0107 (Connectors issues access tokens), ADR 0023 (the Secrets service holds custody) and ADR 0110 (proposed: an agent process holds no credential for an external system) decide where secrets come from; a single host with neither service needs a local file. The source is the operator's choice (class H).

## Acceptance

- `daemon.secrets` names the source behind one trait with three implementations: Connectors (access tokens, never refresh tokens), the Secrets service, and an encrypted local file; a deployment names one explicitly, and the daemon refuses to start without one.
- Every `secret:<name>` in the effective config resolves at instance start; a missing name is a named refusal before anything starts.
- A session's box receives one slot, the model token; forge writes go through the gates route; `GET /v1/secrets` lists names and the instances that use them, never values; no log line carries a value (a test greps the daemon's log after a start).
- For `placement: host`, secrets reach the session the way they do today (`forge_token_env`), unchanged for one release.
