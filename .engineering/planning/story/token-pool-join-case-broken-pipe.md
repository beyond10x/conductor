---
format: aep.planning-md/3
id: story:token-pool-join-case-broken-pipe
kind: story
status: draft
title: The agent-session join case does not fail on a closed pipe
tags:
- from-finding
relations:
- informed_by: story:secret-put-refusal-case-broken-pipe
revision: 1
---
## Outcome

The case `token_pool::an_agent_session_joins_nothing` passes whether or not the refused join has
exited before the case writes the token to its standard input.

## Why

`crates/conductor/tests/token_pool.rs:386` starts a join as an agent session, writes the token to
the child's standard input and expects the write to succeed. The join refuses an agent session
before it reads its input, so under load the child has exited when the write happens and the write
fails with `Broken pipe`. Seen once, on 2026-11-11, in a gate run of the instance-pods work that
did not touch the file; the same 12 cases passed when run alone. The cause is the one
`story:secret-put-refusal-case-broken-pipe` records for `secrets_source.rs`.

## Acceptance

Given a join that exits with `ActorNotHuman` before it reads its input, when the case writes the
token, then a failed write is not a failure of the case, and the case still asserts the exit
status, `ActorNotHuman` on standard error and that the token is printed nowhere and stored
nowhere.
