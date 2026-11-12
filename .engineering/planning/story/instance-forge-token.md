---
format: aep.planning-md/3
id: story:instance-forge-token
kind: story
status: draft
title: An instance pod carries its own forge token, or the daemon's
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 2
---
## Why

Read at `v1.4.0`: an instance pod gets a forge token only through `daemon.kubernetes.env`
(`GH_TOKEN` or `GITLAB_TOKEN` as a `secret`), so every instance pod of one daemon holds the same
token and reaches every repository that token reaches
(`website/docs/guides/run-in-kubernetes.md`, "Forge credentials"). No key of an instance names a
forge token: `forge_token: secret:<name>` appears only as an example of a secret reference
(`spec/domains/secret.yaml:16`). `story:instance-pods` left "which forge token an instance pod
gets" open with the operator.

Operator, 2026-11-12: the token may be one per daemon or one per instance; allow both.

## Outcome

Where `daemon.kubernetes` is set, an instance may name its own forge token as
`forge_token: secret:<name>`; its pod then carries that token in the variable its forge's command
line reads, and no other instance's pod carries it. An instance that names none keeps what
`daemon.kubernetes.env` gives every pod.

## Acceptance

- `forge_token` is a declared key of an instance in the specification, `secret:<name>` and
  nothing else, validated by YAML path. It names the forge through the instance's sources: with
  a `github` source the variable is `GH_TOKEN`, with a `gitlab` source `GITLAB_TOKEN`; an
  instance whose sources name no forge or two forges and that sets the key does not validate,
  and the refusal names the key and the reason.
- A config that sets `forge_token` on an instance without `daemon.kubernetes` does not validate:
  nothing but the pod supervisor reads the key.
- The pod of an instance that names a `forge_token` has that variable from its own Secret
  `conductor-instance-<name>` and carries none of the daemon's forge tokens: every
  `daemon.kubernetes.env` entry named `GH_TOKEN` or `GITLAB_TOKEN` stays out of that pod, and its
  secret out of the instance's Secret unless an entry the pod keeps names it too (found on a
  local cluster on 2026-11-12: with only the same variable replaced, an instance on one forge
  with a token of its own still carried the daemon's token of the other). The pod of an instance
  that names none keeps the daemon's entries.
- A `forge_token` that names one of the daemon's own secrets does not validate, and the pod
  supervisor creates nothing for one.
- `PUT /v1/instances/{name}` that changes the key replaces the pod once; the same body twice
  replaces nothing.
- On a local cluster: two instances, each with its own secret, plus one with none under a
  daemon-wide entry; each pod's variable is read from its own Secret and the three values differ
  as configured, shown without printing a value.
- The guide `website/docs/guides/run-in-kubernetes.md` and the config reference say both setups:
  one token per daemon, one per instance.

## Not in this story

A clone, a push and a merge request from an instance pod against a real forge; short-lived
tokens the daemon mints and refreshes; a token per source for an instance on two forges.
