---
format: aep.planning-md/3
id: story:placement-substrate-remote
kind: story
status: draft
title: 'placement: substrate with a daemon URL runs an instance on a remote substrate host'
summary: HTTPS/WSS with an Identity token; the instance API over substrate's transport; attach as a stream
relations:
- decomposes: epic:architecture-next
- depends_on: story:placement-substrate-local
- depends_on: story:store-mirror
revision: 1
---
## Why

Operator, 2026-11-07: "the instances themselves, they should be capable of running remotely ... my daemon I would run locally." Design §5 and §6.2.

## Acceptance

- `placement: {substrate: {daemon: https://…, identity: secret:<name>}}` starts the runtime on that host through the SDK's remote transport; the instance API is reached through substrate's session transport and proxied by the daemon unchanged.
- `conductor session attach --name <s>` and the TUI attach a remote session as a PTY stream; `deliver` and `list` work the same.
- A remote substrate that does not answer makes the instance `unreachable` in every view, never an error; the mirror keeps its last state.
- The runtime's `store:` defaults to `sqlite://` inside the workspace for a remote placement, with the mirror on.
