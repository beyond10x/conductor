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
revision: 1
---
## Why

Design §7: the config names secrets, never holds them; a placement receives them as sealed slots. The source is the operator's choice (class H); the default is an age-encrypted file.

## Acceptance

- `daemon.secrets` names the source; the default implementation reads an age-encrypted file with the daemon's identity; the OS keyring and an environment source are behind the same trait.
- Every `secret:<name>` in the effective config resolves at instance start; a missing name is a named refusal before anything starts.
- `GET /v1/secrets` lists names and the instances that use them, never values; no log line ever carries a value (a test greps the daemon's log after a start).
- For `placement: host`, secrets reach the session the way they do today (`forge_token_env`), unchanged.
