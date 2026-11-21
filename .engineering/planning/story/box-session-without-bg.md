---
format: aep.planning-md/3
id: story:box-session-without-bg
kind: story
status: implemented
title: A boxed runtime's sessions run without claude --bg
summary: A child provider runs claude -p with stream-json input as a child process, needing no Unix socket
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-forge-work
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: crates/conductor/tests/fake_provider/mod.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: website/docs/concepts/the-host.md
- confidence: cited
  path: website/docs/guides/run-isolated.md
- confidence: cited
  path: website/docs/security/sessions.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-15T12:13:22Z", actor: "human:agent", revision: 2, executor: "human:timo"}
- {from: "proposed", to: "active", at: "2026-11-15T12:13:22Z", actor: "human:agent", revision: 3, executor: "human:timo"}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:38Z", actor: "human:unknown", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
## Why

No session starts in a substrate box. Read from a live instance at `placement: systemd` on
conductor 1.12.0 on 2026-11-15: every spawn of the runtime failed `LaunchFailed: claude --bg exited
1: … bg manager failed to start: Failed to listen at /tmp/cc-daemon-<uid>/<id>/control.sock`.
Substrate refuses `socket(AF_UNIX)` in every box (its ADR 0039 allows `AF_INET` and `AF_INET6`,
`NETLINK_ROUTE`, and a Unix socket only through `socketpair`), and Claude Code's `--bg`
background service listens on a pathname Unix socket, so it can never start in a box. The only
session provider a box could use was `none`, which is `claude --bg`; its stop is `claude stop`
and `claude rm`, its resume `claude --bg --resume <id>`, and a delivery to its session goes
through `claude attach` in a tmux pane, all of them through that service or a socket of tmux.

## Outcome

- A provider `child` starts the session as a plain child process of the process that starts
  it: `claude -p --input-format stream-json --output-format stream-json --verbose
  --replay-user-messages --session-id <uuid>` with the arguments `--bg` gets
  (`background_args`), in its own process group, its standard input a FIFO it holds open for
  reading and writing, its output in files. Conductor chooses the id, which names the
  transcript under `CLAUDE_CONFIG_DIR` as a `--bg` session's does.
- The session's entry (name, id, directory, pid, start time, exit code) and files live in
  `conductor-sessions/` of Claude Code's state directory. Every reader of the Claude session
  list (spawn, the sessions collector, the watch, the loop, the guard, the dashboard, build
  run) sees these sessions beside the ones `claude agents --json` prints, in that list's shape:
  live while the pid runs and names the id, `busy` from a message to its `result`.
- The first prompt and each `conductor session deliver` are one stream-json user message
  written to the FIFO, in place of `claude attach`. A stop sends `SIGTERM` to the process group,
  `SIGKILL` after 30 s, and removes the files; a resume is `claude -p --resume <id>` the same way.
- `conductor.config/1`'s `confinement.provider: child` asks for it; conductor also takes it in
  place of `none` wherever this process may not create a Unix socket (`EACCES`, `EPERM`,
  `EAFNOSUPPORT`), as in a box. `claude --bg` stays unchanged everywhere else.

## Acceptance

- Under a seccomp filter that refuses `socket(AF_UNIX)` as a box does, `conductor spawn` with
  `provider: none` starts the controller through `child`; the session takes the charter and a
  delivered message on its input, is listed live by the sessions collector with its id and the
  provider's record, is stopped by `pause-controller`, resumed from its id by
  `resume-controller`, takes the next message and is stopped by `stop-controller`; no command
  calls `--bg`, `attach`, `stop` or `rm`, and no socket file is made
  (`tests/box_session.rs`).
- A delivery to a name no child session holds is refused `SessionNotReachable`
  (`tests/box_session.rs`).
- The message line, the busy state and the liveness check (`src/confine/child.rs`).

## Boundaries

- `conductor session attach` and the dashboard's attach open no child session: it has no
  terminal. Its transcript and output files are the way to read it.
- The tool broker's session socket (`TOOL_SOCKET`) is a Unix socket: in a box a session
  reaches no broker tool. That stays as it is.
- The `child` provider measures no box of its own; its record says the session runs in the box
  of the process that starts it, so `confinement.mode: strict` refuses it.
