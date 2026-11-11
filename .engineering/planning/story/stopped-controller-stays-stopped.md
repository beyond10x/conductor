---
format: aep.planning-md/3
id: story:stopped-controller-stays-stopped
kind: story
status: draft
title: A stopped controller stays stopped, and the cycle starts none for a repository with no work
tags:
- from-trial
scope:
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
revision: 2
---
## Why

Reported by the supervising session of a trial instance on 2026-11-10 from conductor 1.1.0 (its daemon still on 1.0.1). A controller reported a hand-over and ended; the loop recorded a new controller Running three times within an hour, each a fresh session with no task, while the repository's only dispatch was Done.

Dispatch v1 (crates/conductor/src/run/dispatch.rs, `dispatch_v1`) starts a controller each cycle tick for every Active repository that has a charter and no Running or Paused controller. A Stopped controller does not count, so `controller stop-controller` holds until the next cycle only, and a repository with nothing to do gets a session with only its charter. No guide says how a controller is retired.

## Acceptance

- A controller stopped with `controller stop-controller` is not started again by the cycle tick until an operator act or a new dispatch for its repository says so; a test holds two cycle ticks after a stop.
- The cycle tick starts no controller for a repository that has no open dispatch and no goal it would be sent one for; a test holds it.
- website/docs/guides/start-conductor.md says how a controller is retired and which of stop, pause, a removed charter and `repository deactivate-repository` keeps it stopped.
