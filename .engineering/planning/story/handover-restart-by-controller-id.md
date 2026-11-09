---
format: aep.planning-md/3
id: story:handover-restart-by-controller-id
kind: story
status: active
title: A pending hand-over restart follows the controller it was asked of, never a younger session of the same name
summary: The loop stopped a 5-min-old session restarted by hand under the same name
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: inferred
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/systemd.rs
- confidence: inferred
  path: crates/conductor/tests/deliver.rs
- confidence: inferred
  path: crates/conductor/tests/session_attach.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T09:58:06Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T09:58:07Z", actor: "human:timo", revision: 3}
---
# Story: handover-restart-by-controller-id

## Why

`handovers()` in `crates/conductor/src/run/handlers.rs` keys a pending hand-over by session
name and forgets it only when the repository's running controller has another session name. A
session restarted by hand under the same name keeps the entry alive, and the loop stops the
younger session.

Host teacher's evidence, relayed by the supervisor, 2026-11-08 (local times): a controller session
wrote its hand-over at 05:04 and its sub-agents ran to about 10:35; at 10:36 a restart by hand
stopped it and started a new session under the same name; at 10:41:14 the loop logged
`session gone` for the old one, at 10:41:40 it stopped the new one (5 min old), and at 10:42:10
it spawned a third. The same race on a second instance, 10:36:52 to 10:36:58: the loop stopped a
session and started another in the minute of the supervisor's own `conductor spawn`, which then
printed `LaunchFailed: … paste-buffer … no buffer conductor`; the two launches pasted into one
tmux server through the one buffer name `conductor` (`type_text`,
`crates/conductor/src/confine/systemd.rs`), and one consumed the other's buffer.

## Acceptance

1. A pending hand-over entry holds the controller id (and session id) it was asked of; it is
   dropped once the repository's running controller record has another id, whatever the session
   name. A test: an entry for controller A, the repository's controller replaced by B under the
   same session name; the tick stops nothing and forgets the entry.
2. The loop never stops a session started after the entry's delivery. Test.
3. Each `type_text` call loads and pastes a buffer of its own name, so two launches into one
   tmux server never consume each other's buffer. Test with two concurrent calls.

## Out of Scope

The supervisor's hand-restart procedure (its profile).

## Ambiguities

None.
