---
format: aep.planning-md/3
id: story:pod-instance-model-token-pool
kind: story
status: draft
title: A pod instance takes more than one model token and resumes on another after a usage limit
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 2
---
## Why

Read in the code and the guide at v1.6.0: an instance that runs as a pod runs every session on
one model token, and a usage limit on that token stops its sessions until the limit ends.

- The token pool (`spec/domains/token.yaml`, `crates/conductor/src/token.rs`) leases one token
  per agent start (`lease_into`), marks a token `Limited` at a usage-limit stop and says
  whether another is available (`on_limit`), and the loop then resumes the agent on it.
- A pod instance cannot use it. What a pod instance has is one `model_token`, named in its
  mapping.
- Neither `GET /v1/instances/{name}/work/{reference}` nor the instance's health tells a control
  plane that a usage limit stopped the unit's session.

The case: the people who use an instance each give a token; a control plane stores the values
and tells conductor to build the instance's pool from references to them; the instance keeps
working when one token meets its limit.

## The shape this draft is written to (a lean, not a decision)

The instance request carries a list of model token references. Each names a Secret of the
instance pods' namespace and a key in it (the reference shape and the label rule of the forge
credential by reference, which is a story of its own), an owner by a name that is no secret,
and the kind `subscription_token` or `api_key` (`story:model-token-kind`). Conductor builds
the instance's pool from the list: one lease per agent start, `Limited` with its time at a
usage-limit stop, a resume on another token. `GET /v1/instances/{name}/tokens` answers, per
token, the owner, the state and `limited_until`, and never a value.

Open beside it, to decide before anything is built: whether the control plane, not conductor,
leases (it hands one token per session start, or swaps the one token of a running instance,
and conductor only reports the stop). This draft describes the first; the second needs the
report and nothing else of the list below.

## What stands in the way in a pod today, and what each needs

1. The store per pod. The pool's records are in the daemon's store; a runtime in a pod keeps a
   store of its own and does not read the daemon's (`crates/conductor/src/daemon/api.rs`,
   about line 1414). Needed: the pool of a pod instance lives in the pod's store and is built
   there from the rendered mapping at start, or the daemon keeps it and answers lease and limit
   calls of the runtime. The first changes the pod's start and the runtime; the second adds a
   call from pod to daemon, which the namespace's network rule does not admit today. One unit
   either way; the first is the smaller change to the network.
2. `conductor join` and `conductor leave` answer `409` for a pod instance (same place; the
   guide's Limits). Needed: nothing, if the list in the mapping is the only way a pod
   instance's pool is filled; the `409` then stays and the guide says where the pool comes
   from. The entity's own open question (which command creates a pool,
   `spec/domains/token.yaml`, `TokenPool`) is answered by this: the mapping does.
3. Values fixed at pod start. A pod reads its secrets through `secretKeyRef`
   (`crates/conductor/src/daemon/pods.rs`), so a token added to the list, or a changed value,
   reaches a pod only when it is replaced. Needed for a first version: nothing, a changed list
   replaces the pod once as any change of the rendering does, and the story says that sessions
   end with it. Needed to add a token without ending sessions: a route by which a value
   reaches a running pod, which the forge credential's refresh also needs; that route is
   designed once, there.
4. The headroom check. After a usage-limit stop with no other token available, the loop asks
   `ais status --json --provider claude` whether the account has headroom again
   (`crates/conductor/src/run.rs`, `Ais`), and the release image holds no `ais`. Not run: what
   a pod's loop does then. Needed: a pod instance resumes by the token's `limited_until`
   instead (the usage-limit line names the reset time), and the missing program is no error.
   One unit, with a test of a harness double that prints the limit text.
5. The route of a leased token. `apply` names the leased token in `apiKeyHelper`
   (`conductor token reveal --token <id>`). A subscription token needs the variable route of
   `story:model-token-kind` for a leased token as well. Part of the pool unit, after that
   story.
6. The report. The pool has a read (`/v1/instances/{instance}/tokens` in the binding); whether
   the daemon forwards it for a pod instance, and whether it carries `limited_until`, is not
   read yet. The work row gets no member for a usage-limit stop today. Needed: the forward and
   one member of the work row (or the token read alone, if a control plane joins the two).
   One unit.

## Acceptance (to be written once the lean is confirmed)

- A pod instance whose mapping lists two model tokens, the first of which meets a usage limit,
  resumes the stopped session on the second: shown with a harness double.
- With one token limited and no other available, the session resumes after `limited_until`
  with no `ais` in the image.
- A control plane reads per token the owner, the state and `limited_until`, and reads that a
  unit's session stopped on a usage limit; no answer holds a value.
- A reference to a Secret without the label is refused; a kind the specification does not
  declare is refused.
- No token value in a log line, a command line, an answer or a file other than the instance's
  Secret.
- An instance with one `model_token` and no list behaves as before.

## Notes

- Not built and not accepted. After `story:model-token-kind` and after the forge credential by
  reference, whose reference shape and label rule it uses.
- Whose usage a session draws on is visible to the people who gave a token: the story says
  which names the API shows to whom.
