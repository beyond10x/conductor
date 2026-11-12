---
format: aep.planning-md/3
id: story:session-socket-path-length
kind: story
status: draft
title: A long session name gets a working broker socket or a refusal
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 at v1.4.0, derived and not run: each session's broker socket is bound at
`<state>/broker/sockets/<session>.sock` (`crates/conductor-broker/src/registry.rs`, the socket
path; `server.rs`, the bind) with no fallback for a long path, and a bind that fails during
register is ignored. A unix socket path holds 108 bytes. In an instance pod the fixed part of
the path is 50 bytes, so a session name over about 57 bytes cannot be bound, and that session's
tool calls fail with nothing reported. A controller's session is named
`<session_prefix>-<repository>`; `session_prefix` has no maximum, and an instance name of the
allowed 39 characters used as the prefix leaves 17 for the repository's name. The instance's own
API socket already falls back to a hashed path when its path would be too long
(`crates/conductor/src/instance.rs`).

A consumer control plane that derives instance names and prefixes from its own records asked
whether anything depends on the prefix being short (2026-11-12).

## Outcome

A session whose name is long gets a working broker socket, or its start is refused with the
reason; no session runs with a socket that was never bound.

## Acceptance

- A test binds the broker socket of a session whose name makes the path longer than a socket
  path may be, in a state directory as long as a pod's, and the session's tool call is
  answered; or the session's start is refused and names the length.
- A bind that fails during register is reported where the operator reads failures, not ignored.
- `session_prefix` gains a checked maximum, or the reference says which length is safe and why.
