---
format: aep.planning-md/3
id: story:need-to-own-repo-not-echoed
kind: story
status: implemented
title: A NEED that names its sender's own repository is not echoed back to the sender
summary: mailbox need() relays a [NEED <repo> <id>] <repo> back to <repo>'s controller, the session that sent it; it should stay for the supervisor as an unrouted need does
scope:
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: docs/design/conductor-run.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T04:41:03Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-20T04:41:03Z", actor: "human:unknown", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-20T05:33:47Z", actor: "human:unknown", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

A controller that needs something no other repository of the instance owns (a CI job's log the
bot API cannot read, an operator's step) writes `[NEED <repo> <id>] <other-repo> <what>`. Naming
`conductor` as `<other-repo>` already keeps the need for the supervisor, unanswered
(`other_repository` answers `None` for the recipient, crates/conductor/src/run/mailbox.rs:913-915
and :704; module doc :20). A controller that names its own repository instead (it reads the form
as "the repository this is about") gets its need back: in `fn need` (mailbox.rs:689) the
`other != repo` test (:714) skips the unit-of-work path, and :729-732 hand the whole text to
`deliver_or_keep` with `repo: other`, the sender, deliberately (:712-713, "as before"). Nobody acts
on it there.

## Outcome

A `[NEED]` whose `<other-repo>` equals its `<repo>` is neither delivered to nor kept for that
repository, whether its controller runs or not: it stays in conductor's mailbox for the supervisor,
as a need to `conductor` does, the sender is answered once that it stays there, and no
`need-unroutable` alert is raised.

## Acceptance (fails on main)

- With `alpha`'s controller running, `[NEED alpha REQ-1] alpha the log of job 1`: the need's text is
  neither delivered to nor kept for `alpha`; `alpha` gets exactly one message,
  `[REPORT alpha -] ... stays in conductor's mailbox ...`; the message record stays `Received`
  (mailbox.rs:1084-1085); no `need-unroutable` line is raised.
- The same need with `alpha` not running keeps nothing in `alpha`'s pending messages.
- `[NEED alpha REQ-2] beta ...` routes as today.

## Boundaries

Mailbox routing only; the message forms are unchanged (design conductor.md § 6), no new noun or
error, so no spec change. docs/design/conductor-run.md:36 (the cases a `[NEED]` stays for the
supervisor) gains "or names its sender's own repository"; the module table (mailbox.rs:20) and the
doc of `need` (:674-688) say the same.

## Scope

crates/conductor/src/run/mailbox.rs, crates/conductor/tests/mailbox.rs (or tests/run.rs),
docs/design/conductor-run.md.
