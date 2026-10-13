---
format: aep.planning-md/3
id: story:sweep-dead-requesters
kind: story
status: active
title: The sweep closes requests and grants whose build run is gone
relations:
- decomposes: epic:disk-protocol
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T22:39:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T22:39:08Z", actor: "human:timo", revision: 3}
---
## Why

Conductor reported on 2026-10-13/14: a grant whose `build run` died after it was granted but before
its command started stayed Granted with no process group and counted against free disk for 2.5 hours,
so eight waiting builds made no progress; ten requests whose waiters had died stayed Requested; and
`release-grant` refused them with a message that did not say why.

## What

A `build run` holds its run lock from its request to its release. The sweep before every reservation
answers refused a request no run holds, releases a granted grant no run holds, and releases a running
grant whose process group is gone and no run holds. `release-grant`'s refusal says the grant is a
request, or was released or refused.

## Acceptance

- `crates/conductor/tests/build_run.rs`: `a_request_or_grant_whose_run_is_gone_is_closed_by_the_next_run`
  (a dead request is refused, a dead 6G grant released, and the 1G run that would not fit beside it
  runs); the run-lock and group-gone tests still pass.

## Scope

`crates/conductor/src/host.rs`, `crates/conductor/src/build.rs`, `crates/conductor/tests/build_run.rs`.
