---
format: aep.planning-md/3
id: story:pod-instance-model-token-pool
kind: story
status: draft
title: A pod instance takes more than one model token and resumes on another after a usage limit
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read in the code and the guide at v1.6.0: an instance that runs as a pod runs every session on
one model token, and a usage limit on that token stops its sessions until the limit ends.

- The token pool (`spec/domains/token.yaml`, `crates/conductor/src/token.rs`) leases one token
  per agent start, marks a token `Limited` at a usage-limit stop and resumes the agent on
  another token that is `Ok`.
- A pod instance cannot use it: `conductor join` and `conductor leave` answer `409` for it,
  since each instance pod has a store of its own, which does not read the daemon's pool
  (`website/docs/guides/run-in-kubernetes.md`, Limits).
- What a pod instance has is one `model_token`, named in its mapping.
- The loop asks `ais status` whether the account has headroom again
  (`crates/conductor/src/run.rs`), and the release image holds no `ais`. Not run: what a pod's
  loop does with the sessions a usage limit stopped.
- Neither `GET /v1/instances/{name}/work/{reference}` nor the instance's health tells a control
  plane that a usage limit stopped the unit's session.

The case: the people who use an instance each give a token, and the instance keeps working
when one of them meets its limit.

## Open, to decide before anything is built

Which side leases a token to a session start:

- A. The pool inside the pod: the instance's own store holds the tokens, as on a host, and the
  mapping names several secrets. The control plane only names them.
- B. The control plane: it hands the instance one token per session start, or swaps the one
  token of a running instance, and conductor only reports which session stopped on a limit.

The story does not choose. Each option needs: where a token's value rests, who can read which
person's token, what the API reports at a usage limit, and what resumes a stopped session.

## Acceptance (to be written once the open point is decided)

- A pod instance with two model tokens whose first meets a usage limit resumes the stopped
  session on the second, shown with a harness double.
- A control plane reads from the API that a session stopped on a usage limit, and on which
  token by a name that is no secret.
- No token value in a log line, an answer or a file other than the secrets source.
- An instance with one `model_token` behaves as before.

## Notes

- Not built and not accepted. After `story:model-token-kind`, which gives one token its kind.
