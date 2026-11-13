---
format: aep.planning-md/3
id: story:forge-access-anonymous
kind: story
status: implemented
title: 'An instance pod clones public GitHub repositories with no forge credential when its mapping says forge_access: anonymous, and writes nothing'
relations:
- decomposes: epic:deployable-in-kubernetes
scope:
- confidence: inferred
  path: crates/conductor-docs/src/config.rs
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: inferred
  path: crates/conductor/src/collect
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/config/pod.rs
- confidence: inferred
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/daemon/pods.rs
- confidence: inferred
  path: crates/conductor/src/guard.rs
- confidence: inferred
  path: crates/conductor/src/spec_api.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: website/docs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:56:59Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-13T10:56:59Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-13T13:56:16Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Read in the code at v1.7.0: an instance that runs as a pod clones nothing without a stored
token of its forge.

- A pod that carries no token variable of a source's forge tries no clone; each repository is a
  member failure of the pass, "is not cloned: the pod carries no token of … so no clone is
  tried" (`crates/conductor/src/collect/checkouts.rs`, `brought`).
- The pod itself starts, so the instance looks alive and no session has a repository to work
  in.
- A missing `forge_token` says nothing about intent: it is also what a forgotten token looks
  like.

The case: a control plane starts an instance on public GitHub repositories to read them, with
no credential of that forge stored anywhere. It has itself established that each repository is
public before it sends the mapping.

## Acceptance

- The instance mapping takes the member `forge_access`, declared in `spec/`, with one value,
  `anonymous`. Absent, every behaviour is the one of today, the refusal above included.
- A mapping with `forge_access: anonymous` is refused with a declared error at the key's or the
  offending member's YAML path where: the value is another one; the instance names a
  `forge_token`; a source is not a `github` source; a `github` source has no `include`; the
  daemon runs no pods.
- The pod of such an instance carries no forge token variable and no credential helper: not
  the daemon's `gh` or `glab` entries of `daemon.kubernetes.env`, not a forge variable of its
  own. A test reads the rendered pod and the instance's Secret.
- The runtime of such an instance clones each included repository over HTTPS with no
  credential. A clone that fails is one member failure of the pass whose line says the
  repository is not public or not reachable, with git's last error line as today.
- Every write through the instance's forge route (a push, a pull request, a comment) is
  refused before any call to the forge or to git's remote, with one line that names
  `forge_access: anonymous`; the instance's `detail` carries one line that says the instance
  writes nothing to its forge.
- The collector's reads of pull requests and issues either run with no credential or are
  skipped with one line a pass. No read of such an instance uses a stored token, whichever
  source it belongs to.
- `GET /v1/instances/{name}` answers the member.
- The guide for Kubernetes says what the member does, what such an instance cannot do, and
  that whoever sends the mapping answers for the repositories being public.
- Each case fails before the change. No test calls a real forge: a clone is from a local
  repository or a recorded command, as the existing cases of the checkouts do.

## Not in this story

- `gitlab` and `local` sources beside the member.
- A mix of anonymous and credentialed sources in one instance.
- A check by conductor that a repository is public before the clone: the clone's result is the
  check.
- Any model credential or model route.

## Open

- Whether `gh` reads pull requests and issues of a public repository without a token inside
  the pod, and at which rate limit, is not shown by a test without a real forge. The story
  takes the stricter path unless a recorded command shows the read: skip with one line.
