---
format: aep.planning-md/3
id: story:merge-wakes-idle-controller
kind: story
status: draft
title: A controller idle on its own merge request is told when it merges
summary: When a merge request a controller opened is merged, an idle controller of that repository gets one message, so its next step (tag, release, next wave) does not wait for a person
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
scope:
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: inferred
  path: crates/conductor/src/watch.rs
revision: 1
---
## Why

Agentbox friction (2026-11-16): a backend release merge request merged while its controller sat
idle, and the controller was not woken. The operator tagged the release by hand. A
`report-unblocked` did not help, because the dispatch was already `Started`. The next cycle did
not wake it either, because a cycle starts controllers and does not message live ones.

## Outcome

When the watch sees that a merge request a live controller opened has merged, it sends that
controller one message naming the merge request and its merge commit. The controller then takes
its next step without a person.

## Acceptance

- A merge request opened by a controller that is live and idle is merged. Within one watch tick
  the controller gets exactly one message naming the merge request and the merge commit. A run
  test with a fake forge holds it.
