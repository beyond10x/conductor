---
format: aep.planning-md/3
id: story:lease-guard
kind: story
status: draft
title: The guard denies writes from a conductor session that does not hold the lease
relations:
- decomposes: epic:operating-loop-hardening
- depends_on: story:single-conductor-lease
- depends_on: story:guard-store-migrate
scope:
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
revision: 2
---
## Why

A conductor lease (story:single-conductor-lease) refuses a second claim, but a session that
ignores the refusal can still write. Every conductor-role tool call already reaches `conductor
guard` through the role's `PreToolUse` hook (`.claude/conductor-settings.json`), so the guard is
where the lease is held: while a live holder exists, a write from another conductor-role session
is denied. Split out of story:single-conductor-lease after the plan critics' round 1.

## Design

The guard reads the lease from the instance's state directory. For a conductor-role session whose
session id (the hook payload's `session_id`, `crates/conductor/src/guard.rs:229`) is not the live
holder's, it denies every write (a records write, SendMessage, a Bash write form) and names the
holder's session id and pid. Reads stay allowed. With no lease, or a holder whose pid is gone, the
guard decides as it does today.

## Acceptance

- The guard denies a records write and a SendMessage from a conductor-role session that is not the
  live lease holder, naming the holder's session id and pid; tests in
  `crates/conductor/tests/guard_hook.rs`.

## Notes

- Ordered after story:guard-store-migrate, which changes the guard's rules and their tests in
  the same module.

## Scope

`crates/conductor/src/guard.rs`, `crates/conductor/tests/guard_hook.rs`.
