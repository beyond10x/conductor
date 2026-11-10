---
format: aep.planning-md/3
id: story:build-run-waiting-ends-on-signal
kind: story
status: draft
title: A build run that waits in the queue ends on a signal
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
revision: 4
---
## Why

Seen while wave 49 ran (2026-11-10), not reproduced in a test: a `conductor build run` that waited in the host queue (waiting for disk, position 1) did not end on SIGINT, on SIGTERM or on a second SIGINT within about 25 s, and was ended with SIGKILL. The case `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger` covers a neighbouring path and fails at times under load.

## Acceptance

- A run that waits in the queue for any reason (slot, disk, memory, target lock) ends within 2 s of SIGINT or SIGTERM, withdraws its request from the ledger and exits with the signal's status; a test holds each wait reason.
