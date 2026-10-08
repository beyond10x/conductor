---
format: aep.planning-md/3
id: story:watch-silent-controller
kind: story
status: draft
title: The watch reports a controller that waits on nothing
tags:
- session-review
revision: 1
---
## Why

Found by a review of an instance's sessions after its records moved out of the product checkout (two independent reviewers). A controller waited about 60 minutes on a sub-agent whose last tool call never returned; it
told conductor the sub-agent was "still running", and nothing noticed. The session list showed the
controller as `waiting`.

## Acceptance

- The watch prints `session waiting: <name> <id> <minutes>m` once when a controller has been
  `waiting` past a threshold (a config key, spec first) while a sub-agent transcript of it has
  been silent as long.
- A test with a fake session list and sub-agent transcripts.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
