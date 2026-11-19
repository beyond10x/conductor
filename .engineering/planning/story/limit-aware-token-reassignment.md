---
format: aep.planning-md/3
id: story:limit-aware-token-reassignment
kind: story
status: active
title: A session at its usage limit moves to a pool token with headroom at once, else to the first token that resets
summary: The limit line marks the token limited until its reset; the session resumes on another token in the same tick, or on the token whose reset passes first, with no probe
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#206
relations:
- decomposes: epic:architecture-next
- depends_on: story:host-token-pool
- depends_on: story:limit-resume-at-reset
scope:
- confidence: inferred
  path: crates/conductor/src/limit.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: crates/conductor/tests/token_pool.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T14:13:04Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-18T14:13:05Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #206, item 3. A session stopped at its usage limit is not moved to a token with headroom on a
boxed host, and with every token limited it waits for an external probe:

- `token::on_limit` marks the session's token and answers whether another is free only where the
  runtime opens the daemon's database (`crates/conductor/src/token.rs:931-955`, returns `false`
  with no `runtime_store`); a boxed runtime never does, so `run/handlers.rs` `on_limit`
  (`crates/conductor/src/run/handlers.rs:545-566`) never resumes it on another token.
- `token::limit` marks the token limited for a fixed five hours (`token.rs:352-365`,
  `LIMIT_WINDOW` at `token.rs:75`), whatever the weekly limit's reset says.
- With no other token free the session waits in `limited` until the `Headroom` probe (`ais`)
  answers yes (`run/handlers.rs:648-655`, `crates/conductor/src/run.rs:124-135`); the resume then
  leases with `headroom = true` (`crates/conductor/src/spawn.rs:633-634`), which lets `pick` take a
  `Limited` token before its `limited_until` (`token.rs:207-212`).

`story:limit-resume-at-reset` (wave 56) parses the 5-hour and weekly reset from the session's limit
line (`crates/conductor/src/limit.rs`), resumes at it with no `ais`, and gives `token::limit` the
parsed reset; `story:host-token-pool` puts every token of the host in one pool that a boxed runtime
reaches through conductord. This story joins the two.

## Outcome

- A usage-limit line of a session marks the token it holds `Limited` until the reset the line names
  (5-hour or weekly), through conductord for a boxed runtime; with no parsable reset the token's
  limit window applies.
- When a token of the host pool has headroom (`Available`, or `Limited` with `limited_until` past),
  the session's lease moves to it and the session is resumed from its transcript in the same tick.
- When none has, the session waits and is resumed on the first token whose `limited_until` passes,
  at that instant, ahead of its own token's reset when another resets first.
- The limit is read from the session's own line and the tokens' state only: `pick` loses its
  `headroom` argument (a `Limited` token is taken only once its reset has passed), and the resume
  asks no probe.
- The `usage-limit` line of the dispatch log names the token and its reset; the resume's line names
  the token it moved to. Never a value.

Decision (class C, controller, 2026-11-18): the other live sessions on a token that reached its limit
are not interrupted; each moves when its own limit line arrives. Undone by moving them at once, should
waiting cost more than the turn it ends.

## Acceptance (tests that fail on the current code)

- `tests/run.rs` `a_limited_session_resumes_on_another_pool_token_in_the_same_tick`: two tokens, a
  limit line with a reset on the first; the session is resumed on the second within the tick, and the
  first is `Limited` until the parsed reset. The run test's `Headroom` seam fails the test if asked.
- `tests/run.rs` `with_every_token_limited_a_session_resumes_on_the_token_that_resets_first`: tokens
  limited until T+5h and T+1h; at T+1h the session resumes on the second.
- `token.rs` `a_limited_token_is_not_picked_before_its_reset`.
- `tests/token_pool.rs` `a_boxed_runtime_marks_its_token_limited_through_conductord`.

## Boundaries

- The parser and resume at reset with an empty pool are `story:limit-resume-at-reset`.
- How many sessions one token takes is `story:host-token-pool`'s pick.
