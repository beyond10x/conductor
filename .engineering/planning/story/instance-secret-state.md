---
format: aep.planning-md/3
id: story:instance-secret-state
kind: story
status: draft
title: A control plane reads whether an instance's secrets are set
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 at v1.4.0: an instance mapping names secrets (`model_token`, `forge_token`
and other `secret:<name>` keys) that resolve against the daemon's secrets source. A caller that
holds the control token cannot learn whether a name it wrote resolves: `GET /v1/secrets`
refuses the control token, and an instance whose secret is missing shows only as a refused
start in its row's `detail`.

A consumer control plane that writes the keys of the daemon's Kubernetes Secret itself, so that
no secret value passes the daemon's API, asked to read back "set or not set, and since when"
for what an instance names (2026-11-12).

## Outcome

A caller that holds the control token reads, for each secret an instance's mapping names,
whether the daemon's source holds it and when it last changed, and never a value.

## Decisions (2026-11-12, reversible)

- `GET /v1/instances/{name}/secrets`, answered by the daemon itself (not forwarded to the
  pod), admitted for the control token: one row per secret the instance's mapping names, with
  the key that names it, the secret's name, `set` (true or false) and, for a `files` source,
  the file's modification time. No value, no length, no digest.
- Only names the instance's own mapping names: no listing of the source.
- `GET /v1/secrets` is unchanged and keeps refusing the control token.

## Acceptance

- With the control token the call answers the rows above; a secret added to the mounted source
  reads `set: true` at the next call without a daemon restart.
- A name the daemon keeps to itself, or the secret of a daemon forge-token entry, never
  appears: a mapping that names one does not validate in the first place.
- No value of any secret is in the answer, in a log line or in an error.
- The specification declares the answer; the guide's list of calls carries it.
