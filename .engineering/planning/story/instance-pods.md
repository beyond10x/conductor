---
format: aep.planning-md/3
id: story:instance-pods
kind: story
status: draft
title: The daemon runs each instance as a pod, and a control plane manages instances over its API
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Operator, 2026-11-11: the daemon runs in a Kubernetes namespace; a consumer control plane talks to it over HTTP to create, modify and list instances; an instance is a pod of its own; the daemon is the control layer the consumer shows in its own UI. Conductor ships no chart: it names the image, and the consumer writes its own chart around it. How the daemon creates an instance and keeps a route to it is conductor's concern, not the consumer's.

Read on 2026-11-11 at `v1.3.0`: the daemon ran in a pod from the released image, and each instance's runtime was a child process inside that pod (`decision-blocker:cluster-placement-and-store`, answer A). One instance could only be changed by writing the whole config file with `PUT /v1/config`, and only a person's bearer could write.

## Outcome

Where `daemon.kubernetes` is set, conductord creates, observes and deletes one pod per instance through the Kubernetes API with the service account its own pod mounts, and a machine caller holding the control token adds, reads, replaces, restarts and removes one instance over the daemon's HTTP API.

## Acceptance

- `daemon.kubernetes` (`namespace`, `image`, `owner`, `api_port`, `volume`, `resources`, `env`) and `daemon.control_token` are declared in the specification and validated by YAML path; `conductor daemon serve --daemon-config FILE` writes the daemon's section of the config file at every start and keeps the file's instances.
- `POST /v1/instances`, `GET`, `PUT` and `DELETE /v1/instances/{name}` and `POST /v1/instances/{name}/restart` are served with the statuses the HTTP binding declares; each change validates the whole resulting file first and records one config revision with the caller as its actor.
- A bearer equal to the control token is the machine principal `control-plane`: admitted for the reads and those five routes, refused on every other write and on the secrets routes.
- Per instance the daemon creates a Secret `conductor-instance-<name>`, a claim `conductor-instance-<name>-data` and a pod `conductor-instance-<name>` with the four labels; the pod passes the `restricted` pod security level, mounts no service-account token and carries no owner reference; nothing a caller sends reaches the pod's image, command, volumes, env sources, namespace or labels.
- The daemon's own secrets (the one `daemon.control_token` names, a `secret:<name>` password of `daemon.store`) reach no instance pod: a config that names one under `globals`, an instance or `daemon.kubernetes.env` does not validate, and the pod supervisor creates nothing for one.
- `conductor doctor` asks the cluster for the 10 rights the pod supervisor uses and no other: create, get, list and delete on pods and Secrets, create on volume claims, get on a pod's log.
- A lost pod is created again under the failed-start back-off; a daemon restart adopts its pods by label and restarts none; removal deletes the pod and the Secret and keeps the claim.
- On a local cluster: a daemon in a namespace, an instance added over the API, its pod Running, a restart, a deleted pod created again, a daemon restart with the pod kept, and a removal; the guide `website/docs/guides/run-in-kubernetes.md` holds the manifests that run used.

## Decisions taken while building (coordinator, 2026-11-11 and 2026-11-12)

- The daemon calls the Kubernetes API itself (a small client on rustls in `crates/conductor/src/kube.rs`), with no watch and no cache: one list of pods per tick. The substrate `pod` adapter is not on the wire and was not used.
- A machine caller is admitted by a control token (`daemon.control_token: secret:<name>`), not by the Identity service.
- No chart, no manifests in a release: the guide's manifests are the documented install. `story:cluster-install` holds that question.
- An API caller cannot shape the pod: every field of the three objects comes from `daemon.kubernetes`, the instance's name and the rendered config file.
- The claim outlives the instance: deleting it is the installer's act.
- After a security review (2026-11-12, `crates/conductor/tests/sec_instance_pods.rs`): an instance may not name a secret of the daemon's own; the daemon holds no right to read, list or delete a volume claim; the guide asks for a namespace that holds the instance pods and nothing else, since the daemon's rights there reach every Secret and ServiceAccount of it. Left as it is: every other secret of the daemon's source can be named into an instance pod by whoever holds the control token, and the row of an instance whose secret is missing names it.
- ess is pinned at 0.93 (`story:ess-pin-lifted`) and `ControlPlane` is the first `ActorKind` variant, both because of ess defects reported to the ess maintainers.

## Open with the operator

- A second sandbox layer inside the instance pod, or the pod boundary alone (built: the pod boundary alone).
- Which forge token an instance pod gets.
- One namespace for the daemon and its instance pods, or two (built: `daemon.kubernetes.namespace` names it; the guide asks for two).
