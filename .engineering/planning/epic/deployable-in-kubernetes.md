---
format: aep.planning-md/3
id: epic:deployable-in-kubernetes
kind: epic
status: draft
title: Conductor is deployable into a Kubernetes cluster, beside a consumer control plane
relations:
- decomposes: initiative:adoption
- informed_by: story:placement-substrate-remote
- informed_by: story:store-shared
revision: 5
---
## Why

Operator, 2026-11-10: "deployable into k8s cluster alongside" a system of the operator's, named here a consumer control plane.

Read on 2026-11-10 at wave/49 3290f2211:

- The repository holds no container image build (no Dockerfile), no manifest and no chart. The release workflow (.github/workflows/release.yml) attaches binary archives for four targets only.
- The design covers the agents' side of a cluster: a placement names a remote substrate daemon by URL and the `pod` adapter runs one box per pod (docs/design/architecture-next.md:298-301; story:placement-substrate-remote, implemented).
- The store has a `postgres` adapter beside `sqlite` (story:store-shared, implemented; conductor.example.yaml:4 shows the sqlite default).
- The daemon and its dashboard are described for a workstation; the operator's wording of 2026-11-07 was "my daemon I would run locally". Nothing describes the daemon itself as a workload in a cluster.

## Outcome

Conductor's daemon, dashboard and instance runtimes run as workloads in a Kubernetes cluster, installed beside a consumer control plane in that cluster, with the shared store on postgres, agents placed through the `pod` adapter, and credentials taken from cluster secrets. A release publishes what the install needs (an image and the install manifests) next to the binary archives.

## Open before decomposition

- Answered on 2026-11-10 by the session that develops it: that system is a consumer control plane for system changes, deployed from a Helm chart into one namespace (a model server pod, a UI and a Prometheus), today on a local cluster only. It names conductor as one kind of work handler, one instance per group of repositories, and has tried nothing with conductor yet. Not decided on either side: conductor's placement and store in a cluster (decision-blocker:cluster-placement-and-store) and the work contract between the two (story:control-plane-work-handler, UNMAPPED there).
- Whether the harness sessions (Claude Code today) run in pods of that cluster or stay on a remote host the daemon reaches.
- The install format: plain manifests, a Helm chart or a Kustomize base.
- Where an agent's worktree lives in a cluster (a volume per instance, or a clone per pod).

Decomposed on 2026-11-10 from the consumer's answers into five draft stories: story:release-container-image, story:daemon-probes-and-metrics, story:container-start-without-prompt (the three it asked for first), story:cluster-install (blocked on the placement decision) and story:control-plane-work-handler (not ready: its contract is not modelled here). The plan-critic panel has not read them.

## Direction of 2026-11-11

Operator, 2026-11-11: the daemon runs in a namespace and is the control layer a consumer control plane talks to over HTTP (create, modify, list instances); an instance is a pod of its own; conductor ships no chart and names its image, and the consumer writes its own chart around it. How the daemon creates an instance and keeps a route to it is conductor's concern.

That answers three of the questions above: the instance runtimes run in pods the daemon creates (story:instance-pods), the install format is no chart of conductor's (the guide `website/docs/guides/run-in-kubernetes.md` holds manifests a reader adapts; story:cluster-install), and an instance's worktrees live on a volume claim per instance. The Outcome and the Acceptance below still read as written on 2026-11-10 (postgres, the `pod` adapter, one install command, a CI job): whether they change with the direction is the operator's to say.

## Acceptance

- On a local cluster (kind), one documented install command brings up the daemon, the dashboard and one scratch instance; the instance runs one cycle with its agent in a pod and its records in postgres, and `conductor status` against that deployment shows it.
- A CI job runs that install and cycle on every release candidate.
