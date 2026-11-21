---
format: aep.planning-md/3
id: story:upgrade-checks-daemon-protocols
kind: story
status: implemented
title: instance upgrade refuses a release whose runtime needs a protocol the running conductord lacks, and a boxed upgrade answers only once its start reached the token pool
summary: GET /v1/version names protocols; upgrade refuses on a missing one; a boxed runtime probes the pool
refs:
- provider: github
  reference: beyond10x/conductor#223
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/daemon/dial.rs
- confidence: cited
  path: crates/conductor/src/daemon/upgrade.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/box_upgrade.rs
- confidence: cited
  path: crates/conductor/tests/instance_upgrade.rs
- confidence: cited
  path: spec/domains/store.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:40:00Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:40:01Z", actor: "human:unknown", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:37Z", actor: "human:unknown", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Issue #223. `conductor instance upgrade --tag v1.16.0` under a conductord still on 1.15.0 came up
("answers on 1.16.0") and then every spawn failed: "the host's token pool: conductord closed the
connection unanswered: this runtime's start is not the one it routes". A 1.16.0 runtime in a box
opens each token pool call with `CONDUCTOR-POOL/1` (`instance.rs` `POOL_HELLO`, `token.rs`
`dial_call`/`ask_pool`, wave 57's host token pool); conductord 1.15.0 knows only
`CONDUCTOR-DIAL/1` (`daemon/dial.rs` `greeting`, absent in 1.15.0), so it refuses the first line
and closes, and `ask_pool` reports any empty answer as a route mismatch. The upgrade's health
check (`daemon/upgrade.rs` `settle`) takes a boxed runtime as answering once substrate says its
session runs, which never asks the pool.

## Outcome

- `GET /v1/version` names `protocols`, the runtime protocols the daemon speaks
  (`spec/domains/store.yaml` `conductor.store.Build`/`Version`, `instance.rs` `PROTOCOLS`).
- `conductor instance upgrade` reads it before any upgrade and refuses, exit 1, naming the
  daemon's version and the protocol it lacks, when the release's runtime needs one it does not
  speak (`instance.rs` `RUNTIME_NEEDS`, `daemon/upgrade.rs` `daemon_lacks`); a daemon that names
  no protocols speaks those of the releases up to its version.
- A runtime in a box probes conductord's pool at its start until answered (`token.rs`
  `keep_probing`, pool call `probe`); conductord keeps the start that probed (`token::probed`),
  and from `token::PROBE_SINCE` on an upgrade of a boxed runtime is answered only once its new
  start reached the pool, else it is rolled back at `daemon.upgrade_timeout`.

## Acceptance (fail on main)

- `tests/instance_upgrade.rs`
  `instance_upgrade_refuses_a_release_whose_runtime_needs_a_protocol_the_daemon_lacks`,
  `instance_upgrade_reads_the_protocols_the_daemon_names`.
- `tests/box_upgrade.rs`
  `an_upgraded_boxed_runtime_answers_only_once_its_start_reached_the_token_pool`,
  `an_upgraded_boxed_runtime_that_never_reaches_the_token_pool_is_rolled_back`.

## Boundaries

- No change to the pool's lease protocol; a 1.16.0 runtime under a 1.15.0 daemon is refused, not
  made to work.
- A local (non-box) runtime leases from the store itself; its health check is unchanged.
