---
format: aep.planning-md/3
id: story:report-is-tied-to-its-session
kind: story
status: draft
title: A report moves a dispatch only when the session it was sent to wrote it
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

The second security review of the work call (`review-result:work-call-security-round-2`,
finding 1, confirmed by a failing case) read how a report reaches a dispatch. Every session of
an instance writes into one mailbox; `message send` is open to every session and takes any
`--from`. The loop takes a line `[REPORT <repo> <dispatch id>] done …` for the dispatch named
when that dispatch is of `<repo>`. Nothing ties `<repo>` to the session that sent the line. So
a session working in repository A can write a report that names repository B and B's dispatch
consistently, and the next tick moves B's dispatch to Done and makes that line B's
`last_report`.

This was so before the work call. The work call makes it readable from outside: a control
plane that reads `GET /v1/instances/{name}/work/{reference}` is told `Done` with a report text
by a session that did not do that unit.

Also read in that review: the `dispatch report-*` commands move any dispatch by its id alone;
the guard denies them to controllers, and a controller that runs a script file the guard does
not read gets around that.

Not decided: what a session's identity is for this check. The store records the sending
session as the message's actor; `--from` is the sender's own word.

## Acceptance

- A report line is taken for a dispatch only where the session that sent the message is the
  one the dispatch was sent to (or a session of that dispatch's repository: the story says
  which and why). The reviewer's case
  `a_session_of_one_repository_does_not_report_for_another_repositorys_unit` in
  `crates/conductor/tests/sec_work_route.rs` runs, without its `ignore`, and is green.
- A report of the right session still moves its dispatch; the conductor's own session and the
  operator can still report for any dispatch, and the test names how they are told apart.
- `--from` alone does not pass the check: a case sends as another session's name and is
  refused or handled as a report naming no dispatch.

## Notes

- Pre-existing, found by a review of a later change: it waits in the backlog.
- The case is in the suite with `#[ignore]` naming this story, so the suite stays green and the
  case is not lost.
