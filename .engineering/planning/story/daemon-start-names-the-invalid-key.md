---
format: aep.planning-md/3
id: story:daemon-start-names-the-invalid-key
kind: story
status: draft
title: A daemon whose config does not validate at its start names the key
tags:
- from-finding
relations:
- informed_by: story:instance-pods
revision: 1
---
## Why

Reported by a consumer control plane on 2026-11-12, not reproduced here: the released image `v1.3.0`, started in a cluster with a daemon-config that holds `daemon.kubernetes` (a key that release does not know), never became Ready, and nothing told the operator which key the file was refused for. The operator saw a pod that stays not ready.

Not read yet at the current `main`: what an unattended daemon prints and what `/readyz` answers when its config file does not validate at its start.

## Outcome

A daemon whose config file or daemon-config file does not validate at its start says which key and why, where an operator of a cluster looks: in the pod's log, and in the body of `/readyz` while it is not ready.

## Acceptance

- Given a daemon-config file with a key the binary does not know, when `conductor daemon serve --daemon-config FILE` starts, then one line on standard error names the YAML path and the reason, and either the process exits non-zero or `/readyz` answers `503` with that path in its body; a test holds whichever the design chooses.
- The guide `website/docs/guides/run-in-kubernetes.md` says which image version first reads `daemon.kubernetes`, so a reader with an older image knows why its daemon is not ready.
