---
format: aep.planning-md/3
id: story:watch-silent-controller
kind: story
status: active
title: The watch reports a live controller whose transcripts stay silent
tags:
- session-review
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T02:22:17Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-13T02:22:17Z", actor: "human:timo", revision: 6}
---
## Why

Found by a review of an instance's sessions after its records moved out of the product checkout (two
independent reviewers). A controller waited about 60 minutes on a sub-agent whose last tool call
never returned; it told conductor the sub-agent was "still running", and nothing noticed. The
session list showed the controller as `waiting`.

The operator asked on 2026-10-13 why conductor does not keep an eye on a working controller such as
the ess session. Conductor is woken only by the watch's events (an exit, a usage limit, context,
disk, CI), by a controller's messages, and by its 2-hourly cycle; a controller that is live but
makes no progress produces none of them.

## Acceptance

- The watch prints `session silent: <name> <id> <minutes>m` once when a live controller session
  (its state `working` or `waiting`) has written nothing to its transcript, and none of its
  sub-agents to theirs, for longer than `thresholds.silent_session` (a config key, spec first,
  default 30 minutes), and again only after it has written something and fallen silent anew; a
  test with a fake session list and transcripts whose modification times are set.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model/`, `crates/conductor/src/config.rs`,
`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
