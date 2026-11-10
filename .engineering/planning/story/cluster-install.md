---
format: aep.planning-md/3
id: story:cluster-install
kind: story
status: draft
title: One documented install brings conductord up in a Kubernetes cluster
relations:
- decomposes: epic:deployable-in-kubernetes
- depends_on: story:release-container-image
- depends_on: story:daemon-probes-and-metrics
revision: 1
---
## Why

A consumer control plane deploys its parts from a chart with a ConfigMap for config and Secrets created at install, and would ask the same of conductor (2026-11-10). Which workloads conductor is in a cluster is not decided: decision-blocker:cluster-placement-and-store.

## Acceptance

- One documented install (plain manifests or a chart, per the decision) brings up conductord with its config in a ConfigMap and its credentials in Secrets that the install does not hold; no service account token is mounted unless the chosen placement needs one.
- The workload's probes are the endpoints of story:daemon-probes-and-metrics, and its image is the one of story:release-container-image, pinned by tag and digest.
- On a local cluster the install reaches `ready` and one instance takes a snapshot.
