---
format: aep.planning-md/3
id: story:secret-put-refusal-case-broken-pipe
kind: story
status: draft
title: The refused secret put case does not fail on a closed pipe
scope:
- confidence: cited
  path: crates/conductor/tests/secrets_source.rs
revision: 2
---
## Outcome

The case `secrets_source::an_agent_session_puts_no_secret` passes whether or not the refused
`conductor secret put` has exited before the case writes the value to its standard input.

## Why

`crates/conductor/tests/secrets_source.rs:166` writes the value to the child's standard input
and expects the write to succeed. `secret put` refuses an agent session before it reads its
input, so under load the child has exited when the write happens and the write fails with
`Broken pipe` (os error 32). The case failed that way in two full package runs of wave 50 and
passed when run alone; the file was last changed in v1 wave 4, so the wave did not introduce it.

## Acceptance

Given a `conductor secret put` that exits 1 with `ActorNotHuman` before it reads its input, when
the case writes the value, then a failed write is not a failure of the case, and the case still
asserts exit 1, `ActorNotHuman` on standard error and that the value is printed nowhere.
