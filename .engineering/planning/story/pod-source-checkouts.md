---
format: aep.planning-md/3
id: story:pod-source-checkouts
kind: story
status: draft
title: An instance pod gets the checkouts of its forge sources
relations:
- decomposes: epic:deployable-in-kubernetes
- depends_on: story:source-include-list
- depends_on: story:pod-git-reaches-forge
scope:
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/tests/kubernetes_guide.rs
- confidence: cited
  path: crates/conductor/tests/pod_checkouts.rs
- confidence: cited
  path: website/docs/guides/run-in-kubernetes.md
revision: 3
---
## Why

Read on 2026-11-12 at v1.4.0: conductor clones nothing. A checkout is read only where a
directory under the checkouts root already holds `.git`, and a controller's start fails with
"is not a git checkout" where it does not (`crates/conductor/src/spawn.rs`, the checkout
check). On a workstation the operator's checkouts are there before conductor is. In an
instance pod the checkouts root is an empty directory on a new volume claim
(`/var/lib/conductor/data/checkouts`), so an instance created over `POST /v1/instances` with a
`github` or `gitlab` source lists the forge's repositories and can start no agent in any of
them until somebody runs `git clone` inside the pod. The Kubernetes guide does not say so.

A consumer control plane that creates instances over the instance routes asked for this on
2026-11-12: it holds no right to run a command in the instance's namespace and should not get
one.

## Outcome

An instance in a pod gets a checkout of each repository its forge sources include, made by the
pod itself with the pod's own forge token, and keeps it fetched; nobody runs a command inside
the pod.

## Decisions (2026-11-12, reversible)

- The pod clones the repositories a source's `include` list names
  (`story:source-include-list`), and no others. A forge source without `include` behaves as in
  v1.4.0: nothing is cloned. Reason: a group of several hundred projects would fill the claim,
  and an upgrade must not start cloning for instances that run today.
- Only in an instance pod. On a workstation and in a single container the operator's checkouts
  are the operator's.
- Over HTTPS, through git's credential helper of `story:pod-git-reaches-forge`: no token in a
  URL, in `.git/config` or in a log line.

## Acceptance

- At its start and at each collect pass, a pod clones every included repository of its
  `github` and `gitlab` sources that has no checkout, to the path the collector reads it from,
  with `origin` on the source's host. An archived repository is not cloned.
- A checkout that is there is never replaced, reset or re-cloned; a directory at the path that
  holds no `.git` is left alone and reported.
- An included repository's checkout is fetched at the collect pass, as a `github` checkout is
  today.
- A clone that fails (no right, no such repository, no token of that forge in the pod, the
  claim full) is reported per repository where the control plane can read it without a command
  in the pod: the instance row's `detail` or a field beside it, and `conductor doctor`. It
  stops neither the other clones nor the pod, and is tried again at the next pass.
- The guide's section on the calls says where a checkout must be, that the pod clones what
  `include` names and nothing else, which variables a self-hosted GitLab needs
  (`GITLAB_TOKEN`, `GITLAB_HOST`), and which rights of a token conductor's own calls and a
  session's merge request and push need.

## Not in this story

- SSH remotes.
- A clone on a workstation or in a single container.
