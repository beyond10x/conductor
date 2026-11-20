---
format: aep.planning-md/3
id: story:instance-api-refusal-said-once-flake
kind: story
status: implemented
title: A dialing runtime says its refusal once, on every run
summary: instance_api refusal-said-once test counts a substring two lines carry; whether the pool probe prints the second depends on a reset versus a clean close
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/token.rs
- confidence: cited
  path: crates/conductor/tests/instance_api.rs
- confidence: cited
  path: crates/conductor/tests/token_pool.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T12:18:40Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-20T12:18:40Z", actor: "human:unknown", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-20T14:31:48Z", actor: "human:unknown", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`crates/conductor/tests/instance_api.rs` `a_dialing_runtime_says_a_refusal_once_and_a_surplus_dial_not_at_all`
failed once on CI at its assertion `:1673-1677` (`said once`, left 2, right 1) and passed on the same
code before and after.

- The assertion counts the substring `closed the connection` in the runtime's log
  (`instance_api.rs:1674`). Two different lines carry it:
  - the dialer's refusal, `dial <address>: the daemon closed the connection before any request: its
    journal says why` (`crates/conductor/src/dashboard.rs:1066-1072`, said once by `dial_forever`,
    `:1018-1037`), the line the test means;
  - the pool probe's, `the host's token pool: conductord closed the connection unanswered: this
    runtime's start is not the one it routes; probing again` (`crates/conductor/src/token.rs:1176-1181`,
    said by `keep_probing`, `:1107-1132`), started with the dial (`crates/conductor/src/instance.rs:847-851`).
    It runs in this test too: the runtime's wrong token is refused on its pool connection as on its
    dial connections.
- Whether the probe's line is that one depends on TCP timing. A pool call writes the greeting and
  then the request as two writes on a new connection (`token.rs:1168-1170`). The dial-in listener
  reads the greeting byte by byte and never the request (`crates/conductor/src/daemon/dial.rs:306-339`),
  refuses the wrong token (`dial.rs:221-228`) and drops the stream. If the request has arrived, the
  close is with unread data and the kernel sends a reset: `read_line` fails and the probe says
  `the host's token pool: read conductord's answer: Connection reset by peer` (`token.rs:1173-1175`),
  which the count does not match: the usual pass. If the listener closes before the request arrives,
  the close is a clean end of stream: `read_line` reads nothing and the probe says
  `closed the connection unanswered` (`token.rs:1176-1181`): the count is 2, the failure seen.
- So the test's matcher is ambiguous, and the product's diagnosis of one refusal of a pool call
  reads two ways by timing: one names the refusal, the other a reset that names nothing.

## Outcome

- A refused pool call reads the same however the close arrives, decided on the runtime's side in
  `token.rs` `ask_pool`: once connected, a reset or broken pipe on writing the greeting or request
  (`ask conductord: ...`, `token.rs:1168-1170`, the rarer third variant of the same race), a reset
  on reading before any answer byte (`token.rs:1171-1175`), and an end of stream before any answer
  byte (`token.rs:1176-1181`) all give the one existing error `the host's token pool: conductord
  closed the connection unanswered: this runtime's start is not the one it routes`, byte for byte,
  with no io cause appended: `keep_probing` says a line each time the error text changes
  (`token.rs:1121-1126`), so two texts for one refusal would still print twice.
- The dial-in listener is not changed to drain or half-close a refused connection: it would read
  an unauthenticated peer's bytes after refusing it, and the runtime-side mapping removes the race
  without it.
- Other io errors (connect refused, timeouts, an answer that is no JSON) keep their own texts.
- The test counts the line it means: the dialer's refusal (`before any request`) exactly once, and
  the probe's refusal (`closed the connection unanswered`) at most once, each matched by text no
  other line carries.

## Acceptance (tests that fail on the current code)

- `tests/token_pool.rs` `a_refused_pool_call_reads_the_same_whether_the_close_resets_or_ends`: a
  test listener that reads the greeting line, waits (by `peek`) until the request is in its buffer,
  then drops the stream with it unread (a reset, deterministic on Linux), and one that reads the
  request too before dropping (an end of stream); `token::hold_dial` at each and
  `token::probe_pool` answer `Some(Err(e))` with the same `e` both times, equal to the
  `closed the connection unanswered` text. On main the first answers `read conductord's answer:
  Connection reset by peer`: it fails on every run.
- `tests/instance_api.rs` `a_dialing_runtime_says_a_refusal_once_and_a_surplus_dial_not_at_all`
  rewritten: `before any request` counted exactly 1, `closed the connection unanswered` at most 1,
  and the log holds no `read conductord's answer` and no `ask conductord` line; with the product
  change it holds whichever way the probe's connection closes.
- The other tests that touch a refused pool call still pass unchanged: `tests/token_pool.rs`
  `a_session_process_cannot_lease_or_reveal_a_token_it_does_not_hold` (its own `pool_call`, which
  matches no text) and `tests/instance_upgrade.rs`'s refused-protocol test (matches `refused` and
  `CONDUCTOR-POOL/1`, not the pool error). No product code matches either text (`grep` of `src/`).

## Boundaries

- No change to what the dial-in listener reads, says in its journal, or how it closes; to `Said`'s
  once-per-cause rule; or to the dialer's retry and its line.
- No change to the pool protocol's lines (`POOL_HELLO`, the JSON request and answer), to
  `keep_probing`'s cadence (`PROBE_EVERY`) or to how `lease`, `release_for_stop` and the limit call
  propagate a pool error: only the text of a reset-before-answer error changes, to the one an end
  of stream already gives.
- No specification change: the error is a runtime log and error text, not a modelled error.
- The new test holds the process's one dial as the other dial tests of `tests/token_pool.rs` do
  (the `DIALED` lock, `:1719-1721`), serialized with them.
