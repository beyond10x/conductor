---
format: aep.planning-md/3
id: story:mailbox-rejection-once-dated
kind: story
status: implemented
title: A refused mailbox message is logged once, its sender told once, and recorded Rejected
summary: 'Issue #229: one dated journal line, one answer, the store''s message Rejected'
refs:
- provider: github
  reference: beyond10x/conductor#229
relations:
- decomposes: epic:conductor-run
scope:
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T01:06:26Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-20T01:06:27Z", actor: "human:unknown", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-20T02:20:15Z", actor: "human:unknown", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #229, observed on 1.15.0: the runtime log repeated, on every cycle, for five mailbox files,
`conductor run: mailbox <file>.md: request "<id>" lists no lettered option: one line each, ...`.
Expected: the rejection logged once per file, its sender told once, the line dated.

## Problem

- (a) Repeated: `mailbox.handled` kept only the names that sorted before the file handled now, so
  older names were acted on again (issue #219, fixed in 1.17.0, `run/mailbox.rs:89-97`).
- (b) Told: `answer_refused` (`run/mailbox.rs:184`) answers the sender with `[REPORT <repo> -]
  refused: <reason>` for a `[DECISION-REQUEST]` of a repository the instance knows
  (`known`, `run/mailbox.rs:909`); a request naming a repository it does not know is answered to
  no one, by design. What the loop does not do: record the rejection in the store. The message
  `conductor message send` recorded stays `Received` for ever, because `act` bails before
  `mark_handled` (`run/mailbox.rs:150`); with the decider seam `decide::parse_request`
  (`decide/handle.rs:982`) refuses before anything is raised, so no row says it was refused.
- (c) Dated: every line of the runtime log is led by its RFC 3339 time since
  `story:dial-refusal-diagnosable` (`instance.rs:473-560`, `log_to` and `stamped`; test
  `tests/instance_api.rs` `stamped`); `Tick::note` (`run.rs:685`) writes to standard error, which
  the stamp covers. Dating it again in `note` would date runtime-log lines twice.

## Outcome

- A mailbox message the loop refuses (any kind but a `[NEED]`, which only `route-need` moves) is
  moved to `Rejected` in the store with the reason (`message reject-message`,
  `conductor.dispatch.RejectMessage`, already in the specification: no new noun).
- Regression: a `[DECISION-REQUEST]` with no lettered option, over two ticks, with and without the
  decider seam: one journal line, one `refused` answer to its sender, the message `Rejected`.

## Acceptance (fail on main)

- `tests/run.rs` `a_decision_request_with_no_lettered_option_is_refused_once_and_its_sender_told_once`:
  fails on main on the store's `Rejected` (the message stays `Received`); the once-only and
  told-once parts are regressions for #219 and `story:report-on-sent-dispatch`.

## Boundaries

- No change to the line's form or to the runtime log's stamp; no new specification noun.
- A request naming a repository the instance does not know stays answered to no one.
