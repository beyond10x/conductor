---
format: aep.planning-md/3
id: story:spawn-paste-buffer-per-call
kind: story
status: archived
title: A launch's paste uses a buffer of its own, so a live session is never reported LaunchFailed
summary: spawn printed LaunchFailed (no buffer conductor) for a session that started and got its prompt
relations:
- decomposes: epic:loop-without-handwork
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-11-08T09:58:08Z", actor: "human:timo", revision: 2}
---
# Story: spawn-paste-buffer-per-call

## Why

Host supervisor, 2026-11-08: `conductor spawn --repository <repo>` on one instance printed
`refused: LaunchFailed: the systemd provider's start tmux -S <session>.tmux paste-buffer -d -p -b
conductor -t =<session>: exited 1: no buffer conductor`, while the session had started, received
its launch prompt and worked from its hand-off; the controller stayed recorded running. The
spawn reported a failure for a launch that succeeded, and the LaunchFailed line woke the
supervisor's watcher.

`type_text` in `crates/conductor/src/confine/systemd.rs` loads every paste into one tmux buffer
named `conductor` and pastes it with `-d`, which deletes it. Inferred, not reproduced: a second
`type_text` into the same session's tmux server between this call's `load-buffer` and
`paste-buffer` (or a repeated paste step) consumed the buffer first.

## Acceptance

1. Each `type_text` call loads and pastes a buffer of its own name (`conductor-<pid>-<nanos>`), so
   two calls into one server never consume each other's buffer. A test runs two concurrent calls
   against one tmux server and both texts arrive.
2. A paste step that fails after the launch prompt reached the pane does not report
   `LaunchFailed` for a session that is live and recorded running. Test.

## Out of Scope

Changing how the launch prompt is composed.

## Ambiguities

Whether the second paste came from a concurrent delivery or a repeated step is not known; the
per-call buffer covers both.
