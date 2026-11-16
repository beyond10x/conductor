---
format: aep.planning-md/3
id: story:dial-refusal-diagnosable
kind: story
status: draft
title: A dial-in connection the daemon closes says why, and the runtime log is timestamped
summary: conductord logs the reason it closed a dialed connection, the runtime's line names the cause it can know, and every runtime log line carries a time
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-runtime-dials-conductord
scope:
- confidence: cited
  path: crates/conductor/src/daemon/dial.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: inferred
  path: crates/conductor/src/daemon/api.rs
revision: 1
---
## Why

Agentbox friction log F20 (2026-11-15, still on 1.13.0). The boxed runtime's
`state/sessions/runtime.log` held 78 lines of `conductor instance serve: dial
127.0.0.1:7301: the daemon closed the connection unanswered: it does not route this instance by
dial-in, or not with this token`. This was read on 2026-11-16 in a 402-line log. All the while
`/healthz` answered 200 and proxied reads worked. No line carries a timestamp, so the lines
cannot be matched with the daemon's journal. Read at v1.13.0:

- The runtime's text names two causes (`crates/conductor/src/dashboard.rs:1040-1047`). The
  daemon also closes an admitted connection silently when the instance already holds
  `MAX_IDLE` idle ones (`crates/conductor/src/daemon/dial.rs:36-57`), and conductord logs none
  of its closes.
- Another candidate is a second runtime still dialing with its old token after a daemon
  restart (F6, `story:daemon-restart-one-runtime`). Which cause produced the lines is not
  verified.

## Outcome

- conductord writes one journal line per closed dialed connection that names the cause. The
  causes are an unknown instance, a wrong token, a timed-out hello, or a full pool. Repeats
  are said once until the cause changes.
- The runtime's line says only what it knows (closed before any request).
- Every line of `runtime.log` starts with an RFC 3339 time.

## Acceptance

- A connection that dials with a wrong token, and one past the full pool, each produce a daemon
  line that names its cause. Every line the runtime writes to its log starts with a timestamp.
  `daemon::dial` tests and an instance API test hold both.
