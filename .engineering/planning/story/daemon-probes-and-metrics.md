---
format: aep.planning-md/3
id: story:daemon-probes-and-metrics
kind: story
status: draft
title: conductord answers liveness, readiness and a Prometheus metrics endpoint
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

A workload in a cluster is restarted and routed by probes. conductord answers `/v1/instances` and the dashboard; nothing is documented as a liveness or readiness endpoint, and there is no metrics endpoint a Prometheus can scrape. Asked for by a consumer control plane on 2026-11-10.

## Acceptance

- conductord answers a liveness path (200 while the process serves) and a readiness path (200 once the store answers and the config is read, 503 with the reason otherwise); both are in the specification's HTTP binding and the reference.
- A metrics path answers in the Prometheus text format with at least: instances by state, sessions by state, snapshot age, build queue length.
- A test starts the daemon with a store that does not answer and reads 503 from readiness and 200 from liveness.
