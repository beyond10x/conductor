---
format: aep.planning-md/3
id: story:host-token-pool
kind: story
status: active
title: 'The host''s token pool: conductord owns it and leases its tokens to every instance, boxed or not'
summary: One pool per host in conductord's store; join and leave change it with no config edit or restart; a token serves several sessions; a boxed runtime leases through conductord
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#206
relations:
- decomposes: epic:architecture-next
- depends_on: story:pool-token-kind-files-source
- informed_by: story:token-pool
- informed_by: story:box-subscription-token
scope:
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/daemon/dial.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/src/secret.rs
- confidence: inferred
  path: crates/conductor/src/store/token.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/token_pool.rs
- confidence: cited
  path: spec/domains/token.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T11:48:53Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-18T11:48:53Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #206, items 1 and 2. Every host today runs each instance on the token its config names, and the
pool that exists cannot serve a boxed instance:

- A pool token belongs to one instance. `pick` admits only `token.data.instance == *instance`
  (`crates/conductor/src/token.rs:204`), `pool` filters the same way (`token.rs:239-247`), and
  `contribute` names the secret `claude-token/<instance>/<id>` (`token.rs:449-453`). The join
  route resolves an instance of the config file before it takes a token (`token.rs:1035-1048`,
  `:1078`). Adding capacity means a token per instance, or a `model_token: secret:<name>` and a
  config edit per instance.
- A lease is exclusive: `pick` admits only `Available` tokens, or `Limited` ones (`token.rs:207-212`);
  a `Leased` token is never picked again, so eight controllers need eight tokens, and the ninth
  start is refused `NoTokenAvailable` (`token.rs:618-626`). `spec/domains/token.yaml:141-157` holds
  one `agent` per token.
- A boxed runtime cannot reach the pool. The runtime opens the daemon's database directly
  (`runtime_store`, `token.rs:585-588`), which a box does not mount, so `lease_for_start` finds no
  store and every boxed start falls back to the one `model_token` sealed into the box when it
  opened (`token.rs:854-866`, `crates/conductor/src/instance.rs:776-782`,
  `crates/conductor/src/placement.rs:26-31`). A token added later reaches a box only through a
  config edit and a box restart, which ends every running session.
- A daemon that runs pods refuses join and leave (`crates/conductor/src/daemon/api.rs:1474-1485`).

Wave 56 (`docs/waves/wave-56.md`, "Decisions taken in this wave") already decided: one pool per
host, shared by its instances; a boxed runtime leases pool tokens from conductord, so a token added
later needs no box restart; `model_token` stays the fallback when the pool is empty.
`story:pool-token-kind-files-source` gives each token its kind and lets `join` and `leave` write the
`files` source; this story builds on it.

## Outcome

- The pool is the host's: conductord's store holds it, and a token names no instance.
  `conductor join <daemon url>` contributes to the host's pool (`--instance` stays, for the operator
  record and grants it already writes); `conductor leave` withdraws. No instance config names a
  pool token, and adding or removing one restarts nothing.
- The daemon leases a pool token per session start of any instance on the host, the instance
  recorded on the lease. A token holds any number of leases at once; the pick takes a token with
  headroom (`Available`, or `Limited` with `limited_until` past) with the fewest live leases, ties by
  `token_id`; a role's `token_owner` keeps its rule.
- A boxed runtime leases, releases and marks limited through conductord, over the dial-in it
  already has (`crates/conductor/src/daemon/dial.rs`, the `conductord` aperture): a request the
  runtime opens, admitted by the token the daemon minted for that runtime. The lease answer carries
  the token's kind and value once; the runtime keeps it in memory for that start and hands it to the
  harness as `token::lease_into` does today (`CLAUDE_CODE_OAUTH_TOKEN` for a `subscription_token`,
  the helper for an `api_key`, whose `conductor token reveal` asks conductord the same way inside a
  box). No value is in a log, a record, an argument or the box's environment.
- With an empty host pool, an instance that names `model_token` runs on it as today; one that names
  none starts as today's empty pool does.
- Spec first (AGENTS.md rule 1): `PoolToken` loses `instance`, a lease becomes its own record
  (`TokenLease`: token, agent, instance, leased_at), `ContributeToken` and `JoinInstance` take no
  required instance, `NoTokenAvailable` names the host, and the lease, release and limit calls of a
  boxed runtime are bound in `crates/conductor/http.yaml`; `ess specify validate`, `task regen`, then
  the handlers. `conductor store migrate` keeps the tokens of an older store: each per-instance
  token joins the host pool as it is.

Decision (class C, controller, 2026-11-18): a token serves several sessions at once (one lease per
session, not per token), and the pick spreads sessions over the tokens with headroom by their live
leases. Undone by a per-token lease cap, a config key, should one token prove too few for its leases.

Decision (class C, controller, 2026-11-18): a boxed runtime reaches the pool over its existing
dial-in aperture rather than a new egress aperture, since substrate takes at most four
(`placement.rs:113`) and a GitLab forge already uses three.

## Acceptance (tests that fail on the current code)

- `tests/token_pool.rs` `a_token_joined_to_the_host_is_leased_by_two_instances_whose_config_names_none`:
  one `join` with no `--instance`, two instances with no `model_token`; each start leases it, the
  config file is byte for byte unchanged.
- `tests/token_pool.rs` `one_token_serves_several_sessions_at_once`: one token, three starts, three
  leases, no `NoTokenAvailable`.
- `tests/token_pool.rs` `a_boxed_runtime_leases_a_token_joined_after_its_box_opened`: a runtime with a
  dial-in address and no daemon database leases, through a test conductord, a token joined after the
  runtime started; its harness gets the value in its variable; no restart.
- `tests/token_pool.rs` `no_value_of_the_host_pool_reaches_a_log_or_a_record` greps the daemon's and
  the runtime's logs and the store after a lease, a limit and a release.
- `ess specify validate --path spec` exits 0 with the conformance scenarios of the new commands.

## Boundaries

- Moving a limited session to another token is `story:limit-aware-token-reassignment`; showing the
  pool in `conductor status` is `story:status-token-pool`.
- A pod instance keeps its `409` and `story:pod-instance-model-token-pool`.
- One pool per host; a pool shared by several hosts is not in this story.

## Adversary review (2026-11-18), taken into this story

- A session process never holds what admits the runtime to conductord: an `api_key` lease reaches
  the harness at start as a `subscription_token` does, or through a capability bound to that one
  lease; `conductor token reveal` checks the caller holds the lease.
- A lease is keyed by instance and agent; conductord takes the instance from the dial admission,
  never from the request; release and limit match the lease's instance.
- A token's usage state (Ok, Limited, Withdrawn) is apart from its lease records; the pick counts
  live leases; a lease never runs on a withdrawn token; leases of a runtime start that ended are reaped.
- `limited_until` is at most a week and a margin ahead, and only the holder marks its token limited.
- The value is in the harness process's environment only, never in the box spec's; the runtime's
  children inherit neither `CLAUDE_CODE_OAUTH_TOKEN` nor `ANTHROPIC_API_KEY`.
- The security page says a joined token serves every instance on the host, and that `store migrate`
  moves per-instance tokens into the host pool; `join` prints the instances it will serve.

Further acceptance (tests that fail on the current code):

- `a_session_process_cannot_lease_or_reveal_a_token_it_does_not_hold`
- `a_runtime_cannot_lease_release_or_limit_for_another_instance_s_agent`
- `releasing_one_of_two_leases_leaves_the_token_in_use`
- `a_lease_racing_a_leave_never_runs_on_a_withdrawn_token`
- `leases_of_a_runtime_that_ended_are_reaped`
- `a_limit_further_than_a_week_ahead_is_clamped`

Risk named, not decided here: a subscription token belongs to one person's plan; several
concurrent sessions of several instances use its windows that many times faster, and sharing it
may conflict with the provider's terms. An `api_key` or a commercial seat suits a shared pool.

Decision (operator, 2026-11-18): a config key caps a token's concurrent sessions; by default 4 per
`subscription_token` and no cap for an `api_key`. The pick never gives a token more live leases than
its cap; with every token at its cap a start waits as with an empty pool. A token at its limit hands
its sessions to another (`story:limit-aware-token-reassignment`). This replaces the class C decision
above on how many sessions a token serves.

- `a_subscription_token_serves_at_most_its_cap_of_sessions`
