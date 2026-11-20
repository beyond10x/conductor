---
format: aep.planning-md/3
id: story:need-to-own-repo-not-echoed
kind: story
status: draft
title: A NEED that names its sender's own repository is not echoed back to the sender
summary: mailbox need() relays a [NEED <repo> <id>] <repo> back to <repo>'s controller, the session that sent it; it should stay for the supervisor as an unrouted need does
revision: 2
---
## Problem

A controller that needs something no other repository of the instance owns (a CI job's log the
bot API cannot read, an operator's step) has only `[NEED <repo> <id>] <other-repo> <what>`. Naming
its own repository as `<other-repo>` relays the message back to its own session
(crates/conductor/src/run/mailbox.rs `fn need`, the `other != repo` branch: "a need that names its
own repository ... waits for its own controller"), where nobody acts on it; the supervisor, who
reads conductor's mailbox, answered it only because it was told elsewhere. Naming a repository the
instance does not hold is answered `not relayed` and alerts `need-unroutable`, which is no better.

## Outcome

A `[NEED]` whose `<other-repo>` is the sender's own repository and whose sender is that
repository's running controller is not delivered back to it: it stays in conductor's mailbox for
the supervisor, the sender is answered once that it stays there, and no `need-unroutable` alert is
raised for it.

## Acceptance (fails on main)

- With `alpha`'s controller running, `[NEED alpha REQ-1] alpha the log of job 1` delivers nothing
  to `alpha`'s session but one answer `[REPORT alpha -] ... stays in conductor's mailbox ...`.
- A `[NEED alpha REQ-2] beta ...` routes as today.

## Boundaries

Mailbox routing only; the message forms are unchanged (design § 6).
