---
format: aep.planning-md/3
id: story:instance-removal-volume-claim
kind: story
status: draft
title: Removing an instance says what happens to its volume claim, and a setting decides it
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Observed in one run of an instance on a Kubernetes cluster at 1.7.0, reported by a consumer:

- `POST /v1/instances` answered 201, the pod was Running 1/1 after 29 s and cloned its one
  repository.
- `DELETE /v1/instances/<name>` answered 200. The daemon's log held "removed by" the caller and
  "Stop (ConfigChange): Succeeded". The pod and the instance's Secret were gone within 10 s.
- The volume claim `conductor-instance-<name>-data` (10G, the storage class of
  `daemon.kubernetes.volume`) stayed Bound with its volume. No line of the daemon's log names it.

Read in the code at v1.7.0: the removal deletes the pod, then the Secret, and never the claim
(`crates/conductor/src/daemon/pods.rs`, the removal and the sweep). The claim is created at the
pod's start and a conflict there is accepted, so an instance made again under the same name gets
the old volume.

So every removed instance keeps a volume until someone deletes the claim by hand, and nothing
tells the caller.

## What

- The removal of an instance says what it leaves: the answer to the `DELETE` and one line of the
  daemon's log name the claim when it is kept.
- A stated setting of the daemon decides whether a removal deletes the claim or keeps it. The
  story decides the default and writes why.
- The guide says what a kept claim costs, what an instance made again under the same name finds
  on it, and how to delete it.

## Acceptance

- With the setting on delete, a case removes an instance against the stand-in cluster and reads
  the claim's delete after the pod's and the Secret's; a refused delete is one line naming the
  claim, and the removal still answers.
- With the setting on keep, the answer to the `DELETE` names the claim, and so does one log line.
- `spec/` declares the setting and the member of the answer.
- A claim that carries another instance's label or no label of the daemon is never deleted.
- Each case fails before the change. No test calls a real cluster.
