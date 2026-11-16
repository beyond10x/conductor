---
format: aep.planning-md/3
id: story:pool-token-kind-files-source
kind: story
status: draft
title: A pool token carries its kind, and a files-source instance can take pool tokens
summary: conductor join records whether a token is a subscription token or an API key, and works where the secrets source is files
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-subscription-token
scope:
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/src/secret.rs
- confidence: inferred
  path: spec/domains/token.yaml
- confidence: inferred
  path: crates/conductor/tests/token_pool.rs
revision: 1
---
## Why

Agentbox friction log F5 (2026-11-15, instance `conductor-ai-agent-platform`, conductor 1.11.0
and still at 1.13.0). The box has no answering OS keychain (F4), so its daemon reads secrets from
the `files` source. With that source `conductor join` is refused `SourceReadOnly`
(`crates/conductor/src/secret.rs:580`, `crates/conductor/src/token.rs:477`), so the instance's
token pool stays empty and every agent runs on the one model token. Even where join works, a
pool token is always handed to the harness through `apiKeyHelper` as an API key, so a
subscription token contributed with `conductor join` would be sent as the wrong kind.
`story:box-subscription-token` names both in its Boundaries and says "Giving pool tokens a
kind is a separate story"; no such story exists in the store at v1.13.0
(`story:pod-instance-model-token-pool` is a pod-only draft).

## Outcome

- A pool token's record names its kind (`subscription_token` or `api_key`), given at join; a
  `subscription_token` reaches the harness as `CLAUDE_CODE_OAUTH_TOKEN` with no
  `apiKeyHelper`, as the model token of `story:box-subscription-token` does.
- Under the `files` source the pool's values are read from files the operator places in
  `daemon.secrets.dir` (one file per pool token, a name conductor derives), so `conductor join`
  either registers such a file by name or says exactly which file to create; conductor still
  never writes the files source.

## Acceptance

- On an instance whose secrets source is `files`, a token the operator registers as a
  `subscription_token` pool token is leased to an agent start, which receives it as
  `CLAUDE_CODE_OAUTH_TOKEN` with no `apiKeyHelper` in its settings copy (`tests/token_pool.rs`).

## Boundaries

- A pod instance's pool stays `story:pod-instance-model-token-pool`.
