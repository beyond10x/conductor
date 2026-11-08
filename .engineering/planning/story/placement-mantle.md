---
format: aep.planning-md/3
id: story:placement-mantle
kind: story
status: archived
title: 'placement: mantle runs a whole instance on a mantle worker'
summary: 'a kind: Session manifest per instance; the instance API over mantle''s forwarded socket'
relations:
- decomposes: epic:architecture-next
- depends_on: story:placement-substrate-remote
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-11-07T23:41:19Z", actor: "human:timo", revision: 2}
---
## Why

Design §5: a coarse placement for an instance whose sessions should live on a worker nobody else touches; the last migration step.

## Acceptance

- `placement: {mantle: {class: <placement class>}}` writes a `mantle.beyond10x.dev/v1alpha1` `Session` manifest (repositories from `sources`, agent kind, resources from limits, network capabilities from the egress list) and starts it with `mantle start`; stop and status map to `mantle stop|status`.
- The instance API rides mantle's forwarded socket; attach uses `mantle attach`'s PTY; the store is `sqlite://` in the worker with the mirror on.
- A worker that is gone makes the instance `unreachable`; `conductor instance restore` seeds a new worker from the mirror.
