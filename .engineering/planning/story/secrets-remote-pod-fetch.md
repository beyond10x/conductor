---
format: aep.planning-md/3
id: story:secrets-remote-pod-fetch
kind: story
status: draft
title: An instance pod reads its secrets from a secrets service itself, by reference, with its own identity
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read in the code at v1.7.0: every secret an instance's mapping names reaches its pod through a
Kubernetes Secret.

- A mapping's `secret:<name>` resolves against the daemon's secrets source, and the daemon
  copies each value into the Secret `conductor-instance-<name>` at every pod start
  (`crates/conductor/src/daemon/pods.rs`, the manifests and the start).
- The pod mounts that Secret as files, and its rendered config names the source `files`
  (`crates/conductor/src/config/pod.rs`).
- The source kind `remote` is declared and answers nothing (`crates/conductor/src/secret.rs`).
- So a person's model token or forge token is a plain value in a Secret of the instance pods'
  namespace for the pod's life, readable by everyone who may read Secrets there.

The operator ruled that out for the secrets of persons: they rest encrypted in a secrets
service, and no Kubernetes Secret holds one.

## What

An instance pod reads the secrets its mapping names from a secrets service itself, by
reference, with an identity of its own.

- The daemon's secrets source can be the remote service. The daemon then copies no value of an
  instance into a Secret; the instance's Secret holds the rendered file and the runtime's API
  token only.
- The pod carries a projected service-account token for the service's audience. Nothing else of
  the service account is mounted.
- The runtime resolves `model_token` and `forge_token` from the service when it needs them: the
  model token's helper per call, the forge variable at the pod's start or by a credential
  helper, as the story decides.
- No value is written into a mapping, a log line, an event, a `detail` or a file on the
  instance's volume.

## Acceptance

- `spec/` declares what a `remote` source needs: the service's address, the audience of the
  token, and what names the tenant.
- With such a source, the Secret of an instance pod holds no key but the rendered file and the
  runtime's API token; a test reads the manifests of an instance that names `model_token` and
  `forge_token`.
- The pod's spec names a service account and one projected token volume with that audience, and
  mounts no other token. A daemon without such a source renders the pod it renders today, byte
  for byte.
- A session started in such a pod gets its model token from the service through the helper; a
  `subscription_token` reaches the harness's process as today, read from the service at the
  session's start. A test runs against a stand-in of the service that checks the bearer and the
  action.
- The service refusing, not answering or not holding the name are three declared errors, each
  one line that names the secret's name and never a value.
- The guide says which Role and which grant on the service's side a pod's subject needs, that
  every session of a pod can read what the pod's subject may read, and what an operator sees
  when the grant is missing.
- Each case fails before the change. No test calls a real service or uses a real token.

## The cheaper alternative, not taken here

The daemon alone reads from the service at each pod start and copies the values into the
instance's Secret as today. It changes the daemon's secret source alone; this story also changes
the pod's spec, the runtime's secret source and the model token's helper. The daemon's own
Secret no longer holds a person's value, and the
instance's Secret still does, for the pod's life. It leaves the pod's shape as it is and needs
one grant, the daemon's, instead of one per pod subject.

## Open

- One service account for the instance pods' namespace or one per instance. The service's
  grants name a subject, and the first matching entry decides the tenant, so one account for
  all instances gives every instance every other instance's secrets.
- The service's workload read does not check the secret's owner. What separates two persons'
  values is the tenant and the name alone.
- Whether the forge token stays a pod variable, read once at the runtime's start and held in
  its environment, or becomes a git credential helper that reads per call.
- A harness process holds a `subscription_token` in its environment for the pod's life; a
  replaced value reaches sessions after the pod starts again.
- The service's client is async and conductor's secret source is not; the story names where
  the runtime is.
