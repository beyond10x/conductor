---
format: aep.planning-md/3
id: story:status-subscription
kind: story
status: draft
title: Sessions follow state changes instead of polling status
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

A session polled conductor's full status every 3-5 minutes for over an hour (`docs/design/self-improvement.md`, F10, I7).

## Acceptance

- `conductor status --follow` prints one line per change (dispatch state, a merged or released PR, a red CI run, an operator item, a disk shortfall) as the watch sees it, and exits on a closed pipe; the gateway and other sessions use it instead of repeated status reads; tests over a fixture watch.

## Scope

`crates/conductor/src/status.rs`, `crates/conductor/src/watch.rs`, tests, `.agents/conductor.md`.
