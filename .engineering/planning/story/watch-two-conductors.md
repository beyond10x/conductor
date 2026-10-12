---
format: aep.planning-md/3
id: story:watch-two-conductors
kind: story
status: implemented
title: conductor watch reports two live conductor sessions as an incident
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T17:53:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T17:53:56Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T18:06:13Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

A session resumed by hand without the instance's settings runs no `SessionStart` hook and no
guard, so neither the lease claim (story:single-conductor-lease) nor the guard (story:lease-guard)
sees it. On 2026-10-12 exactly such a session ran as a second `conductor` beside the managed one.
`conductor watch` already lists the live sessions; it should say when two of them carry the
conductor role's session name, so conductor-dev and the operator hear of it. Split out of
story:single-conductor-lease after the plan critics' round 1.

## Design

`conductor watch` (`crates/conductor/src/watch.rs`) reads the live sessions from `claude agents
--json`. When two or more live sessions carry the conductor role's session name (with the
instance's `session_prefix`, if any), it prints one incident line naming each session id.

## Acceptance

- `conductor watch` reports two live sessions named as the conductor role as an incident line; a
  test in `crates/conductor/tests/watch.rs` over a fixed `claude agents --json` listing holds it.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
