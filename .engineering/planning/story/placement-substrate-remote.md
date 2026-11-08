---
format: aep.planning-md/3
id: story:placement-substrate-remote
kind: story
status: implemented
title: a placement names a remote substrate daemon by URL; the box, the API and attach cross the network
summary: HTTPS/WSS with an Identity bearer; the instance API over substrate's transport; attach as a stream; the pod adapter in a cluster
relations:
- decomposes: epic:architecture-next
- depends_on: story:placement-substrate-local
- depends_on: story:store-shared
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:28Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:28Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T19:46:03Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-07: "the instances themselves, they should be capable of running remotely ... my daemon I would run locally"; 2026-11-08: "(bwrap, docker, k8s)". Design §6.

## Acceptance

- `placement: {daemon: https://…, bearer: secret:<name>, adapter: linux | oci | pod}` opens the box on that daemon through the SDK's remote transport; the instance API is reached through substrate's session transport and proxied by conductord unchanged.
- `conductor agent attach` and the TUI attach a remote agent as a PTY stream; `deliver` and the federated `sessions` view work the same.
- A remote substrate that does not answer makes the instance `unreachable` in every view, never an error; the synced copy keeps its last state and `instance restore` seeds a new box from it.
- The `pod` adapter runs one box in a cluster: the scratch instance runs one cycle there with its records in the shared store.
