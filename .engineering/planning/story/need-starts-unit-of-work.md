---
format: aep.planning-md/3
id: story:need-starts-unit-of-work
kind: story
status: implemented
title: A NEED to a repository with no charter becomes a unit of work
summary: The mailbox opens one unit-of-work dispatch for the chartless repository and routes the NEED to it; the cycle starts it
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
- depends_on: story:loop-alerts-when-it-cannot-act
scope:
- confidence: cited
  path: crates/conductor/src/alert.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T19:25:00Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-19T19:25:00Z", actor: "human:unknown", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-20T02:20:14Z", actor: "human:unknown", revision: 9, decided_on: {"recorded":{"test_result":2}}}
---
## Why

A `[NEED <repo> <id>] <other-repo> <what>` to a repository the instance knows but that has no
running controller is "kept for its next start" (`crates/conductor/src/run/mailbox.rs:18`,
`need` at `:467`). A repository with no charter starts only for a unit of work
(`Tick::spawn`, `crates/conductor/src/run.rs:800`), so the NEED waits for a person to send it one
(#207). The design already says a NEED is routed as a dispatch (`docs/design/conductor.md:51-53`),
and the spec records the dispatch a NEED opened (`RouteNeed`, `spec/domains/dispatch.yaml:1973`;
`Message.dispatch_id`, `:601-607`); the loop never opens one.

## Outcome

A NEED for a repository with no charter, no running controller and no open dispatch becomes a unit
of work for it, which the cycle starts on the default charter; a NEED that names no repository of
the instance raises one alert.

## Acceptance

- In the mailbox pass, such a NEED opens one dispatch for `<other-repo>` as the work call does
  (`crate::dispatch::send_work`, `crates/conductor/src/dispatch.rs:187`): its brief the NEED's whole
  text, its reference `need-<repo>-<request-id>` with every character outside
  `A-Z a-z 0-9 . _ -` written `-` and cut to 64 characters (the form of
  `spec/domains/dispatch.yaml:799-805`). The message is routed to it (`RouteNeed`) and reads
  `Handled` with that `dispatch_id`; the NEED is not also kept in `pending.json`.
- The same NEED sent again opens nothing: `send_work` answers the dispatch that holds the
  reference.
- The requesting controller is told `[REPORT <repo> -] need <request-id> opened <dispatch-id> for
  <other-repo>`, delivered or kept as `deliver_or_keep` does.
- "No charter" means the charter file is missing, empty, or the default charter
  (`crate::spawn::is_default_charter`): a repository on the default charter also gets a
  controller only for a dispatch (`Tick::spawn`), so a NEED to it opens the unit too.
- A NEED for a repository with a charter of its own, a running controller or an open dispatch
  keeps today's behaviour, as does a NEED whose `<other-repo>` is the requesting repository.
- A NEED whose field names no repository of the instance (`NotRelayed`, `:482`) raises one alert
  through `crate::alert`, condition `need-unroutable`, subject `<repo> <request-id>`, besides
  today's answer to the sender.
- Tests in `crates/conductor/tests/mailbox.rs` that fail on main: a NEED to a chartless known
  repository opens a dispatch with that reference and the message reads `Handled`; sent twice it
  opens one; a NEED to a repository on the default charter opens one; a NEED naming no
  repository writes one `alert` line.

## Boundaries

No dispatch for a repository the instance does not know (`known`, `run/mailbox.rs:534`);
`controllers.max_working` applies to the start as to any unit of work.

## Scope

`crates/conductor/src/run/mailbox.rs`, `crates/conductor/tests/mailbox.rs`.

## Notes

Depends on `story:loop-alerts-when-it-cannot-act` for `crate::alert`. Shares no file with
`story:unit-merge-policy` or `story:failing-start-recreates-box`.
