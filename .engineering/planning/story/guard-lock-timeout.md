---
format: aep.planning-md/3
id: story:guard-lock-timeout
kind: story
status: draft
title: A store lock timeout in the guard says so, not a denial of the tool
revision: 1
---
## Why

Observed in a live run: the guard's "the verdict was not recorded: the store did not take it within
2 s" reached a controller as a denial of the tool call. The session cannot tell a rule from a busy
store. The timeout is raised in `crates/conductor/src/guard.rs` (the `recv_timeout(RECORD_DEADLINE)`
branch, line 344).

## Acceptance

When recording the verdict times out, the hook's message says the store was busy and that a retry
is safe; the verdict itself (allow or deny) is decided by the rules, not by the timeout.
