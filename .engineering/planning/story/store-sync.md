---
format: aep.planning-md/3
id: story:store-sync
kind: story
status: archived
title: the daemon syncs every instance's store, local and remote, and restores a box from its copy
summary: GET store/events?since= on the instance API; one synced SQLite file per instance at the daemon; conductor instance restore
relations:
- decomposes: epic:architecture-next
- depends_on: story:store-sqlite
- depends_on: story:instance-api
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "archived", at: "2026-11-08T00:13:20Z", actor: "human:timo", revision: 4}
---
## Why

Design §8: every instance keeps its authoritative store in its box; the daemon's synced copy is what the UIs read and what survives the box (operator, 2026-11-08: "with sync").

## Acceptance

- `GET /v1/store/events?since=<id>` on the instance API returns events in order; the daemon pulls every instance into `<daemon state>/sync/<instance>.sqlite` every `daemon.cadence.sync` (default 30 s) and on an event notification; a test shows the copy complete within one cadence for a local and for a remote placement.
- The federated views and the dashboard read the synced copies, so an unreachable instance still shows its last state with its last sync time.
- `conductor instance restore <name>` seeds a new runtime's store from the copy and refuses when the runtime already has events the copy lacks.
- `conductor.store`'s `Sync` view (`StoreSync`) reports per instance the last event id copied, the time and the lag.
