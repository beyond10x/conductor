---
format: aep.planning-md/3
id: story:store-mirror
kind: story
status: draft
title: the daemon mirrors every instance's store and restores a box from the mirror
summary: GET store/events?since= on the instance API; a sqlite mirror per instance at the daemon; conductor instance restore
relations:
- decomposes: epic:architecture-next
- depends_on: story:store-trait
- depends_on: story:instance-api
revision: 1
---
## Why

Design §6.2: a remote instance keeps its authoritative store in its box; the daemon's mirror is what the UIs read and what survives the box.

## Acceptance

- `GET /v1/store/events?since=<id>` on the instance API returns events in order; the daemon pulls every instance into `<daemon state>/mirror/<instance>.sqlite` every `daemon.cadence.mirror` (default 30 s) and on an event notification; a test shows the mirror complete within one cadence.
- The federated views and the dashboard read the mirror, so an unreachable instance still shows its last state with its mirror time.
- `conductor instance restore <name>` seeds a new runtime's store from the mirror and refuses when the runtime already has events the mirror lacks.
