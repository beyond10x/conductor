---
format: aep.planning-md/3
id: story:pool-token-kind-files-source
kind: story
status: active
title: A pool token carries its kind, and a files-source instance can take pool tokens
summary: conductor join records whether a token is a subscription token or an API key, and works where the secrets source is files
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-subscription-token
scope:
- confidence: cited
  path: crates/conductor/src/secret.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/token_pool.rs
- confidence: cited
  path: spec/domains/token.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 4}
---
## Why

Issue #200 (agent box friction F5) and #206 item 1. With the `files` secrets source `conductor join` is
refused `SourceReadOnly` (`crates/conductor/src/secret.rs:577-584`), and a pool token always reaches
the harness through `apiKeyHelper` as an API key (`token.rs` `apply`), even a subscription token.

## Outcome

- A pool token records its kind (`subscription_token` or `api_key`, `conductor.config.ModelTokenKind`),
  given at join (`--kind`, default `subscription_token`); a `subscription_token` reaches the harness
  as `CLAUDE_CODE_OAUTH_TOKEN` with no `apiKeyHelper`, an `api_key` through `apiKeyHelper` as today.
- Under the `files` source, `join` writes the token's file into `daemon.secrets.dir` (owner-only
  mode 0600, the name conductor derives) and `leave` removes it; no other secret is ever written there.
  Decision (class C, controller, 2026-11-18): #206 asks for "an equivalent that writes the files source".

## Acceptance

- `tests/token_pool.rs` `a_files_source_subscription_pool_token_reaches_the_harness_in_its_variable`:
  join under `files` writes the file 0600, a start leases it and gets `CLAUDE_CODE_OAUTH_TOKEN`, no
  `apiKeyHelper` in its settings copy. Fails on main.
- `tests/token_pool.rs` `leave_with_the_files_source_removes_the_token_file`.

## Boundaries

- A host-wide pool is `story:host-token-pool`; a pod's pool stays `story:pod-instance-model-token-pool`.
