---
format: aep.planning-md/3
id: epic:deployable-in-kubernetes
kind: epic
status: draft
title: Conductor is deployable into a Kubernetes cluster, beside the operator's assembly
relations:
- decomposes: initiative:adoption
- informed_by: story:placement-substrate-remote
- informed_by: story:store-shared
revision: 1
---
## Why

Operator, 2026-11-10: "deployable into k8s cluster alongside my assembly".

Read on 2026-11-10 at wave/49 3290f2211:

- The repository holds no container image build (no Dockerfile), no manifest and no chart. The release workflow (.github/workflows/release.yml) attaches binary archives for four targets only.
- The design covers the agents' side of a cluster: a placement names a remote substrate daemon by URL and the `pod` adapter runs one box per pod (docs/design/architecture-next.md:298-301; story:placement-substrate-remote, implemented).
- The store has a `postgres` adapter beside `sqlite` (story:store-shared, implemented; conductor.example.yaml:4 shows the sqlite default).
- The daemon and its dashboard are described for a workstation; the operator's wording of 2026-11-07 was "my daemon I would run locally". Nothing describes the daemon itself as a workload in a cluster.

## Outcome

Conductor's daemon, dashboard and instance runtimes run as workloads in a Kubernetes cluster, installed beside the operator's assembly in that cluster, with the shared store on postgres, agents placed through the `pod` adapter, and credentials taken from cluster secrets. A release publishes what the install needs (an image and the install manifests) next to the binary archives.

## Open before decomposition

- UNMAPPED: "my assembly". No document in this repository declares it, and the tree does not say what it is (a product, a namespace, a chart or a set of services). The relation to it is unknown as well: same namespace or a neighbour one, a shared postgres or its own, a shared ingress and identity or separate ones, one install unit or two.
- Whether the harness sessions (Claude Code today) run in pods of that cluster or stay on a remote host the daemon reaches.
- The install format: plain manifests, a Helm chart or a Kustomize base.
- Where an agent's worktree lives in a cluster (a volume per instance, or a clone per pod).

This epic is not decomposed: guardrail 7 of the planning skill asks for the new noun to be modelled first, and the first open point is the operator's to answer.

## Acceptance

- On a local cluster (kind), one documented install command brings up the daemon, the dashboard and one scratch instance; the instance runs one cycle with its agent in a pod and its records in postgres, and `conductor status` against that deployment shows it.
- A CI job runs that install and cycle on every release candidate.
