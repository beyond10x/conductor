---
format: aep.planning-md/3
id: story:box-subscription-token
kind: story
status: implemented
title: A boxed instance runs its agents on a subscription token
summary: The runtime in a box reads its model-token slot; a subscription_token there reaches the harness as CLAUDE_CODE_OAUTH_TOKEN
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: crates/conductor/tests/token_pool.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/concepts/config.md
- confidence: cited
  path: website/docs/concepts/the-host.md
- confidence: cited
  path: website/docs/guides/run-in-kubernetes.md
- confidence: cited
  path: website/docs/reference/config.md
- confidence: cited
  path: website/docs/security/operators-and-credentials.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-15T07:51:40Z", actor: "human:agent", revision: 4, executor: "human:timo"}
- {from: "proposed", to: "active", at: "2026-11-15T07:51:40Z", actor: "human:agent", revision: 5, executor: "human:timo"}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:39Z", actor: "human:unknown", revision: 7, decided_on: {"recorded":{"test_result":2}}}
---
## Why

An instance at a placement that opens a substrate box (`placement: systemd` or `linux`) could not
run its agents on a Claude subscription token (`claude setup-token`): validate refused
`model_token_kind: subscription_token` there, on the ground that a box receives its model token
through a slot of the daemon's pool as an API key. Reading the code showed more: the runtime in
the box never read the model-token slot at all. Substrate hands it over at descriptor 3, without
close-on-exec, and nothing consumed it, so every process the runtime started inherited the
descriptor, and the agents ran on whatever login the harness found. Claude Code reads a
subscription token only from `CLAUDE_CODE_OAUTH_TOKEN` in its process environment.

## Outcome

The runtime in a box reads the model-token slot (`model_token`, or a pool slot
`model_token_<i>`) once at its start, as it reads the daemon's token slot, and closes the
descriptor. For an instance whose `model_token_kind` is `subscription_token`, each agent whose
pool leases it no token starts with that value as `CLAUDE_CODE_OAUTH_TOKEN` in the harness's
environment and a settings copy that names no `apiKeyHelper`, the route a placement with no box
already takes; the `conductor session start` the runtime runs for the maintainer receives the
value in that variable and hands it on. The kind travels with the instance's config, which the
runtime in the box reads; the slot carries only the value. `api_key` keeps its behaviour: the
launch is left as it is.

## Acceptance

- `model_token_kind: subscription_token` validates at a placement that opens a box
  (`tests/config.rs`).
- A process the boxed runtime starts sees neither the daemon's token's descriptor nor the model
  token's (`tests/placement.rs`).
- `is_model_token_slot` knows `model_token` and `model_token_<digits>` only
  (`tests/placement.rs`).
- In a box, a `subscription_token` handed to `session start` reaches the harness in
  `CLAUDE_CODE_OAUTH_TOKEN` with no `apiKeyHelper` in the settings copy, and reads no secrets
  source; an `api_key` leaves the launch as it is (`tests/token_pool.rs`).

## Boundaries

- The token pool is unchanged. A pool token reaches every placement through the settings'
  `apiKeyHelper` (`conductor token reveal --token <id>`), never through a box's slot, and its
  record names no kind; a subscription token contributed with `conductor join` would be sent as
  an API key. Giving pool tokens a kind is a separate story. With the `files` secrets source,
  `conductor join` is refused (`SourceReadOnly`), so such an instance's pool stays empty and its
  agents run on the model token.
- A new value reaches a boxed instance's agents with the next start of its box: the runtime reads
  the slot only at its own start.
- An `api_key` in a box is still not handed to the harness; that is today's behaviour, kept.
