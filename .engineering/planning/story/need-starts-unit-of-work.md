---
format: aep.planning-md/3
id: story:need-starts-unit-of-work
kind: story
status: draft
title: A NEED to a repository with no charter becomes a unit of work
summary: The mailbox opens one dispatch for the chartless repository, and the cycle starts it
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
revision: 1
---
## Why

A `[NEED <repo> <id>] <other-repo> <what>` to a repository the instance knows but that has no
running controller is "kept for its next start" (`crates/conductor/src/run/mailbox.rs:18`). A
repository with no charter and no open dispatch never starts, so the NEED waits for a person to
send it a unit of work (#207).

## Outcome

A NEED kept for a repository with no charter and no open dispatch becomes a unit of work for it,
and the cycle starts its controller on the default charter.

## Acceptance

- The mailbox pass that keeps such a NEED opens one dispatch for `<other-repo>` with the NEED's text
  as its brief and `need:<request-id>` as its reference, once per request id.
- The requesting controller is told `[REPORT <repo> -] relayed as <dispatch-id>`.
- A NEED to a repository with a charter keeps today's behaviour.
- A test fails on main: a NEED to a chartless repository opens no dispatch.

## Boundaries

No dispatch for a repository the instance does not serve; the cap `max_working` applies as to any
dispatch.
