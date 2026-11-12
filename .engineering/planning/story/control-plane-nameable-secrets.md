---
format: aep.planning-md/3
id: story:control-plane-nameable-secrets
kind: story
status: draft
title: A control plane names only the secrets the operator allows it
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 on the branch of `story:instance-forge-token`: a caller that holds the
control token writes an instance mapping, and a mapping may name any secret of the daemon's
secrets source as `forge_token`, `model_token` or another `secret:<name>` key, except the
daemon's own (`secret::daemons_own`) and the secret of a daemon forge-token entry
(`secret::daemons_forge`). The daemon then writes that secret's value into the instance's
Secret, which the pod mounts. A security review of the forge token work
(`review-result:forge-token-security-round-1`, finding 2) names what follows: beside a `gitlab`
source whose host the caller chooses, a secret named as `forge_token` is offered by git to that
host. v1.4.0's release notes list the same reach as a known limit.

Whether that matters depends on what the operator keeps in the source: where every secret of it
is meant for the control plane's instances, nothing is gained; where the source also holds
secrets of other uses, the control token reaches them.

## Outcome

A caller that holds the control token names only the secrets the operator has allowed it to
name; a mapping that names another is refused, and nothing of it reaches a pod.

## Acceptance

- The daemon's section declares which secrets of its source an instance mapping written over
  the instance routes may name (a list of names or one prefix: the decision below); a mapping
  that names a secret outside it answers `422` with the YAML path of the reference, and the pod
  supervisor creates nothing for a file that names one.
- An instance the operator writes into the config file itself is held to the same rule, or is
  not, as the decision below says; the specification says which.
- Without the declaration a daemon behaves as it does today, and `conductor doctor` says in one
  row that the control token may name every secret of the source but the daemon's.

## Open with the operator

- A list of names, or a prefix (for example every secret whose name starts with `instance-`).
- Whether the default without the declaration is "every secret" (today) or "none".
