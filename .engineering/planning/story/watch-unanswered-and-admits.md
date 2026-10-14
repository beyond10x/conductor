---
format: aep.planning-md/3
id: story:watch-unanswered-and-admits
kind: story
status: draft
title: The watch names an unanswered report and disk coming back
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

Conductor's history review (2026-10-06 to 14): 23 controller reports and NEEDs waited more than 30
minutes for conductor's next event, 53 h in all, at most 6 h. A blocked controller also sat at 27G
free while `/` rose to 34G, released only by a message.

## Acceptance

- The watch prints `report unanswered: <repo> <dispatch> <m>m` once when a received report or NEED
  has had no conductor event on its dispatch for 30 minutes, and again each further 30; a test
  holds it.
- The watch prints `disk admits <n>G` once when free disk rises back over a level a refusal or a
  `blocked` report named; a test holds it.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.
