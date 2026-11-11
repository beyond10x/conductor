---
format: aep.planning-md/3
id: decision-blocker:cluster-placement-and-store
kind: decision-blocker
status: cleared
title: Nobody has decided conductor's placement and store in a cluster
relations:
- blocks: story:cluster-install
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-11-10T12:50:19Z", actor: "human:timo", revision: 3}
---
## Question

How does conductor run in a cluster beside a consumer control plane? Nothing written on either side decides it (the consumer's session, 2026-11-10).

- A: one conductord per repository group, each a StatefulSet with a volume, sqlite store, harness sessions inside the same pod. Cost: no shared view across groups; a pod restart ends its sessions.
- B: one conductord per cluster with the postgres store, instance runtimes placed as pods through the substrate `pod` adapter. Cost: needs a substrate daemon in the cluster and the postgres adapter in use for the first time outside tests.
- C: conductord stays on a host outside the cluster and only reports to the control plane over HTTP. Cost: nothing to deploy, and nothing runs beside the control plane.

Also open: the namespace (the consumer's or its own), and where an agent's worktree lives (a volume per instance or a clone per pod).

## Answer (operator, 2026-11-10)

A. One conductord per repository group as a StatefulSet with a persistent volume and a sqlite store on it; harness sessions run in the same pod. The other supported arrangement is a Postgres container in the cluster that conductord connects to with `daemon.store: postgres://…` (`story:store-postgres-proven` holds it); it is an option, not the default. Still open and left to `story:cluster-install`: the namespace, and where an agent worktree lives.
