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
revision: 2
---
## Why

A consumer control plane deploys its parts from a chart with a ConfigMap for config and Secrets created at install, and would ask the same of conductor (2026-11-10). Which workloads conductor is in a cluster is not decided: decision-blocker:cluster-placement-and-store.

## State on 2026-11-12

The placement question is answered twice: decision-blocker:cluster-placement-and-store (answer A, runtimes as child processes in the daemon's pod, shipped in v1.3.0) and then the operator's direction of 2026-11-11 (one pod per instance, no chart from conductor; story:instance-pods). The documented install as built is the guide `website/docs/guides/run-in-kubernetes.md`: manifests for a ServiceAccount, a Role and RoleBinding in the instances' namespace, a ConfigMap with the daemon's section, a Secret, a Service and a StatefulSet, run on a local cluster. A release attaches no manifest. The first acceptance line's "no service account token is mounted unless the chosen placement needs one" now reads: the daemon's pod mounts one, an instance pod mounts none. The third line's "one instance takes a snapshot" was not run on the cluster.

## Acceptance

- One documented install (plain manifests or a chart, per the decision) brings up conductord with its config in a ConfigMap and its credentials in Secrets that the install does not hold; no service account token is mounted unless the chosen placement needs one.
- The workload's probes are the endpoints of story:daemon-probes-and-metrics, and its image is the one of story:release-container-image, pinned by tag and digest.
- On a local cluster the install reaches `ready` and one instance takes a snapshot.
