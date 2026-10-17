---
format: aep.planning-md/3
id: story:session-attach-watch
kind: story
status: draft
title: session attach --watch shows a live session read-only
relations:
- decomposes: epic:session-attach
- depends_on: story:session-attach-command
revision: 2
---
# Story: session-attach-watch

## Outcome

`conductor session attach --watch` shows a live session and its output without any way to type into it, so the operator can look at a controller without risking an interrupted turn, and several people can watch at once.

## Context

- The read-only mode is a property of the connection that the relay enforces, not of the client: a modified client must not be able to type (the relay of `story:session-pty-relay`, replacing the launcher at crates/conductor/src/confine/systemd.rs:72-74).
- Several watchers are harmless; one read-write client at a time is the rule of `story:session-attach-command`.
- `--watch` is the `watch` field of `conductor.session.AttachSession` (modelled by `story:session-attach-command`, spec/domains/session.yaml beside :38).
- The relay's `read-write` and `watch` modes and the refusal frame are modelled by `story:session-pty-relay` in `spec/domains/session.yaml`; this story adds none.

## Acceptance

1. `session attach --watch --role conductor` prints the recent output, then live output; keystrokes typed in that terminal reach nothing (the stand-in harness's input stays empty).
2. A raw connection to the socket that announces `watch` and then sends input frames has them dropped and is closed with a refusal frame; the harness's input is unchanged.
3. Three `--watch` clients and one read-write client run at once; the read-write client's input reaches the harness and all four see its output.
4. `--watch` honours the detach key and exits 0, as the read-write mode does.
5. A watch client of a session whose relay has exited flushes the last output, prints that the session ended and exits 0.

## Out of Scope

Sharing a watch over a network; recording.

## Ambiguities

None.
