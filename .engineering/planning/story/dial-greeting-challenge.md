---
format: aep.planning-md/3
id: story:dial-greeting-challenge
kind: story
status: draft
title: The dial-in greeting proves the runtime's token without sending it, and a replay is refused
summary: Challenge-response with a nonce from each side replaces the token in clear; a new token per daemon start
relations:
- decomposes: epic:architecture-next
- informed_by: story:host-token-pool
revision: 2
---
## Why

Adversary review of `story:host-token-pool` (2026-11-18). The greeting `CONDUCTOR-DIAL/1 <instance>
<token>` carries the runtime's token in clear (`crates/conductor/src/daemon/dial.rs`), and the runtime
never authenticates conductord. A local process that binds the dial port while conductord is down
receives the token and can dial in as that instance; with the host token pool that token reaches
every model token on the host, and a fake listener can answer leases with a token of its own.

## Outcome

- The runtime and conductord each send a nonce and answer the other's with an HMAC under the
  runtime's token; the token is never on the wire. A greeting replayed is refused.
- conductord mints a new runtime token on each start of the runtime.

## Acceptance

- `a_dial_greeting_does_not_carry_the_runtime_token_and_a_replayed_greeting_is_refused`.
- `a_runtime_refuses_a_listener_that_cannot_answer_its_challenge`.
