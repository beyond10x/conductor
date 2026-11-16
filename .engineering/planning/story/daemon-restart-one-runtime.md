---
format: aep.planning-md/3
id: story:daemon-restart-one-runtime
kind: story
status: draft
title: A stopped and restarted conductord leaves exactly one runtime per boxed instance
summary: What stopping conductord does to a boxed runtime is decided and documented, and a restart never runs a second runtime beside the first
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:refused-stop-keeps-runtime
scope:
- confidence: cited
  path: crates/conductor/src/daemon/mod.rs
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: inferred
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: website/docs/concepts/the-host.md
revision: 1
---
## Why

Agentbox friction log F6 (2026-11-15, conductor 1.10/1.11, placement `systemd`): stopping
`conductord.service` left the boxed runtime running (pid 584166 alive after the stop), and the
next start had two runtimes dialing in to the same instance. Read at v1.13.0:

- The daemon stops its runtimes on a signal only when they are its children
  (`end_children_on_signal`, `crates/conductor/src/daemon/mod.rs:498-532`); a box opened through
  substrate is not, so it outlives the daemon.
- A restarted daemon adopts a box it finds in `boxes.json` when substrate says its session runs
  (`Daemon::look`, `crates/conductor/src/daemon/reconcile.rs:1312-1328`), and mints a new token
  only when it starts a runtime (`reconcile.rs:1102-1104`). When adoption fails (no entry, or
  substrate does not answer), a second box is opened while the first keeps dialing with its old
  token; the runtime log then repeats `the daemon closed the connection unanswered ... not with
  this token` (F20).
- No document says whether a box outliving the daemon is intended.

## Outcome

- The docs say what stopping conductord does to a boxed runtime (it keeps running and is
  adopted, or it is closed), and the code does that.
- Before it opens a box for an instance, the daemon closes every box substrate still runs for
  that instance that it does not adopt, so one instance never has two runtimes.

## Acceptance

- With a fake substrate that still runs an instance's box whose `boxes.json` entry is gone, a
  daemon start opens a new box for the instance only after closing the old one, and afterwards
  substrate lists exactly one session for the instance (`tests/placement.rs` or a daemon test).
