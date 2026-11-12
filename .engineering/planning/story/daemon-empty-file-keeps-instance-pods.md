---
format: aep.planning-md/3
id: story:daemon-empty-file-keeps-instance-pods
kind: story
status: draft
title: A daemon that starts on an empty config file does not delete the instance pods it finds
tags:
- from-finding
relations:
- informed_by: story:instance-pods
revision: 1
---
## Why

Read in the code on 2026-11-12 (`crates/conductor/src/daemon/reconcile.rs`, the sweep after the stops; `crates/conductor/src/daemon/pods.rs`, `Pods::sweep`): a daemon that runs its instances as pods deletes every pod and Secret with its `conductor/daemon` label whose instance its config file does not hold. That is how a removed instance's objects go. It also means a daemon that starts on an empty or replaced volume, with a config file that holds no instance, deletes the pods of every instance it ran before, at its first pass. The claims stay, so no instance data is lost, but every running agent session of those instances ends, and the instance list is empty.

A consumer control plane asked on 2026-11-12 whether a Deployment whose home is copied fresh at each start could stand in for the StatefulSet of the guide. With that workload every restart of the daemon would do this. The guide now says so (`website/docs/guides/run-in-kubernetes.md`, "What survives a restart of the daemon").

## Outcome

A daemon whose config file was created at this start, and which finds pods of its owner label, does not delete them at its first pass without the operator or the control plane having said so.

## Acceptance

- Given a daemon that starts with a config file it wrote new at this start and a namespace that holds pods and Secrets of its owner label, when its first pass runs, then it deletes none of them, its `/v1/instances` answer or its doctor names them as objects no instance holds, and a line on standard error says how to remove them or how to add their instances again.
- Given a daemon whose config file existed before this start, when an instance is removed, then its pod and Secret are deleted as today.

## Open with the operator

- Whether the daemon should rebuild its instance list from what it finds (each instance's rendered config is in its Secret), keep the found pods untouched until told, or delete them as today. The acceptance above is written for the second.
