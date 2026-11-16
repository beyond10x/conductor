---
format: aep.planning-md/3
id: story:host-commands-reach-boxed-session
kind: story
status: draft
title: The operator's session commands on the host reach a session in a box, or change nothing
summary: session deliver and controller stop-controller from the host act on a boxed child session through the runtime, and a command that cannot reach it refuses without ending the record
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-session-without-bg
- informed_by: story:decision-relay-recorded-answers
scope:
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: inferred
  path: crates/conductor/src/dashboard/agents.rs
- confidence: inferred
  path: crates/conductor/src/controller.rs
revision: 1
---
## Why

Agentbox friction logs F28, F36 and F46 (2026-11-15/16, conductor 1.13.0). The box runs
`conductor-ai-agent-platform` at `placement: systemd`. Its sessions are child-provider sessions
inside a bwrap box with `--unshare-pid` (observed).

- F28/F36: a controller's NEED waits in the mailbox for a supervisor role. None runs
  (`globals.roles: []`). The operator's `conductor session deliver` from the host is refused
  `SessionNotReachable` (`crates/conductor/src/session.rs:290`). The daemon's proxied
  `agents/{name}/deliver` route (`crates/conductor/src/daemon/api.rs:30`) admits only a person's
  write from the dashboard (`ActorNotHuman` otherwise, `api.rs:55-59`). So an operator at a
  terminal cannot answer a NEED.
- F46: `conductor controller stop-controller` from the host ended the record. The `claude -p`
  process (pid 3953270) lived on for 4 h, and the cycle refused that repository with
  `SessionRunning` until the operator killed it. The child stop looks for the session's entry
  under the caller's own HOME (`registry_here`,
  `crates/conductor/src/confine/child.rs:98-103`) and signals a pid
  (`child.rs:587-611`). Neither is the box's when the caller is on the host. Likely cause; not
  verified with a trace.

## Outcome

- On the host, `session deliver` and `stop-controller` for a boxed instance go through
  conductord to the runtime, over the dial-in connection, as the operator, with the operator's
  own grant. The runtime delivers to or stops its child session.
- A command that cannot reach the session refuses and leaves the controller record as it was.

## Acceptance

- On the host, against a boxed instance with a live child session, `session deliver` puts the
  text on the session's input. `stop-controller` ends the session's process before it marks the
  record stopped. With the runtime unreachable, both refuse and the record stays `Running`.
  Box-session tests with a fake substrate hold all three.

## Boundaries

- Whether an instance should run a supervisor role is the operator's config choice. This story
  gives the operator a path that needs none.
