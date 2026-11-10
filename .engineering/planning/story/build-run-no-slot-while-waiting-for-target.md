---
format: aep.planning-md/3
id: story:build-run-no-slot-while-waiting-for-target
kind: story
status: draft
title: A build run holds no slot while it waits for its repository's target directory
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
revision: 4
---
## Why

Seen while wave 49 ran (2026-11-10): `conductor build run` takes one of the host's build slots before it holds its repository's target directory lock. Several runs of one repository then hold several slots while one of them builds, and runs of other repositories wait with the message that all slots are taken.

## Acceptance

- A run that waits for its repository's target directory holds no build slot; a test starts two runs of one repository on a host of one slot beside a run of a second repository and sees the second repository build while the first one's second run waits.
- The waiting line still names the reason (another run of this repository holds its target directory).
